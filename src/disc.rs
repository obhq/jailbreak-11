use crate::addr::AddrBuilder;
use crate::sock::PacketSocket;
use erdp::ErrorDisplay;
use libc::ETH_P_PPP_DISC;
use macaddr::MacAddr6;
use std::borrow::Cow;
use std::io::Write;
use std::sync::Arc;

/// Server for PPPoE Discovery Stage.
pub struct DiscoveryServer {
    sock: PacketSocket,
    ab: Arc<AddrBuilder>,
}

impl DiscoveryServer {
    pub fn new(sock: PacketSocket, ab: Arc<AddrBuilder>) -> Self {
        Self { sock, ab }
    }

    pub async fn run(&self) -> bool {
        let mut buf = [0; 1500];

        loop {
            // Wait for PPPoE discovery packet.
            let (len, addr) = match self.sock.recv(&mut buf).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "Failed to receive a packet from PPPoE discovery socket: {}.",
                        e.display()
                    );

                    return false;
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
                0 => match data.code {
                    0x19 => self.parse_padr(addr, data),
                    _ => eprintln!(
                        "Unexpected PPPoE discovery unicast packet {} from {}.",
                        data.code, addr
                    ),
                },
                1 => match data.code {
                    0x09 => self.parse_padi(addr, data),
                    _ => eprintln!(
                        "Unexpected PPPoE discovery broadcast packet {} from {}.",
                        data.code, addr
                    ),
                },
                _ => eprintln!("Unexpected sll_pkttype from {addr}."),
            }
        }
    }

    fn parse_padi(&self, addr: MacAddr6, data: Payload) {
        if data.session_id != 0x0000 {
            eprintln!("Unexpected PPPoE SESSION_ID from {addr}.");
            return;
        }

        // Process tags.
        let mut sn = None; // Service-Name
        let mut hu = None; // Host-Uniq

        for (t, v) in &data.tags {
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
        let mut pado = Payload {
            code: 0x07,
            session_id: 0x0000,
            tags: vec![
                (0x0102, Cow::Borrowed("OBHQ Jailbreak 11.00".as_bytes())),
                (0x0101, Cow::Borrowed(sn.as_bytes())),
            ],
        };

        if let Some(hu) = hu {
            pado.tags.push((0x0103, Cow::Borrowed(hu)));
        }

        if let Err(e) = self.sock.send(
            self.ab.build(ETH_P_PPP_DISC as _, Some(addr)),
            pado.serialize(),
        ) {
            eprintln!("Failed to send PADO packet to {}: {}.", addr, e.display());
        }
    }

    fn parse_padr(&self, addr: MacAddr6, data: Payload) {
        if data.session_id != 0x0000 {
            eprintln!("Unexpected PPPoE SESSION_ID from {addr}.");
            return;
        }

        // Process tags.
        let mut sn = None; // Service-Name
        let mut hu = None; // Host-Uniq

        for (t, v) in &data.tags {
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

        // Send PPPoE Active Discovery Session-confirmation (PADS) packet.
        let session_id = 0;
        let mut pads = Payload {
            code: 0x65,
            session_id,
            tags: vec![(0x0101, Cow::Borrowed(sn.as_bytes()))],
        };

        if let Some(hu) = hu {
            pads.tags.push((0x0103, Cow::Borrowed(hu)));
        }

        if let Err(e) = self.sock.send(
            self.ab.build(ETH_P_PPP_DISC as _, Some(addr)),
            pads.serialize(),
        ) {
            eprintln!("Failed to send PADS packet to {}: {}.", addr, e.display());
        }
    }
}

/// Ethernet payload for PPPoE packet.
struct Payload<'a> {
    code: u8,
    session_id: u16,
    tags: Vec<(u16, Cow<'a, [u8]>)>,
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

            tags.push((ty, Cow::Borrowed(value)));
            payload = &payload[(4 + length)..];
        }

        Some(Self {
            code,
            session_id,
            tags,
        })
    }

    fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Write VER, TYPE and CODE.
        buf.push(0x11);
        buf.push(self.code);

        // Write SESSION_ID and a placeholder for LENGTH.
        buf.write_all(&self.session_id.to_be_bytes()).unwrap();
        buf.write_all(&[0; 2]).unwrap();

        // Write tags.
        let mut len = 0usize;

        for (t, v) in &self.tags {
            let l: u16 = v.len().try_into().unwrap();

            buf.write_all(&t.to_be_bytes()).unwrap();
            buf.write_all(&l.to_be_bytes()).unwrap();
            buf.write_all(v).unwrap();

            len += 4 + Into::<usize>::into(l);
        }

        assert!(len <= (1500 - 6));

        // Write LENGTH.
        buf[4..6].copy_from_slice(&(len as u16).to_be_bytes());
        buf
    }
}
