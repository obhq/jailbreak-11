use libc::{recvfrom, sendto, sockaddr, sockaddr_ll, socket, socklen_t, AF_PACKET, SOCK_DGRAM};
use pretty_hex::{hex_write, HexConfig};
use std::fmt::Write;
use std::io::Error;
use std::mem::{size_of_val, zeroed};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

/// Encapsulate an `AF_PACKET` socket.
pub struct PacketSocket(OwnedFd);

impl PacketSocket {
    pub fn new() -> Result<Self, Error> {
        let disc = unsafe { socket(AF_PACKET, SOCK_DGRAM, 0) };

        if disc < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(Self(unsafe { OwnedFd::from_raw_fd(disc) }))
        }
    }

    pub fn bind(&self, addr: &sockaddr_ll) -> Result<(), Error> {
        let fd = self.0.as_raw_fd();
        let len = size_of_val(addr).try_into().unwrap();
        let addr = addr as *const sockaddr_ll as *const sockaddr;

        if unsafe { libc::bind(fd, addr, len) < 0 } {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn recv(&self, buf: &mut [u8]) -> Result<(usize, sockaddr_ll), Error> {
        // Receive.
        let mut addr: sockaddr_ll = unsafe { zeroed() };
        let mut alen: socklen_t = size_of_val(&addr).try_into().unwrap();
        let received = unsafe {
            recvfrom(
                self.0.as_raw_fd(),
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

        // Print header.
        let received = received as usize;
        let mut log = String::from("R: ");

        Self::write_addr(&mut log, &addr);

        writeln!(log, " (Type = {}, Length = {})", addr.sll_pkttype, received).unwrap();

        // Print data.
        let mut conf = HexConfig::default();

        conf.title = false;

        hex_write(&mut log, &buf[..received], conf).unwrap();

        println!("{log}");

        Ok((received, addr))
    }

    pub fn send(&self, addr: sockaddr_ll, buf: impl AsRef<[u8]>) -> Result<(), Error> {
        // Send.
        let buf = buf.as_ref();
        let sent = unsafe {
            sendto(
                self.0.as_raw_fd(),
                buf.as_ptr().cast(),
                buf.len(),
                0,
                &addr as *const sockaddr_ll as _,
                size_of_val(&addr).try_into().unwrap(),
            )
        };

        if sent < 0 {
            return Err(Error::last_os_error());
        }

        assert_eq!(sent as usize, buf.len());

        // Print header.
        let mut log = String::from("S: ");

        Self::write_addr(&mut log, &addr);

        writeln!(log, " (Length = {})", sent).unwrap();

        // Print sent data.
        let mut conf = HexConfig::default();

        conf.title = false;

        hex_write(&mut log, buf, conf).unwrap();

        println!("{log}");

        Ok(())
    }

    fn write_addr(w: &mut impl Write, addr: &sockaddr_ll) {
        for i in 0..addr.sll_halen {
            let i: usize = i.into();

            if i != 0 {
                write!(w, ":").unwrap();
            }

            write!(w, "{:x}", addr.sll_addr[i]).unwrap();
        }
    }
}
