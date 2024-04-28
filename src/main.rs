use clap::{command, value_parser, Arg};
use libc::{
    recvfrom, sockaddr, sockaddr_ll, socket, socklen_t, AF_PACKET, ETH_P_PPP_DISC, SOCK_DGRAM,
};
use pretty_hex::{HexConfig, PrettyHex};
use std::ffi::c_int;
use std::fmt::{Display, Formatter};
use std::io::Error;
use std::mem::{size_of_val, zeroed};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::process::ExitCode;

fn main() -> ExitCode {
    // Parse arguments.
    let args = command!()
        .arg(
            Arg::new("interface")
                .help("Index of the interface that connected with the PS4")
                .value_name("IF")
                .value_parser(value_parser!(c_int))
                .required(true),
        )
        .get_matches();

    // Create a socket for PPPoE discovery.
    let disc = unsafe { socket(AF_PACKET, SOCK_DGRAM, 0) };

    if disc < 0 {
        eprintln!(
            "Failed to create PPPoE discovery socket: {}.",
            Error::last_os_error()
        );

        return ExitCode::FAILURE;
    }

    // Bind socket to target interface.
    let disc = unsafe { OwnedFd::from_raw_fd(disc) };
    let mut addr: sockaddr_ll = unsafe { zeroed() };

    addr.sll_family = AF_PACKET as _;
    addr.sll_protocol = (ETH_P_PPP_DISC as u16).to_be();
    addr.sll_ifindex = *args.get_one("interface").unwrap();

    if let Err(e) = bind_ll(disc.as_fd(), &addr) {
        eprintln!("Failed to bind PPPoE discovery socket: {e}.",);
        return ExitCode::FAILURE;
    }

    'top: loop {
        // Wait for PPPoE discovery packet.
        let mut buf = [0; 1500];
        let (len, addr) = match recv_ll(disc.as_fd(), &mut buf) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to receive a packet from PPPoE discovery socket: {e}.");
                return ExitCode::FAILURE;
            }
        };

        dump_received(&addr, &buf[..len]);

        // Check if PPPoE Active Discovery Initiation (PADI) packet.
        let padi = match Payload::deserialize(&buf[..len]) {
            Some(v) if v.code == 0x09 => v,
            _ => {
                eprintln!("Unexpected packet from the PS4!");
                continue;
            }
        };

        if padi.session_id != 0x0000 {
            eprintln!("Unexpected PPPoE SESSION_ID from the PS4!");
            continue;
        }

        // Check Service-Name tag.
        let mut sn = None; // Service-Name
        let mut hu = None; // Host-Uniq

        for &(t, v) in &padi.tags {
            match t {
                0x0101 => {
                    if sn.is_some() {
                        eprintln!("Multiple Service-Name tags on PADI packet from the PS4!");
                        continue 'top;
                    }

                    match std::str::from_utf8(v) {
                        Ok(v) => sn = Some(v),
                        Err(_) => {
                            eprintln!("Invalid Service-Name tag on PADI packet from the PS4!");
                            continue 'top;
                        }
                    }
                }
                0x0103 => hu = Some(v),
                _ => {}
            }
        }

        let sn = match sn {
            Some(v) => v,
            None => {
                eprintln!("No Service-Name tag on PADI packet from the PS4!");
                continue;
            }
        };

        println!("PADI: Service-Name = '{sn}', Host-Uniq = {hu:?}");
    }
}

fn bind_ll(fd: BorrowedFd, addr: &sockaddr_ll) -> Result<(), Error> {
    let fd = fd.as_raw_fd();
    let len = size_of_val(addr).try_into().unwrap();
    let addr = addr as *const sockaddr_ll as *const sockaddr;

    if unsafe { libc::bind(fd, addr, len) < 0 } {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

fn recv_ll(fd: BorrowedFd, buf: &mut [u8; 1500]) -> Result<(usize, sockaddr_ll), Error> {
    let mut addr: sockaddr_ll = unsafe { zeroed() };
    let mut alen: socklen_t = size_of_val(&addr).try_into().unwrap();
    let received = unsafe {
        recvfrom(
            fd.as_raw_fd(),
            buf.as_mut_ptr().cast(),
            buf.len(),
            0,
            &mut addr as *mut sockaddr_ll as _,
            &mut alen,
        )
    };

    if received < 0 {
        return Err(Error::last_os_error());
    }

    assert_eq!(alen, size_of_val(&addr).try_into().unwrap());

    Ok((received.try_into().unwrap(), addr))
}

fn dump_received(addr: &sockaddr_ll, data: &[u8]) {
    // Print header.
    print!("R: ");

    for i in 0..addr.sll_halen {
        let i: usize = i.into();

        if i != 0 {
            print!(":");
        }

        print!("{:x}", addr.sll_addr[i]);
    }

    println!(
        " (Type = {}, Length = {})",
        PacketType::new(addr.sll_pkttype),
        data.len()
    );

    // Print data.
    let mut conf = HexConfig::default();

    conf.title = false;

    println!("{:?}", data.hex_conf(conf));
}

enum PacketType {
    Broadcast,
}

impl PacketType {
    fn new(raw: u8) -> Self {
        match raw {
            1 => Self::Broadcast,
            _ => panic!("unknown sll_pkttype {raw}"),
        }
    }
}

impl Display for PacketType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Broadcast => f.write_str("broadcast"),
        }
    }
}

/// Ethernet payload for PPPoE packet.
struct Payload<'a> {
    code: u8,
    session_id: u16,
    tags: Vec<(u16, &'a [u8])>,
}

impl<'a> Payload<'a> {
    fn deserialize(data: &'a [u8]) -> Option<Self> {
        // Check minimum Ethernet payload length.
        if data.len() < 6 {
            return None;
        }

        // Check version and type.
        let ver = data[0] & 0xf;
        let ty = data[0] >> 4;

        if ver != 1 || ty != 1 {
            return None;
        }

        // Read CODE, SESSION_ID, LENGTH and payload.
        let code = data[1];
        let session_id = u16::from_be_bytes(data[2..4].try_into().unwrap());
        let length: usize = u16::from_be_bytes(data[4..6].try_into().unwrap()).into();
        let mut payload = data[6..].get(..length)?;

        // Read tags.
        let mut tags = Vec::new();

        while !payload.is_empty() {
            if payload.len() < 4 {
                return None;
            }

            let ty = u16::from_be_bytes(payload[..2].try_into().unwrap());
            let length: usize = u16::from_be_bytes(payload[2..4].try_into().unwrap()).into();
            let value = payload[4..].get(..length)?;

            tags.push((ty, value));
            payload = &payload[(4 + length)..];
        }

        Some(Self {
            code,
            session_id,
            tags,
        })
    }
}
