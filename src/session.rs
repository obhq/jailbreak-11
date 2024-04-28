use crate::payload::EthernetPayload;
use crate::socket::PacketSocket;
use erdp::ErrorDisplay;
use macaddr::MacAddr6;
use std::borrow::Cow;
use tokio::select;
use tokio_util::sync::CancellationToken;

/// Server for PPPoE Session Stage.
pub struct SessionServer {
    sock: PacketSocket,
}

impl SessionServer {
    pub fn new(sock: PacketSocket) -> Self {
        Self { sock }
    }

    pub async fn run(self, running: CancellationToken) {
        let mut buf = [0; 1500];

        loop {
            // Wait for PPPoE session packet.
            let (len, addr) = select! {
                _ = running.cancelled() => break,
                v = self.sock.recv(&mut buf) => match v {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!(
                            "Failed to receive a packet from PPPoE session socket: {}.",
                            e.display()
                        );

                        running.cancel();
                        return;
                    }
                }
            };

            // Get source address.
            let ty = addr.sll_pkttype;
            let addr = match addr.sll_halen {
                6 => MacAddr6::from(TryInto::<[u8; 6]>::try_into(&addr.sll_addr[..6]).unwrap()),
                _ => unreachable!(),
            };

            if ty != 0 {
                eprintln!("Unexpected sll_pkttype for PPPoE session packet from {addr}.");
                continue;
            }

            // Deserialize the payload.
            let data = match Payload::deserialize(&buf[..len]) {
                Some(v) => v,
                None => {
                    eprintln!("Unexpected PPPoE session packet from {addr}.");
                    continue;
                }
            };

            if data.code() != 0x00 {
                eprintln!(
                    "Unexpected PPPoE session packet {} from {}.",
                    data.code(),
                    addr
                );

                continue;
            }
        }
    }
}

type Payload<'a> = EthernetPayload<Cow<'a, [u8]>>;
