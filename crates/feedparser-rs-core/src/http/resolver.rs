//! Custom DNS resolver enforcing SSRF protection on resolved addresses.
//!
//! [`validate_url`](super::validation::validate_url) only inspects the literal
//! host in a URL string. That check alone does not stop DNS rebinding: a
//! domain that resolves to a public IP when the request is validated can be
//! repointed to a private, loopback, or metadata address by the time the
//! connection is actually established. This resolver re-validates every
//! resolved socket address and drops any that are unsafe, closing that gap
//! at connect time rather than at URL-parse time.

use crate::error::FeedError;
use crate::util::ssrf::validate_ip_addr;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::{SocketAddr, ToSocketAddrs};

/// [`Resolve`] implementation that filters DNS results down to publicly
/// routable addresses, rejecting private, loopback, link-local, and cloud
/// metadata IPs even when the requested domain itself passed literal-host
/// validation.
#[derive(Debug, Default)]
pub(super) struct SsrfSafeResolver;

/// Keeps only socket addresses that pass SSRF validation.
///
/// Extracted from [`SsrfSafeResolver::resolve`] so the filtering rule can be
/// unit-tested against a fixed address list without performing real DNS
/// resolution.
fn filter_safe_addrs(addrs: impl Iterator<Item = SocketAddr>) -> Vec<SocketAddr> {
    addrs
        .filter(|addr| validate_ip_addr(addr.ip()).is_ok())
        .collect()
}

impl Resolve for SsrfSafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();

        Box::pin(async move {
            let lookup_host = host.clone();
            let addrs =
                tokio::task::spawn_blocking(move || (lookup_host.as_str(), 0u16).to_socket_addrs())
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let safe_addrs = filter_safe_addrs(addrs);

            if safe_addrs.is_empty() {
                return Err(Box::new(FeedError::Http {
                    message: format!(
                        "DNS resolution for '{host}' returned no public IP addresses \
                         (possible DNS rebinding attempt)"
                    ),
                })
                    as Box<dyn std::error::Error + Send + Sync>);
            }

            Ok(Box::new(safe_addrs.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// Drives a resolver future to completion without pulling in a full
    /// async test harness (`localhost` resolves via the hosts file/loopback
    /// stack, so this stays offline and deterministic).
    fn block_on_resolve(name: &str) -> Result<Addrs, Box<dyn std::error::Error + Send + Sync>> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("failed to build test runtime");
        rt.block_on(SsrfSafeResolver.resolve(Name::from_str(name).expect("valid DNS name")))
    }

    #[test]
    fn test_resolver_rejects_loopback_only_name() {
        // "localhost" resolves to 127.0.0.1/::1 without a network query.
        let result = block_on_resolve("localhost");
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_safe_addrs_drops_metadata_and_loopback() {
        let addrs = [
            "8.8.8.8:0".parse().unwrap(),
            "169.254.169.254:0".parse().unwrap(),
            "127.0.0.1:0".parse().unwrap(),
        ];

        let filtered = filter_safe_addrs(addrs.into_iter());

        assert_eq!(filtered, vec!["8.8.8.8:0".parse().unwrap()]);
    }

    #[test]
    fn test_filter_safe_addrs_empty_when_all_unsafe() {
        let addrs = [
            "169.254.169.254:0".parse().unwrap(),
            "10.0.0.5:0".parse().unwrap(),
        ];

        assert!(filter_safe_addrs(addrs.into_iter()).is_empty());
    }
}
