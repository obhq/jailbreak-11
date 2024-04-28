use libc::{sockaddr_ll, AF_PACKET};
use macaddr::MacAddr6;
use std::ffi::c_int;
use std::mem::zeroed;

/// Struct to build a [`sockaddr_ll`].
pub struct AddrBuilder {
    interface: c_int,
}

impl AddrBuilder {
    pub fn new(interface: c_int) -> Self {
        Self { interface }
    }

    pub fn build(&self, proto: u16, addr: Option<MacAddr6>) -> sockaddr_ll {
        let mut v: sockaddr_ll = unsafe { zeroed() };

        v.sll_family = AF_PACKET as _;
        v.sll_protocol = proto.to_be();
        v.sll_ifindex = self.interface;

        if let Some(addr) = addr {
            let addr = addr.as_bytes();

            v.sll_addr[..addr.len()].copy_from_slice(addr);
            v.sll_halen = addr.len().try_into().unwrap();
        }

        v
    }
}
