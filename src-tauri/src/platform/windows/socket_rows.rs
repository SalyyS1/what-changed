//! Conversions from the extended socket tables' layout into product facts.
//!
//! Split out from the API calls so the byte-order rules are unit-testable
//! against a fixed pattern. The classic bug in these tables is reading a
//! network-order port as a host integer, which turns 4000 into a different
//! perfectly plausible port rather than an obviously broken one — so the rule is
//! pinned here by a test that names the number.

use std::net::{Ipv4Addr, Ipv6Addr};

use windows::Win32::NetworkManagement::IpHelper::{
    MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, MIB_UDP6ROW_OWNER_PID, MIB_UDPROW_OWNER_PID,
};

use crate::domain::ids::Protocol;
use crate::platform::traits::SocketRow;

/// A local port as the extended tables report it: the network-order value in
/// the low sixteen bits of a `u32`, with the high bits unused.
pub fn port_from_table(raw: u32) -> u16 {
    let network_order = u16::try_from(raw & 0xFFFF).unwrap_or(0);
    u16::from_be(network_order)
}

/// A dotted quad from the IPv4 tables.
///
/// `dwLocalAddr` is the raw `IN_ADDR`: four octets in network order inside a
/// native `u32`. Reading it back in memory order is therefore the octet order,
/// whatever the host's endianness happens to be.
pub fn ipv4_from_table(raw: u32) -> String {
    Ipv4Addr::from(raw.to_ne_bytes()).to_string()
}

/// A compressed IPv6 from the sixteen network-order octets the IPv6 tables
/// carry inline, with the interface scope appended when the address has one.
///
/// Without the scope, two link-local listeners on different interfaces collide
/// into one entity, and the next diff reports a phantom owner change on a
/// socket nobody touched. `netstat` shows the scope, so this matches what a
/// person comparing the two would see.
pub fn ipv6_from_table(octets: [u8; 16], scope_id: u32) -> String {
    let address = Ipv6Addr::from(octets).to_string();
    if scope_id == 0 {
        address
    } else {
        format!("{address}%{scope_id}")
    }
}

/// One TCP/IPv4 listener.
pub fn tcp_v4(row: &MIB_TCPROW_OWNER_PID) -> SocketRow {
    SocketRow {
        protocol: Protocol::Tcp,
        address: ipv4_from_table(row.dwLocalAddr),
        port: port_from_table(row.dwLocalPort),
        owning_pid: row.dwOwningPid,
    }
}

/// One TCP/IPv6 listener.
pub fn tcp_v6(row: &MIB_TCP6ROW_OWNER_PID) -> SocketRow {
    SocketRow {
        protocol: Protocol::Tcp,
        address: ipv6_from_table(row.ucLocalAddr, row.dwLocalScopeId),
        port: port_from_table(row.dwLocalPort),
        owning_pid: row.dwOwningPid,
    }
}

/// One UDP/IPv4 endpoint.
///
/// UDP has no listen state, so every bound endpoint is reported: an unbound
/// UDP port is indistinguishable from a closed one from the outside, and
/// pretending otherwise would hide the thing a person is looking for.
pub fn udp_v4(row: &MIB_UDPROW_OWNER_PID) -> SocketRow {
    SocketRow {
        protocol: Protocol::Udp,
        address: ipv4_from_table(row.dwLocalAddr),
        port: port_from_table(row.dwLocalPort),
        owning_pid: row.dwOwningPid,
    }
}

