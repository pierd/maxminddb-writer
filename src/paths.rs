use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use thiserror::Error;

fn trailing_zeros(s: &[u8]) -> usize {
    let mut count = 0;
    for &b in s.iter().rev() {
        if b == 0 {
            count += 8;
        } else {
            count += b.trailing_zeros();
            break;
        }
    }
    count as usize
}

fn octets_with_mask<const N: usize>(mut start: [u8; N], mut count: usize) -> Vec<([u8; N], u8)> {
    let mut result = Vec::new();
    while count > 0 {
        // calculate the biggest possible mask
        let zeros = trailing_zeros(&start);
        let suffix = zeros.min(count.ilog2() as usize);
        let mask = N * 8 - suffix;
        result.push((start, mask as u8));

        // increment start
        let mut byte_to_change = N - suffix / 8 - 1;
        let mut bit_to_change = suffix % 8;
        loop {
            if let Some(new_val) = start[byte_to_change].checked_add(1 << bit_to_change) {
                start[byte_to_change] = new_val;
                break;
            } else {
                start[byte_to_change] = 0;
                if byte_to_change == 0 {
                    break;
                }
                byte_to_change -= 1;
                bit_to_change = 0;
            }
        }

        // subtract the block from count
        count -= 1 << suffix;
    }
    result
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpAddrWithMask {
    pub addr: IpAddr,
    pub mask: u8,
}

impl IpAddrWithMask {
    pub fn new(addr: IpAddr, mask: u8) -> Self {
        Self { addr, mask }
    }

    pub fn from_count(addr: IpAddr, count: usize) -> Vec<Self> {
        match addr {
            IpAddr::V4(addr) => octets_with_mask(addr.octets(), count)
                .into_iter()
                .map(|(octets, mask)| {
                    let addr = Ipv4Addr::from(octets);
                    Self::new(IpAddr::V4(addr), mask)
                })
                .collect(),
            IpAddr::V6(addr) => octets_with_mask(addr.octets(), count)
                .into_iter()
                .map(|(octets, mask)| {
                    let addr = Ipv6Addr::from(octets);
                    Self::new(IpAddr::V6(addr), mask)
                })
                .collect(),
        }
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

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_trailing_zeros() {
        assert_eq!(trailing_zeros(&[0, 0, 0, 0]), 32);
        assert_eq!(trailing_zeros(&[0, 0, 0, 1]), 0);
        assert_eq!(trailing_zeros(&[0, 0, 1, 0]), 8);
        assert_eq!(trailing_zeros(&[0, 1, 0, 0]), 16);
        assert_eq!(trailing_zeros(&[1, 0, 0, 0]), 24);
        assert_eq!(trailing_zeros(&[1, 0, 0, 1]), 0);
    }

    #[test]
    fn test_octets_with_mask() {
        assert_eq!(
            octets_with_mask([1, 0, 0, 0], 255),
            vec![
                ([1, 0, 0, 0], 25),
                ([1, 0, 0, 128], 26),
                ([1, 0, 0, 192], 27),
                ([1, 0, 0, 224], 28),
                ([1, 0, 0, 240], 29),
                ([1, 0, 0, 248], 30),
                ([1, 0, 0, 252], 31),
                ([1, 0, 0, 254], 32),
            ],
        );
        assert_eq!(
            octets_with_mask([1, 0, 0, 240], 32),
            vec![([1, 0, 0, 240], 28), ([1, 0, 1, 0], 28),],
        );
        assert_eq!(
            octets_with_mask([196, 11, 105, 0], 256),
            vec![([196, 11, 105, 0], 24),],
        );
        assert_eq!(
            octets_with_mask([196, 11, 105, 0], 1024),
            vec![
                ([196, 11, 105, 0], 24),
                ([196, 11, 106, 0], 23),
                ([196, 11, 108, 0], 24),
            ],
        );
    }

    #[test]
    fn test_ip_addr_with_mask() {
        let addr = "196.11.105.0".parse();
        let count = 1024;
        let addrs = IpAddrWithMask::from_count(addr.unwrap(), count);
        assert_eq!(
            addrs,
            vec![
                IpAddrWithMask {
                    addr: IpAddr::V4(Ipv4Addr::new(196, 11, 105, 0)),
                    mask: 24,
                },
                IpAddrWithMask {
                    addr: IpAddr::V4(Ipv4Addr::new(196, 11, 106, 0)),
                    mask: 23,
                },
                IpAddrWithMask {
                    addr: IpAddr::V4(Ipv4Addr::new(196, 11, 108, 0)),
                    mask: 24,
                },
            ]
        );
    }
}
