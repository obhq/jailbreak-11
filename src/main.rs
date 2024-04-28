use clap::{command, value_parser, Arg};
use libc::{sockaddr, sockaddr_ll, socket, AF_PACKET, ETH_P_PPP_DISC, SOCK_DGRAM};
use std::ffi::c_int;
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
    addr.sll_protocol = ETH_P_PPP_DISC.to_be() as _;
    addr.sll_ifindex = *args.get_one("interface").unwrap();

    if let Err(e) = bind_ll(disc.as_fd(), &addr) {
        eprintln!("Failed to bind PPPoE discovery socket: {e}.",);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
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
