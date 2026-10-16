use std::net::SocketAddr;

use ssh_config::AddressFamily;
use tokio::net::lookup_host;

use crate::error::TransportError;

pub(crate) async fn resolve_addresses(
    host: &str,
    port: u16,
    family: AddressFamily,
) -> Result<Vec<SocketAddr>, TransportError> {
    let resolved = lookup_host((host, port))
        .await
        .map_err(|source| TransportError::Resolve {
            host: host.to_owned(),
            port,
            source,
        })?;
    let addresses = filter_by_family(resolved, family);
    if addresses.is_empty() {
        return Err(TransportError::NoAddress {
            host: host.to_owned(),
            port,
        });
    }
    Ok(addresses)
}

fn filter_by_family(
    addresses: impl IntoIterator<Item = SocketAddr>,
    family: AddressFamily,
) -> Vec<SocketAddr> {
    addresses
        .into_iter()
        .filter(|address| match family {
            AddressFamily::Any => true,
            AddressFamily::Inet => address.is_ipv4(),
            AddressFamily::Inet6 => address.is_ipv6(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addresses() -> Vec<SocketAddr> {
        vec!["127.0.0.1:22".parse().unwrap(), "[::1]:22".parse().unwrap()]
    }

    #[test]
    fn should_keep_every_address_when_family_is_any() {
        assert_eq!(
            filter_by_family(addresses(), AddressFamily::Any),
            addresses()
        );
    }

    #[test]
    fn should_keep_only_ipv4_addresses_when_family_is_inet() {
        assert_eq!(
            filter_by_family(addresses(), AddressFamily::Inet),
            vec!["127.0.0.1:22".parse::<SocketAddr>().unwrap()]
        );
    }

    #[test]
    fn should_keep_only_ipv6_addresses_when_family_is_inet6() {
        assert_eq!(
            filter_by_family(addresses(), AddressFamily::Inet6),
            vec!["[::1]:22".parse::<SocketAddr>().unwrap()]
        );
    }

    #[tokio::test]
    async fn should_fail_with_no_address_when_no_address_has_the_requested_family() {
        let outcome = resolve_addresses("127.0.0.1", 22, AddressFamily::Inet6).await;
        assert!(matches!(outcome, Err(TransportError::NoAddress { .. })));
    }

    #[tokio::test]
    async fn should_resolve_a_literal_address_when_host_is_an_ip() {
        let outcome = resolve_addresses("127.0.0.1", 2222, AddressFamily::Any).await;
        assert_eq!(
            outcome.unwrap(),
            vec!["127.0.0.1:2222".parse::<SocketAddr>().unwrap()]
        );
    }
}
