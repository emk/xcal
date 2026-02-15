//! Fantasy port names, extracted from C's `port.c`.
//!
//! Each connection gets a whimsical location name shown to regular users
//! (e.g. "Avalon", "Mordor"). The name is deterministically derived from
//! the client's IP address. Reusable across transports.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    net::IpAddr,
};

/// Fantasy location names assigned to connections.
const FANTASY_NAMES: &[&str] = &[
    "ThePits", "Asylum", "Mordor", "Heaven", "Vortex", "Nowhere", "Fantasia",
    "Paradise", "Eden", "Avalon", "Scotland", "Reality", "Mirkwood", "Bangkok",
    "Void", "LaBrea", "Madwand", "London", "Lorien", "Hades", "Arabia",
    "Orthanc", "Zodiac", "StarsEnd", "Scheme", "Syntax", "Istanbul", "BadLand",
    "Fantasy", "Mercury", "Sidhe", "Fairy", "Luna", "Korea", "Eriador",
    "Tartarus", "Moscow", "Nunnery", "Krystle", "Paradise", "Deep End",
    "Vergil", "Inferno", "Eriador", "Castle", "Ithilien", "Boston", "Tower",
    "Sylvanus", "Draconi", "Polaris", "World", "Aurora", "Jungle",
];

/// Pick a fantasy name for the given IP address.
///
/// Hashes the full address (v4 or v6) for good distribution across the
/// name table. The same address always produces the same name.
#[allow(clippy::expect_used)]
pub fn fantasy_name(addr: IpAddr) -> &'static str {
    let mut hasher = DefaultHasher::new();
    addr.hash(&mut hasher);
    let table_len =
        u64::try_from(FANTASY_NAMES.len()).expect("table len fits u64");
    let idx = usize::try_from(
        hasher
            .finish()
            .checked_rem(table_len)
            .expect("non-zero table"),
    )
    .expect("index < table len, fits usize");
    FANTASY_NAMES[idx]
}

/// Returns a regex pattern matching any fantasy port name.
///
/// Useful for normalizing location names in test assertions, similar to
/// how fixture `normalize.toml` files handle timestamps.
#[cfg(any(test, feature = "test-support"))]
pub fn fantasy_name_regex() -> &'static str {
    use std::sync::OnceLock;
    static PATTERN: OnceLock<String> = OnceLock::new();
    PATTERN.get_or_init(|| {
        // Deduplicate names (FANTASY_NAMES has duplicates like "Paradise", "Eriador").
        let mut unique: Vec<&str> = FANTASY_NAMES.to_vec();
        unique.sort_unstable();
        unique.dedup();
        unique.join("|")
    })
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    #[test]
    fn localhost_gets_a_name() {
        let name = fantasy_name(IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert!(FANTASY_NAMES.contains(&name));
    }

    #[test]
    fn same_addr_same_name() {
        let addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42));
        assert_eq!(fantasy_name(addr), fantasy_name(addr));
    }

    #[test]
    fn ipv6_mapped_uses_v4() {
        // v4 and its v6-mapped form may or may not hash the same — that's
        // fine, we don't need backwards compatibility with the C mapping.
        let v6 = Ipv4Addr::new(10, 0, 0, 1).to_ipv6_mapped();
        let name = fantasy_name(IpAddr::V6(v6));
        assert!(FANTASY_NAMES.contains(&name));
    }

    #[test]
    fn pure_ipv6_gets_a_name() {
        let addr = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert!(FANTASY_NAMES.contains(&fantasy_name(addr)));
    }
}
