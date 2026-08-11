use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use if_addrs::get_if_addrs;
use qrcode::{QrCode, render::unicode};

/// Builds the LAN pairing URL while keeping the one-time token in the fragment.
#[must_use]
pub fn pairing_url(bound_address: SocketAddr, token: &str) -> String {
    let address = access_address(bound_address, discover_lan_ip().ok());
    format!("http://{address}/#pair={token}")
}

pub(crate) fn access_address(bound_address: SocketAddr, discovered: Option<IpAddr>) -> SocketAddr {
    let ip = if bound_address.ip().is_unspecified() {
        discovered.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
    } else {
        bound_address.ip()
    };
    SocketAddr::new(ip, bound_address.port())
}

/// Renders a value as a compact Unicode QR code for terminal onboarding.
///
/// # Errors
///
/// Returns a QR encoding error when the value cannot fit in a QR symbol.
pub fn terminal_qr(value: &str) -> Result<String, qrcode::types::QrError> {
    Ok(QrCode::new(value.as_bytes())?
        .render::<unicode::Dense1x2>()
        .quiet_zone(true)
        .build())
}

fn discover_lan_ip() -> io::Result<IpAddr> {
    let candidates = get_if_addrs()?
        .into_iter()
        .filter_map(|interface| {
            let IpAddr::V4(ip) = interface.ip() else {
                return None;
            };
            let is_up = interface.is_oper_up();
            Some(LanCandidate {
                ip,
                interface_name: interface.name,
                is_up,
            })
        })
        .collect::<Vec<_>>();

    select_lan_ipv4(&candidates).map(IpAddr::V4).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "no usable LAN IPv4 interface was found",
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LanCandidate {
    ip: Ipv4Addr,
    interface_name: String,
    is_up: bool,
}

fn select_lan_ipv4(candidates: &[LanCandidate]) -> Option<Ipv4Addr> {
    candidates
        .iter()
        .filter(|candidate| candidate.is_up && is_usable_lan_ipv4(candidate.ip))
        .max_by_key(|candidate| lan_candidate_score(candidate))
        .map(|candidate| candidate.ip)
}

fn lan_candidate_score(candidate: &LanCandidate) -> (u8, u8, [u8; 4]) {
    let private = u8::from(candidate.ip.is_private());
    let physical = u8::from(!looks_virtual(&candidate.interface_name));
    // Reverse the octets so `max_by_key` remains deterministic without relying on
    // the operating system's interface enumeration order.
    let mut address_order = candidate.ip.octets();
    for octet in &mut address_order {
        *octet = u8::MAX - *octet;
    }
    (private, physical, address_order)
}

fn looks_virtual(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "docker",
        "hyper-v",
        "tailscale",
        "tap",
        "tun",
        "veth",
        "virtualbox",
        "vmware",
        "vpn",
        "wsl",
        "zerotier",
    ]
    .iter()
    .any(|marker| name.contains(marker))
}

fn is_usable_lan_ipv4(ip: Ipv4Addr) -> bool {
    let [first, second, third, _] = ip.octets();
    let documentation = matches!(
        (first, second, third),
        (192, 0, 2) | (198, 51, 100) | (203, 0, 113)
    );
    let benchmarking = first == 198 && matches!(second, 18 | 19);

    !ip.is_unspecified()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_multicast()
        && ip != Ipv4Addr::BROADCAST
        && !documentation
        && !benchmarking
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_address_replaces_only_unspecified_bind_addresses()
    -> Result<(), Box<dyn std::error::Error>> {
        let discovered = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42));
        assert_eq!(
            access_address("0.0.0.0:20778".parse()?, Some(discovered)),
            "192.168.1.42:20778".parse()?
        );
        assert_eq!(
            access_address("127.0.0.1:1234".parse()?, Some(discovered)),
            "127.0.0.1:1234".parse()?
        );
        Ok(())
    }

    #[test]
    fn pairing_url_keeps_the_secret_in_the_fragment_and_qr_encodes_it()
    -> Result<(), Box<dyn std::error::Error>> {
        let url = pairing_url("127.0.0.1:20778".parse()?, "url-safe_token");
        assert_eq!(url, "http://127.0.0.1:20778/#pair=url-safe_token");
        assert!(terminal_qr(&url).is_ok());
        Ok(())
    }

    #[test]
    fn lan_selection_prefers_a_physical_private_interface() {
        let candidates = [
            candidate("VPN adapter", [198, 18, 0, 1], true),
            candidate("Ethernet", [169, 254, 10, 20], true),
            candidate("Tailscale", [10, 0, 0, 9], true),
            candidate("Wi-Fi", [192, 168, 1, 42], true),
            candidate("Disconnected", [192, 168, 1, 2], false),
        ];

        assert_eq!(
            select_lan_ipv4(&candidates),
            Some(Ipv4Addr::new(192, 168, 1, 42))
        );
    }

    #[test]
    fn lan_selection_has_a_deterministic_non_private_fallback() {
        let candidates = [
            candidate("Ethernet B", [8, 8, 8, 8], true),
            candidate("Ethernet A", [1, 1, 1, 1], true),
        ];

        assert_eq!(
            select_lan_ipv4(&candidates),
            Some(Ipv4Addr::new(1, 1, 1, 1))
        );
    }

    fn candidate(name: &str, octets: [u8; 4], is_up: bool) -> LanCandidate {
        LanCandidate {
            ip: Ipv4Addr::from(octets),
            interface_name: name.to_owned(),
            is_up,
        }
    }
}
