//! General-profile outbound TCP: resolve once, classify all, pin the dial.
//!
//! This is not a complete browser policy proxy. It cannot admit Preview,
//! launch Chromium, or prove UDP enforcement. The later HTTP/CONNECT proxy
//! must use it for every connection and have no direct fallback.

mod interfaces;

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use tokio::net::TcpStream;
use vessel_proto::browser::BrowserUrl;

pub use interfaces::InterfaceError;
use interfaces::LocalInterfaces;

/// Bound one resolution independently from the browser's command deadline.
const DIAL_TIMEOUT: Duration = Duration::from_secs(5);
/// A large/malformed resolver answer is refused as a whole, never truncated.
const MAX_DNS_ANSWERS: usize = 32;

/// Network refusals retain system causes but never display the page-selected host/URL.
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    /// An incomplete interface observation cannot authorize public egress.
    #[error("current local interface addresses are unavailable")]
    Interfaces(#[from] InterfaceError),
    /// The URL must still have an HTTP(S) host and effective port.
    #[error("browser destination has no supported host and port")]
    Address,
    /// A failed DNS query is not a permission to use an old cached answer.
    #[error("browser destination resolution failed")]
    Resolve(#[source] std::io::Error),
    /// Neither an empty answer nor a partial answer grants authority.
    #[error("browser destination resolution did not produce a bounded complete answer")]
    Answers,
    /// One forbidden answer makes a mixed public/private name forbidden.
    #[error("browser destination policy refused the resolved address")]
    Refused,
    /// Pinned connection failures never retry by resolving the name elsewhere.
    #[error("browser destination connection failed")]
    Connect(#[source] std::io::Error),
    /// Resolver, observation and all attempted dials share a single deadline.
    #[error("browser destination connection timed out")]
    TimedOut(#[source] tokio::time::error::Elapsed),
}

/// Open one General-profile TCP connection to checked public addresses only.
///
/// # Errors
/// Missing/current interface evidence, incomplete/mixed resolver answers, timeout
/// and connection failures all refuse. Non-Linux interface observation is unsupported.
pub async fn dial_public(destination: &BrowserUrl) -> Result<TcpStream, NetworkError> {
    tokio::time::timeout(DIAL_TIMEOUT, dial(destination))
        .await
        .map_err(NetworkError::TimedOut)?
}

async fn dial(destination: &BrowserUrl) -> Result<TcpStream, NetworkError> {
    let url = destination.parsed();
    let host = url.host_str().ok_or(NetworkError::Address)?;
    // URL's IPv6 host spelling includes brackets; the socket resolver takes
    // a host without them. BrowserUrl already refused aliases and userinfo.
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let port = url.port_or_known_default().ok_or(NetworkError::Address)?;
    let resolved = tokio::net::lookup_host((host, port))
        .await
        .map_err(NetworkError::Resolve)?;
    let mut answers = Vec::new();
    for answer in resolved {
        if answers.len() >= MAX_DNS_ANSWERS {
            return Err(NetworkError::Answers);
        }
        answers.push(answer);
    }
    let local = LocalInterfaces::current().await?;
    check(&answers, &local)?;
    connect_checked(&answers).await
}

async fn connect_checked(answers: &[SocketAddr]) -> Result<TcpStream, NetworkError> {
    let mut last = None;
    for address in answers {
        // Interface assignment may change after DNS or an earlier failed dial.
        let local = LocalInterfaces::current().await?;
        check(answers, &local)?;
        match TcpStream::connect(address).await {
            Ok(stream) => {
                let current = LocalInterfaces::current().await?;
                check(answers, &current)?;
                let peer = stream.peer_addr().map_err(NetworkError::Connect)?;
                if peer != *address {
                    return Err(NetworkError::Refused);
                }
                return Ok(stream);
            }
            Err(error) => last = Some(error),
        }
    }
    match last {
        Some(error) => Err(NetworkError::Connect(error)),
        None => Err(NetworkError::Answers),
    }
}

fn check(answers: &[SocketAddr], local: &LocalInterfaces) -> Result<(), NetworkError> {
    if answers.is_empty() || answers.len() > MAX_DNS_ANSWERS {
        return Err(NetworkError::Answers);
    }
    if answers.iter().any(|address| {
        !vessel_relay::reach::is_public(address.ip()) || local.contains(address.ip())
    }) {
        return Err(NetworkError::Refused);
    }
    Ok(())
}

/// Local-interface comparisons include public IPv4 embedded in IPv6 spellings.
fn canonical(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V4(_) => address,
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4() {
                return IpAddr::V4(v4);
            }
            let [a, b, c, d, e, f, high, low] = v6.segments();
            if a == 0x0064 && b == 0xff9b && c == 0 && d == 0 && e == 0 && f == 0 {
                let [h1, h2] = high.to_be_bytes();
                let [l1, l2] = low.to_be_bytes();
                IpAddr::V4(std::net::Ipv4Addr::new(h1, h2, l1, l2))
            } else {
                address
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mixed_answer_never_dials_its_public_subset() {
        let local = LocalInterfaces::fixture(["192.168.44.2".parse().unwrap()]);
        let public = "203.0.113.2:443".parse().unwrap();
        assert!(check(&[public], &local).is_ok());
        for forbidden in [
            "127.0.0.1:443",
            "169.254.169.254:80",
            "[::1]:80",
            "10.0.0.2:80",
        ] {
            let answers = [public, forbidden.parse().unwrap()];
            assert!(matches!(
                check(&answers, &local),
                Err(NetworkError::Refused)
            ));
        }
        assert!(matches!(check(&[], &local), Err(NetworkError::Answers)));
        assert!(matches!(
            check(&vec![public; MAX_DNS_ANSWERS.saturating_add(1)], &local),
            Err(NetworkError::Answers)
        ));
    }

    #[test]
    fn own_public_interface_and_its_ipv6_aliases_remain_forbidden() {
        let local = LocalInterfaces::fixture(["203.0.113.2".parse().unwrap()]);
        for own in [
            "203.0.113.2:443",
            "[::ffff:203.0.113.2]:443",
            "[::203.0.113.2]:443",
            "[64:ff9b::203.0.113.2]:443",
        ] {
            assert!(matches!(
                check(&[own.parse().unwrap()], &local),
                Err(NetworkError::Refused)
            ));
        }
    }
}