/// One UDP/IPv6 endpoint.
pub fn udp_v6(row: &MIB_UDP6ROW_OWNER_PID) -> SocketRow {
    SocketRow {
        protocol: Protocol::Udp,
        address: ipv6_from_table(row.ucLocalAddr, row.dwLocalScopeId),
        port: port_from_table(row.dwLocalPort),
        owning_pid: row.dwOwningPid,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn port_four_thousand_survives_the_byte_order_conversion() {
        // The API puts 4000's two network-order octets, 0x0F then 0xA0, in the
        // low half of the field. Read as a host integer that is 40975, which is
        // a perfectly plausible port and therefore a silent bug.
        let raw = u32::from_ne_bytes([0x0F, 0xA0, 0x00, 0x00]);
        assert_eq!(port_from_table(raw), 4_000);
        assert_ne!(
            u16::try_from(raw & 0xFFFF).unwrap(),
            4_000,
            "the naive reading is wrong, which is why the conversion exists"
        );

        assert_eq!(
            port_from_table(u32::from_ne_bytes([0x1F, 0x90, 0, 0])),
            8_080
        );
        assert_eq!(port_from_table(0), 0);
        assert_eq!(
            port_from_table(u32::from_ne_bytes([0xFF, 0xFF, 0, 0])),
            u16::MAX
        );
    }

    #[test]
    fn an_ipv4_address_reads_in_octet_order() {
        let loopback = u32::from_ne_bytes([0x7F, 0x00, 0x00, 0x01]);
        assert_eq!(ipv4_from_table(loopback), "127.0.0.1");
        assert_eq!(ipv4_from_table(0), "0.0.0.0");
        let routed = u32::from_ne_bytes([192, 168, 1, 42]);
        assert_eq!(ipv4_from_table(routed), "192.168.1.42");
    }

    #[test]
    fn an_ipv6_address_compresses_and_keeps_its_scope() {
        let mut loopback = [0u8; 16];
        loopback[15] = 1;
        assert_eq!(ipv6_from_table(loopback, 0), "::1");
        assert_eq!(ipv6_from_table([0u8; 16], 0), "::");

        let mut link_local = [0u8; 16];
        link_local[0] = 0xFE;
        link_local[1] = 0x80;
        link_local[15] = 0x01;
        assert_eq!(
            ipv6_from_table(link_local, 12),
            "fe80::1%12",
            "two interfaces must not share one listener entity"
        );
    }

    #[test]
    fn a_row_becomes_the_product_fact_the_diff_keys_on() {
        let tcp = MIB_TCPROW_OWNER_PID {
            dwState: 2,
            dwLocalAddr: u32::from_ne_bytes([0, 0, 0, 0]),
            dwLocalPort: u32::from_ne_bytes([0x0F, 0xA0, 0, 0]),
            dwRemoteAddr: 0,
            dwRemotePort: 0,
            dwOwningPid: 4_242,
        };
        assert_eq!(
            tcp_v4(&tcp),
            SocketRow {
                protocol: Protocol::Tcp,
                address: "0.0.0.0".to_owned(),
                port: 4_000,
                owning_pid: 4_242,
            }
        );

        let udp = MIB_UDPROW_OWNER_PID {
            dwLocalAddr: u32::from_ne_bytes([0x7F, 0, 0, 0x01]),
            dwLocalPort: u32::from_ne_bytes([0x00, 0x35, 0, 0]),
            dwOwningPid: 7,
        };
        assert_eq!(
            udp_v4(&udp),
            SocketRow {
                protocol: Protocol::Udp,
                address: "127.0.0.1".to_owned(),
                port: 53,
                owning_pid: 7,
            }
        );
    }

    #[test]
    fn the_ipv6_rows_carry_their_address_inline_and_their_scope_beside_it() {
        let mut local = [0u8; 16];
        local[15] = 1;
        let tcp = MIB_TCP6ROW_OWNER_PID {
            ucLocalAddr: local,
            dwLocalScopeId: 0,
            dwLocalPort: u32::from_ne_bytes([0x0B, 0xB8, 0, 0]),
            ucRemoteAddr: [0u8; 16],
            dwRemoteScopeId: 0,
            dwRemotePort: 0,
            dwState: 2,
            dwOwningPid: 99,
        };
        assert_eq!(
            tcp_v6(&tcp),
            SocketRow {
                protocol: Protocol::Tcp,
                address: "::1".to_owned(),
                port: 3_000,
                owning_pid: 99,
            }
        );

        let udp = MIB_UDP6ROW_OWNER_PID {
            ucLocalAddr: local,
            dwLocalScopeId: 5,
            dwLocalPort: u32::from_ne_bytes([0x0B, 0xB8, 0, 0]),
            dwOwningPid: 99,
        };
        assert_eq!(udp_v6(&udp).address, "::1%5");
        assert_eq!(udp_v6(&udp).protocol, Protocol::Udp);
    }
}
