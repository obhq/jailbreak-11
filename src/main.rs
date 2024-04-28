use crate::addr::AddrBuilder;
use crate::disc::DiscoveryServer;
use crate::sock::PacketSocket;
use clap::{command, value_parser, Arg};
use libc::ETH_P_PPP_DISC;
use std::ffi::c_int;
use std::process::ExitCode;
use std::sync::Arc;

mod addr;
mod disc;
mod sock;

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
    let disc = match PacketSocket::new() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to create PPPoE discovery socket: {e}.");
            return ExitCode::FAILURE;
        }
    };

    // Bind socket to target interface.
    let ab = Arc::new(AddrBuilder::new(*args.get_one("interface").unwrap()));
    let addr = ab.build(ETH_P_PPP_DISC as _, None);

    if let Err(e) = disc.bind(&addr) {
        eprintln!("Failed to bind PPPoE discovery socket: {e}.");
        return ExitCode::FAILURE;
    }

    // Run discovery server.
    let disc = DiscoveryServer::new(disc, ab.clone());

    if disc.run() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
