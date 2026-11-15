// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::IpAddr;
use std::str::FromStr;

use axum::http::HeaderMap;
use serde::{Deserialize, Deserializer};

/// An address range in CIDR notation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Network {
    address: IpAddr,
    prefix: u8,
}

impl Network {
    #[must_use]
    pub fn contains(&self, candidate: IpAddr) -> bool {
        let candidate = canonical(candidate);
        match (self.address, candidate) {
            (IpAddr::V4(network), IpAddr::V4(candidate)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(network) & mask == u32::from(candidate) & mask
            }
            (IpAddr::V6(network), IpAddr::V6(candidate)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(network) & mask == u128::from(candidate) & mask
            }
            _ => false,
        }
    }
}

/// Treats IPv4-mapped IPv6 addresses as the IPv4 addresses they carry, so
/// a dual-stack listener matches IPv4 ranges.
fn canonical(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(address, IpAddr::V4),
        IpAddr::V4(_) => address,
    }
}

impl FromStr for Network {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let (address, prefix) = text.split_once('/').unwrap_or((text, ""));
        let address: IpAddr = address
            .parse()
            .map_err(|_| format!("invalid network address in {text:?}"))?;
        let address = canonical(address);
        let maximum = if address.is_ipv4() { 32 } else { 128 };
        let prefix = if prefix.is_empty() {
            maximum
        } else {
            prefix
                .parse::<u8>()
                .ok()
                .filter(|prefix| *prefix <= maximum)
                .ok_or_else(|| format!("invalid prefix length in {text:?}"))?
        };
        Ok(Self { address, prefix })
    }
}

impl<'de> Deserialize<'de> for Network {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// The client's address: the peer, or the right-most address in
/// `X-Forwarded-For` that is not a trusted proxy when the peer is one.
#[must_use]
pub fn client_address(peer: IpAddr, headers: &HeaderMap, trusted: &[Network]) -> IpAddr {
    let peer = canonical(peer);
    if !trusted.iter().any(|network| network.contains(peer)) {
        return peer;
    }
    let forwarded = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|part| part.trim().parse::<IpAddr>().ok())
        .map(canonical);
    forwarded
        .rev()
        .find(|address| !trusted.iter().any(|network| network.contains(*address)))
        .unwrap_or(peer)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn net(text: &str) -> Network {
        text.parse().unwrap()
    }

    #[test]
    fn networks_match_their_ranges() {
        assert!(net("10.0.0.0/8").contains("10.200.1.1".parse().unwrap()));
        assert!(!net("10.0.0.0/8").contains("11.0.0.1".parse().unwrap()));
        assert!(net("127.0.0.1").contains("127.0.0.1".parse().unwrap()));
        assert!(!net("127.0.0.1").contains("127.0.0.2".parse().unwrap()));
        assert!(net("0.0.0.0/0").contains("8.8.8.8".parse().unwrap()));
        assert!(net("::1/128").contains("::1".parse().unwrap()));
        assert!(net("fd00::/8").contains("fd12::1".parse().unwrap()));
        assert!(!net("fd00::/8").contains("10.0.0.1".parse().unwrap()));
        assert!(net("127.0.0.0/8").contains("::ffff:127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn malformed_networks_are_rejected() {
        assert!("10.0.0.0/33".parse::<Network>().is_err());
        assert!("::/129".parse::<Network>().is_err());
        assert!("nonsense/8".parse::<Network>().is_err());
        assert!("10.0.0.0/x".parse::<Network>().is_err());
    }

    #[test]
    fn forwarded_addresses_count_only_behind_trusted_proxies() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.9, 10.0.0.5"),
        );
        let trusted = [net("10.0.0.0/8")];
        let proxy: IpAddr = "10.0.0.1".parse().unwrap();
        assert_eq!(
            client_address(proxy, &headers, &trusted),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
        let direct: IpAddr = "198.51.100.7".parse().unwrap();
        assert_eq!(client_address(direct, &headers, &trusted), direct);
        assert_eq!(client_address(proxy, &HeaderMap::new(), &trusted), proxy);
    }
}
