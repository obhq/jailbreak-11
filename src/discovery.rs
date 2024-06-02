use crate::addr::AddrBuilder;
use crate::payload::EthernetPayload;
use crate::session::Sessions;
use crate::socket::PacketSocket;
use erdp::ErrorDisplay;
use libc::ETH_P_PPP_DISC;
use macaddr::MacAddr6;
use std::borrow::Cow;
use std::io::Write;
use std::sync::Arc;
use tokio::select;
use tokio_util::sync::CancellationToken;

/// Server for PPPoE Discovery Stage.
pub struct DiscoveryServer {
    sock: PacketSocket,
    ab: Arc<AddrBuilder>,
    sessions: Arc<Sessions>,
}

impl DiscoveryServer {
    pub fn new(sock: PacketSocket, ab: Arc<AddrBuilder>, sessions: Arc<Sessions>) -> Self {
        Self { sock, ab, sessions }
    }

    pub async fn run(self, running: CancellationToken) {
        let mut buf = [0; 1500];

        loop {
            // Wait for PPPoE discovery packet.
            let (len, addr) = select! {
                _ = running.cancelled() => break,
                v = self.sock.recv(&mut buf) => match v {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!(
                            "Failed to receive a packet from PPPoE discovery socket: {}.",
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

            // Deserialize the payload.
            let data = match Payload::deserialize(&buf[..len]) {
                Some(v) => v,
                None => {
                    eprintln!("Unexpected PPPoE discovery packet from {addr}.");
                    continue;
                }
            };

            // Process the payload.
            match ty {
                0 => match data.code() {
                    0x19 => self.parse_padr(addr, data),
                    _ => eprintln!(
                        "Unexpected PPPoE discovery unicast packet {} from {}.",
                        data.code(),
                        addr
                    ),
                },
                1 => match data.code() {
                    0x09 => self.parse_padi(addr, data),
                    _ => eprintln!(
                        "Unexpected PPPoE discovery broadcast packet {} from {}.",
                        data.code(),
                        addr
                    ),
                },
                _ => eprintln!("Unexpected sll_pkttype for PPPoE discovery packet from {addr}."),
            }
        }
    }

    fn parse_padi(&self, addr: MacAddr6, data: Payload) {
        if data.session_id() != 0x0000 {
            eprintln!("Unexpected PPPoE SESSION_ID from {addr}.");
            return;
        }

        // Process tags.
        let mut sn = None; // Service-Name
        let mut hu = None; // Host-Uniq

        for (t, v) in data.payload() {
            match t {
                0x0101 => {
                    if sn.is_some() {
                        eprintln!("Multiple Service-Name tags on PADI packet from {addr}.");
                        return;
                    }

                    match std::str::from_utf8(v.as_ref()) {
                        Ok(v) => sn = Some(v),
                        Err(_) => {
                            eprintln!("Invalid Service-Name tag on PADI packet from {addr}.");
                            return;
                        }
                    }
                }
                0x0103 => hu = Some(v.as_ref()),
                _ => {}
            }
        }

        // Check Service-Name tag.
        let sn = match sn {
            Some(v) => v,
            None => {
                eprintln!("No Service-Name tag on PADI packet from {addr}.");
                return;
            }
        };

        println!("PADI: Service-Name = '{sn}', Host-Uniq = {hu:?}");

        // Send PPPoE Active Discovery Offer (PADO) packet.
        let mut pado = Payload::new(
            0x07,
            0x0000,
            vec![
                (0x0102, Cow::Borrowed("OBHQ Jailbreak 11.00".as_bytes())),
                (0x0101, Cow::Borrowed(sn.as_bytes())),
            ],
        );

        if let Some(hu) = hu {
            pado.payload_mut().push((0x0103, Cow::Borrowed(hu)));
        }

        if let Err(e) = self.sock.send(
            self.ab.build(ETH_P_PPP_DISC as _, Some(addr)),
            pado.serialize(),
        ) {
            eprintln!("Failed to send PADO packet to {}: {}.", addr, e.display());
        }
    }

    fn parse_padr(&self, addr: MacAddr6, data: Payload) {
        if data.session_id() != 0x0000 {
            eprintln!("Unexpected PPPoE SESSION_ID from {addr}.");
            return;
        }

        // Process tags.
        let mut sn = None; // Service-Name
        let mut hu = None; // Host-Uniq

        for (t, v) in data.payload() {
            match t {
                0x0101 => {
                    if sn.is_some() {
                        eprintln!("Multiple Service-Name tags on PADR packet from {addr}.");
                        return;
                    }

                    match std::str::from_utf8(v.as_ref()) {
                        Ok(v) => sn = Some(v),
                        Err(_) => {
                            eprintln!("Invalid Service-Name tag on PADR packet from {addr}.");
                            return;
                        }
                    }
                }
                0x0103 => hu = Some(v.as_ref()),
                _ => {}
            }
        }

        // Check Service-Name tag.
        let sn = match sn {
            Some(v) => v,
            None => {
                eprintln!("No Service-Name tag on PADR packet from {addr}.");
                return;
            }
        };

        println!("PADR: Service-Name = '{sn}', Host-Uniq = {hu:?}");

        // Spawn a session.
        let session = match self.sessions.spawn() {
            Some(v) => v,
            None => todo!(),
        };

        // Send PPPoE Active Discovery Session-confirmation (PADS) packet.
        let mut pads = Payload::new(
            0x65,
            session.id().get(),
            vec![(0x0101, Cow::Borrowed(sn.as_bytes()))],
        );

        if let Some(hu) = hu {
            pads.payload_mut().push((0x0103, Cow::Borrowed(hu)));
        }

        if let Err(e) = self.sock.send(
            self.ab.build(ETH_P_PPP_DISC as _, Some(addr)),
            pads.serialize(),
        ) {
            eprintln!("Failed to send PADS packet to {}: {}.", addr, e.display());
            return;
        }

        // Spawn a task to handle the session.
        tokio::spawn(session.run());
    }
}

impl<'a> crate::payload::Payload<'a> for Vec<(u16, Cow<'a, [u8]>)> {
    fn deserialize(mut data: &'a [u8]) -> Option<Self> {
        let mut tags = Vec::new();

        while !data.is_empty() {
            if data.len() < 4 {
                return None;
            }

            let ty = u16::from_be_bytes(data[..2].try_into().unwrap());
            let length: usize = u16::from_be_bytes(data[2..4].try_into().unwrap()).into();
            let value = data[4..].get(..length)?;

            tags.push((ty, Cow::Borrowed(value)));
            data = &data[(4 + length)..];
        }

        Some(tags)
    }

    fn serialize(&self, buf: &mut Vec<u8>) {
        for (t, v) in self {
            let l: u16 = v.len().try_into().unwrap();

            buf.write_all(&t.to_be_bytes()).unwrap();
            buf.write_all(&l.to_be_bytes()).unwrap();
            buf.write_all(v).unwrap();
        }
    }
}

type Payload<'a> = EthernetPayload<Vec<(u16, Cow<'a, [u8]>)>>;
