use libc::{socket, AF_PACKET, ETH_P_PPP_DISC, SOCK_RAW};
use std::io::Error;
use std::os::fd::{FromRawFd, OwnedFd};
use std::process::ExitCode;

fn main() -> ExitCode {
    // Create a socket for PPPoE discovery.
    let disc = unsafe { socket(AF_PACKET, SOCK_RAW, ETH_P_PPP_DISC.to_be()) };

    if disc < 0 {
        eprintln!(
            "Failed to create PPPoE discovery socket: {}.",
            Error::last_os_error()
        );

        return ExitCode::FAILURE;
    }

    // Bind socket to a specific interface.
    let disc = unsafe { OwnedFd::from_raw_fd(disc) };

    ExitCode::SUCCESS
}
