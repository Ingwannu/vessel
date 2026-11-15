//! A saved direct route can suggest a browser host; relay/SSH destinations
//! cannot. No credential or book parse diagnostic leaves this file boundary.
use std::path::PathBuf;
use vessel_app_view::connections::Endpoint;
use vessel_app_view::workspace::scripts::PreviewHost;
use vessel_client::NodeAddress;
use vessel_client::book::BookFile;

pub(super) async fn read(path: Option<PathBuf>, machine: String) -> Option<PreviewHost> {
    let path = path?;
    tokio::task::spawn_blocking(move || {
        let file = BookFile::open(path).ok()?;
        let (_, entry) = file.book().get(&machine)?;
        match &entry.address {
            NodeAddress::Ws(url) => host(&Endpoint::of_url(&url.to_string())),
            // A saved secure address may connect through a relay instead;
            // guessing a saved direct route is not proof of reachability.
            NodeAddress::Secure(_) | NodeAddress::Ssh(_) => None,
        }
    })
    .await
    .ok()
    .flatten()
}
fn host(endpoint: &Endpoint) -> Option<PreviewHost> {
    let authority = endpoint.as_str();
    let host = if let Some(v6) = authority.strip_prefix('[') {
        v6.split_once(']')?.0
    } else {
        authority
            .rsplit_once(':')
            .map_or(authority, |(host, _port)| host)
    };
    PreviewHost::parse(host).ok()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_destination_loses_rpc_port_path_and_credentials() {
        let endpoint = Endpoint::of_url("ws://user@example.com:7077/rpc?secret=x");
        assert_eq!(
            host(&endpoint).unwrap().url(3000).unwrap().as_str(),
            "http://example.com:3000/"
        );
        let v6 = Endpoint::of_url("ws://[::1]:7077/rpc");
        assert_eq!(
            host(&v6).unwrap().url(3000).unwrap().as_str(),
            "http://[::1]:3000/"
        );
    }
}
