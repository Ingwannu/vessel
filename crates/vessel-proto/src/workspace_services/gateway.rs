//! Gateway settings describe an explicit Admin choice, not active publication.

use std::{fmt, net::SocketAddr, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::workspace::{InvalidServicePort, ServicePort};

use super::{InvalidServiceProxyUrl, ServiceProxyUrl};

/// No DNS lookup or node RPC bind can implicitly select a public listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ServiceListenEndpoint(SocketAddr);

/// Address diagnostics retain typed causes without retaining submitted text.
#[derive(Debug, thiserror::Error)]
pub enum InvalidServiceListenEndpoint {
    /// The standard literal parser does not resolve names.
    #[error("a service listener is a literal socket address")]
    Address(#[source] std::net::AddrParseError),
    /// A configured public listener cannot use an unspecified TCP port.
    #[error("the service listener port is invalid")]
    Port(#[source] InvalidServicePort),
}

impl ServiceListenEndpoint {
    /// Preserve the complete literal endpoint for the eventual listener owner.
    #[must_use]
    pub const fn address(self) -> SocketAddr {
        self.0
    }
}

impl FromStr for ServiceListenEndpoint {
    type Err = InvalidServiceListenEndpoint;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let address: SocketAddr = text
            .parse()
            .map_err(InvalidServiceListenEndpoint::Address)?;
        let _port =
            ServicePort::try_from(address.port()).map_err(InvalidServiceListenEndpoint::Port)?;
        Ok(Self(address))
    }
}

impl fmt::Display for ServiceListenEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

crate::key::serde_via_text!(ServiceListenEndpoint);

/// A DNS origin from which the node may later derive per-service public aliases.
/// This value proves neither DNS reachability nor service/control separation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct PublicServiceBase(ServiceProxyUrl);

/// Public base validation shares the credential-free origin parser once.
#[derive(Debug, thiserror::Error)]
pub enum InvalidPublicServiceBase {
    /// Keep the concrete input-free URL cause for local diagnosis.
    #[error("the public service base origin is invalid")]
    Origin(#[source] InvalidServiceProxyUrl),
    /// Per-service aliases need a DNS base outside reserved loopback names.
    #[error("a public service base uses a DNS hostname outside loopback")]
    Dns,
}

impl PublicServiceBase {
    /// Metadata stays parsed; a caller cannot extract listener authority.
    #[must_use]
    pub const fn origin(&self) -> &ServiceProxyUrl {
        &self.0
    }
}

impl FromStr for PublicServiceBase {
    type Err = InvalidPublicServiceBase;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let origin: ServiceProxyUrl = text.parse().map_err(InvalidPublicServiceBase::Origin)?;
        if !origin.supports_public_base() {
            return Err(InvalidPublicServiceBase::Dns);
        }
        Ok(Self(origin))
    }
}

impl fmt::Display for PublicServiceBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

crate::key::serde_via_text!(PublicServiceBase);

/// Choosing a gateway alone grants no service anonymous reachability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServiceGateway {
    /// The dedicated service front door defaults to node loopback only.
    // Empty struct choices enforce unknown-field refusal at the serde edge.
    LocalOnly {},
    /// Every published service still needs separate explicit Admin consent.
    Public {
        /// Literal bind choice, independent of the node-control listener.
        listener: ServiceListenEndpoint,
        /// No credentials, path or query can enter generated service URLs.
        public_base: PublicServiceBase,
    },
}

impl Default for ServiceGateway {
    fn default() -> Self {
        Self::LocalOnly {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_services::SetServiceGateway;
    use std::error::Error as _;

    #[test]
    fn listener_literals_preserve_endpoints_and_refuse_resolution_or_zero_ports() {
        for text in ["0.0.0.0:8081", "127.0.0.1:8081", "[::1]:8081"] {
            let endpoint: ServiceListenEndpoint = text.parse().unwrap();
            assert_eq!(endpoint.address(), text.parse::<SocketAddr>().unwrap());
            assert_eq!(endpoint.to_string(), text);
            let json = serde_json::to_string(&endpoint).unwrap();
            assert_eq!(
                serde_json::from_str::<ServiceListenEndpoint>(&json).unwrap(),
                endpoint
            );
        }
        let below = ServicePort::MIN
            .get()
            .checked_sub(ServicePort::MIN.get())
            .unwrap();
        let above = u32::from(ServicePort::MAX.get())
            .checked_add(u32::from(ServicePort::MIN.get()))
            .unwrap();
        for text in [
            "localhost:8081".to_owned(),
            format!("127.0.0.1:{below}"),
            format!("127.0.0.1:{above}"),
        ] {
            let error = text.parse::<ServiceListenEndpoint>().unwrap_err();
            assert!(error.source().is_some());
            let json = serde_json::to_string(&text).unwrap();
            assert!(serde_json::from_str::<ServiceListenEndpoint>(&json).is_err());
        }
    }

    #[test]
    fn public_base_is_dns_metadata_and_default_gateway_does_not_publish() {
        let base: PublicServiceBase = "https://apps.example.test:8443/".parse().unwrap();
        assert_eq!(base.origin().as_str(), "https://apps.example.test:8443/");
        assert_eq!(ServiceGateway::default(), ServiceGateway::LocalOnly {});
        let public = ServiceGateway::Public {
            listener: "0.0.0.0:8081".parse().unwrap(),
            public_base: base,
        };
        let json = serde_json::to_string(&public).unwrap();
        assert_eq!(
            serde_json::from_str::<ServiceGateway>(&json).unwrap(),
            public
        );
        for text in [
            "http://127.0.0.1/",
            "https://192.0.2.1/",
            "http://[::1]/",
            "http://localhost/",
            "http://service.localhost/",
        ] {
            assert!(matches!(
                text.parse::<PublicServiceBase>(),
                Err(InvalidPublicServiceBase::Dns)
            ));
        }
        let error = "https://user:synthetic-secret@example.test/"
            .parse::<PublicServiceBase>()
            .unwrap_err();
        assert!(error.source().is_some());
        assert!(!format!("{error:?}").contains("synthetic-secret"));
        assert!(serde_json::from_str::<ServiceGateway>(r#"{"kind":"future"}"#).is_err());
    }

    #[test]
    fn both_gateway_choices_refuse_extra_fields_before_admin_write() {
        let local = ServiceGateway::default();
        assert_eq!(
            serde_json::to_string(&local).unwrap(),
            r#"{"kind":"local_only"}"#
        );
        let public = ServiceGateway::Public {
            listener: "0.0.0.0:8081".parse().unwrap(),
            public_base: "https://apps.example.test/".parse().unwrap(),
        };
        for gateway in [local, public] {
            let json = serde_json::to_string(&gateway).unwrap();
            assert_eq!(
                serde_json::from_str::<ServiceGateway>(&json).unwrap(),
                gateway
            );
            let fields = json.strip_suffix('}').unwrap();
            let extra = format!(r#"{fields},"consent":"synthetic"}}"#);
            assert!(serde_json::from_str::<ServiceGateway>(&extra).is_err());
            let request = format!(r#"{{"expected":null,"gateway":{extra}}}"#);
            assert!(serde_json::from_str::<SetServiceGateway>(&request).is_err());
        }
    }
}
