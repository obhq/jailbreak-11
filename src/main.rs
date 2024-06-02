use crate::addr::AddrBuilder;
use crate::discovery::DiscoveryServer;
use crate::session::{SessionServer, Sessions};
use crate::socket::PacketSocket;
use clap::{command, value_parser, Arg, ArgMatches};
use erdp::ErrorDisplay;
use libc::{ETH_P_PPP_DISC, ETH_P_PPP_SES};
use std::ffi::c_int;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::select;
use tokio_util::sync::CancellationToken;

mod addr;
mod discovery;
mod payload;
mod session;
mod socket;

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

    // Setup Tokio.
    let tokio = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    tokio.block_on(run(args))
}

async fn run(args: ArgMatches) -> ExitCode {
    let ab = Arc::new(AddrBuilder::new(*args.get_one("interface").unwrap()));
    let sessions = Arc::new(Sessions::default());

    // Create a socket for PPPoE discovery.
    let ds = match PacketSocket::new() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to create PPPoE discovery socket: {}.", e.display());
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = ds.bind(ab.build(ETH_P_PPP_DISC as _, None)) {
        eprintln!("Failed to bind PPPoE discovery socket: {}.", e.display());
        return ExitCode::FAILURE;
    }

    // Create a socket for PPPoE session.
    let ss = match PacketSocket::new() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to create PPPoE session socket: {}.", e.display());
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = ss.bind(ab.build(ETH_P_PPP_SES as _, None)) {
        eprintln!("Failed to bind PPPoE session socket: {}.", e.display());
        return ExitCode::FAILURE;
    }

    // Run servers.
    let running = CancellationToken::new();
    let ds = DiscoveryServer::new(ds, ab.clone(), sessions.clone());
    let ss = SessionServer::new(ss);

    tokio::spawn(ds.run(running.clone()));
    tokio::spawn(ss.run(running.clone()));

    // Wait for shutdown.
    select! {
        v = tokio::signal::ctrl_c() => v.unwrap(),
        _ = running.cancelled() => {}
    }

    ExitCode::SUCCESS
}
