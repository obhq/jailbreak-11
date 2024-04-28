use std::borrow::Cow;
use std::io::Write;

/// Ethernet payload for PPPoE packet.
pub struct EthernetPayload<T> {
    code: u8,
    session_id: u16,
    payload: T,
}

impl<T> EthernetPayload<T> {
    pub fn new(code: u8, session_id: u16, payload: T) -> Self {
        Self {
            code,
            session_id,
            payload,
        }
    }

    pub fn deserialize<'a>(data: &'a [u8]) -> Option<Self>
    where
        T: Payload<'a>,
    {
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
        let payload = data[6..].get(..length)?;

        Some(Self {
            code,
            session_id,
            payload: T::deserialize(payload)?,
        })
    }

    pub fn code(&self) -> u8 {
        self.code
    }

    pub fn session_id(&self) -> u16 {
        self.session_id
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn payload_mut(&mut self) -> &mut T {
        &mut self.payload
    }

    pub fn serialize<'a>(&self) -> Vec<u8>
    where
        T: Payload<'a>,
    {
        let mut buf = Vec::new();

        // Write VER, TYPE and CODE.
        buf.push(0x11);
        buf.push(self.code);

        // Write SESSION_ID and payload.
        buf.write_all(&self.session_id.to_be_bytes()).unwrap();
        buf.write_all(&[0; 2]).unwrap();

        self.payload.serialize(&mut buf);

        assert!(buf.len() <= 1500);

        // Write LENGTH.
        let len: u16 = (buf.len() - 6).try_into().unwrap();

        buf[4..6].copy_from_slice(&len.to_be_bytes());
        buf
    }
}

/// Payload of PPPoE packet.
pub trait Payload<'a>: Sized {
    fn deserialize(data: &'a [u8]) -> Option<Self>;
    fn serialize(&self, buf: &mut Vec<u8>);
}

impl<'a> Payload<'a> for Cow<'a, [u8]> {
    fn deserialize(data: &'a [u8]) -> Option<Self> {
        Some(Cow::Borrowed(data))
    }

    fn serialize(&self, buf: &mut Vec<u8>) {
        buf.extend(self.as_ref())
    }
}
