//! Native thread config for the already resolved MCP launch snapshot.
//! Reuse the fleet's typed transports; do not create a second secret resolver.

use serde::{Serialize, Serializer};
use std::{collections::BTreeMap, fmt};
use vessel_proto::{
    EnvName, HeaderName, HttpUrl, McpServerName, McpTransport, ResolvedMcpServers, ResolvedValue,
};

#[derive(Serialize)]
pub(super) struct ThreadConfig {
    mcp_servers: BTreeMap<McpServerName, Server>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Server {
    Stdio {
        command: std::path::PathBuf,
        args: Vec<String>,
        env: BTreeMap<EnvName, NativeValue>,
    },
    Http {
        url: HttpUrl,
        http_headers: BTreeMap<HeaderName, NativeValue>,
    },
}

struct NativeValue(ResolvedValue);
impl Serialize for NativeValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.expose())
    }
}

impl ThreadConfig {
    pub(super) fn from_servers(servers: &ResolvedMcpServers) -> Option<Self> {
        if servers.is_empty() {
            return None;
        }
        let mcp_servers = servers
            .iter()
            .map(|(name, server)| {
                let server = match server {
                    McpTransport::Stdio { command, args, env } => Server::Stdio {
                        command: command.clone(),
                        args: args.clone(),
                        env: env
                            .iter()
                            .map(|(key, value)| (key.clone(), NativeValue(value.clone())))
                            .collect(),
                    },
                    McpTransport::Http { url, headers } => Server::Http {
                        url: url.clone(),
                        http_headers: headers
                            .iter()
                            .map(|(key, value)| (key.clone(), NativeValue(value.clone())))
                            .collect(),
                    },
                };
                (name.clone(), server)
            })
            .collect();
        Some(Self { mcp_servers })
    }
}

impl fmt::Debug for ThreadConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CodexThreadConfig")
            .field("server_count", &self.mcp_servers.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_config_debug_never_contains_launch_values() {
        let servers = BTreeMap::from([(
            "private-server".parse().unwrap(),
            McpTransport::Stdio {
                command: "private-command".into(),
                args: vec!["private-argument".into()],
                env: BTreeMap::from([(
                    "PRIVATE_ENV".parse().unwrap(),
                    ResolvedValue::new("private-value".into()),
                )]),
            },
        )]);
        let config = ThreadConfig::from_servers(&servers).unwrap();
        let debug = format!("{config:?}");
        assert!(debug.contains("server_count: 1"));
        assert!(!debug.contains("private"));
        assert!(!debug.contains("PRIVATE_ENV"));
        assert!(ThreadConfig::from_servers(&ResolvedMcpServers::new()).is_none());
    }
}
