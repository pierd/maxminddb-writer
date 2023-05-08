use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use thiserror::Error;

pub trait IntoBitPath {
    type Output: Iterator<Item = bool>;

    fn into_bit_path(self) -> Self::Output;
}

impl<T> IntoBitPath for T
where
    T: Iterator<Item = bool>,
{
    type Output = T;

    fn into_bit_path(self) -> Self::Output {
        self
    }
}

pub struct IpAddrWithMask {
    addr: IpAddr,
    mask: u8,
}

impl IpAddrWithMask {
    pub fn new(addr: IpAddr, mask: u8) -> Self {
        Self { addr, mask }
    }
}

impl From<IpAddr> for IpAddrWithMask {
    fn from(addr: IpAddr) -> Self {
        match addr {
            IpAddr::V4(addr) => Self::from(addr),
            IpAddr::V6(addr) => Self::from(addr),
        }
    }
}

impl From<Ipv4Addr> for IpAddrWithMask {
    fn from(addr: Ipv4Addr) -> Self {
        Self {
            addr: IpAddr::V4(addr),
            mask: 32,
        }
    }
}

impl From<Ipv6Addr> for IpAddrWithMask {
    fn from(addr: Ipv6Addr) -> Self {
        Self {
            addr: IpAddr::V6(addr),
            mask: 128,
        }
    }
}

#[derive(Debug, Error)]
pub enum IpAddrWithMaskParseError {
    #[error("address parse error")]
    AddrParseError(#[from] std::net::AddrParseError),
    #[error("mask parse error")]
    MaskParseError(#[from] std::num::ParseIntError),
}

impl FromStr for IpAddrWithMask {
    type Err = IpAddrWithMaskParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('/');
        let addr = parts.next().unwrap_or(s);
        let mask = parts.next();
        let addr = IpAddr::from_str(addr)?;
        if let Some(mask) = mask {
            Ok(Self {
                addr,
                mask: mask.parse()?,
            })
        } else {
            Ok(Self::from(addr))
        }
    }
}

impl IntoBitPath for IpAddrWithMask {
    type Output = IpAddrWithMaskBitPath;

    fn into_bit_path(self) -> Self::Output {
        IpAddrWithMaskBitPath { addr: self, bit: 0 }
    }
}

pub struct IpAddrWithMaskBitPath {
    addr: IpAddrWithMask,
    bit: u8,
}

impl Iterator for IpAddrWithMaskBitPath {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bit >= self.addr.mask {
            return None;
        }
        let result = match self.addr.addr {
            IpAddr::V4(addr) => {
                addr.octets()[self.bit as usize / 8] & (1 << (7 - self.bit % 8)) != 0
            }
            IpAddr::V6(addr) => {
                addr.octets()[self.bit as usize / 8] & (1 << (7 - self.bit % 8)) != 0
            }
        };
        self.bit += 1;
        Some(result)
    }
}
