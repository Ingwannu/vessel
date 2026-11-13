//! The nodes in the user's book and how to reach each: the same book, the
//! same token files and device credentials the CLI uses, so a machine the
//! CLI reaches the app reaches too.

use std::path::{Path, PathBuf};

use vessel_client::book::{BookEntry, BookFile, NodeBook, NodeName};
use vessel_client::secure::{RouteTimeouts, SecureConnector, WsDialer, pair_over};
use vessel_client::{
    BearerToken, Connector as _, CredentialSource, DeviceCredential, NodeAddress, NodeClient,
    Pinned, SecureTarget, WsConnector,
};
use vessel_proto::{ClientInfo, PairingLink};

use crate::LinkError;

/// Where the book and the credentials are: `VESSEL_CONFIG_DIR` (as the
/// CLI reads it), else the system's configuration directory.
pub(crate) fn config_dir() -> Result<PathBuf, LinkError> {
    std::env::var_os("VESSEL_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .ok_or(LinkError::NoHome)
}

/// The book's nodes, in its order.
pub(crate) fn read_book() -> Result<Vec<(NodeName, BookEntry)>, LinkError> {
    let path = NodeBook::path_in(&config_dir()?);
    let file = BookFile::open(&path).map_err(|error| LinkError::Book(error.to_string()))?;
    Ok(file
        .book()
        .nodes
        .iter()
        .map(|(name, entry)| (name.clone(), entry.clone()))
        .collect())
}

/// How this app introduces itself to a node.
pub(crate) fn client_info(label: &str) -> ClientInfo {
    ClientInfo {
        name: "vessel-app".to_owned(),
        version: label.to_owned(),
    }
}

/// Connect to one book node by its route, refusing any node but the one
/// pinned in the book.
pub(crate) async fn connect(
    entry: &BookEntry,
    client: ClientInfo,
) -> Result<NodeClient, LinkError> {
    let home = dirs::home_dir();
    match &entry.address {
        NodeAddress::Ws(url) => {
            let source = entry.token.as_ref().ok_or(LinkError::NoToken)?;
            let path = source.path(home.as_deref());
            let text = read_file(path).await?;
            let token = text
                .trim()
                .parse::<BearerToken>()
                .map_err(|error| LinkError::Token(error.to_string()))?;
            Pinned::new(WsConnector::new(url.clone(), token), entry.id)
                .connect(client)
                .await
                .map_err(|error| LinkError::Connect(error.to_string()))
        }
        NodeAddress::Secure(target) => {
            let credential = read_credential(entry.credential.as_ref(), home.as_deref()).await?;
            let id = entry.id.unwrap_or(credential.node);
            let dialer = WsDialer::for_routes(target.routes())
                .map_err(|error| LinkError::Connect(error.to_string()))?;
            let connector = SecureConnector::new(dialer, target.clone(), id, credential)
                .map_err(|error| LinkError::Connect(error.to_string()))?;
            Pinned::new(connector, Some(id))
                .connect(client)
                .await
                .map_err(|error| LinkError::Connect(error.to_string()))
        }
        // The SSH route needs `vessel-remote` (an `ssh` child and the
        // node's install); left out of the first builds to keep them small.
        NodeAddress::Ssh(_) => Err(LinkError::SshNotYet),
    }
}

async fn read_file(path: PathBuf) -> Result<String, LinkError> {
    tokio::task::spawn_blocking(move || std::fs::read_to_string(&path))
        .await
        .map_err(|error| LinkError::Io(error.to_string()))?
        .map_err(|error| LinkError::Io(error.to_string()))
}

async fn read_credential(
    source: Option<&CredentialSource>,
    home: Option<&Path>,
) -> Result<DeviceCredential, LinkError> {
    let path = source.ok_or(LinkError::NoCredential)?.path(home);
    tokio::task::spawn_blocking(move || DeviceCredential::read(&path))
        .await
        .map_err(|error| LinkError::Io(error.to_string()))?
        .map_err(|error| LinkError::Credential(error.to_string()))
}

/// What a pairing link offers, for a person to check before pairing: the
/// node's id and its key's fingerprint (what `vessel node status` prints
/// on the node), and its routes.
#[derive(Debug, Clone)]
pub struct Offer {
    /// The decoded offer.
    pub(crate) offer: vessel_proto::PairingOffer,
    /// A line to show: id, fingerprint, routes, expiry.
    pub summary: String,
}

/// Decode a pasted pairing link.
///
/// # Errors
/// The text is not a pairing link, or it does not decode.
pub fn read_link(text: &str) -> Result<Offer, LinkError> {
    let offer = text
        .trim()
        .parse::<PairingLink>()
        .and_then(|link| link.decode())
        .map_err(|error| LinkError::PairingLink(error.to_string()))?;
    let routes: Vec<String> = offer
        .direct
        .iter()
        .chain(&offer.relays)
        .map(ToString::to_string)
        .collect();
    let summary = format!(
        "node {} · key {} · via {} · expires {}",
        offer.node,
        vessel_client::fingerprint(&offer.key),
        routes.join(", "),
        offer.expires.format("%Y-%m-%d %H:%M UTC"),
    );
    Ok(Offer { offer, summary })
}

/// Pair with the node `offer` names, write the credential, and add the
/// node to the book as `name` (as `vessel nodes add NAME --pair` does: the
/// credential first, then the entry).
pub(crate) async fn pair(offer: &Offer, name: &str) -> Result<String, LinkError> {
    let name = name
        .trim()
        .parse::<NodeName>()
        .map_err(|error| LinkError::Name(error.to_string()))?;
    let config = config_dir()?;
    let book_path = NodeBook::path_in(&config);
    let offer = &offer.offer;
    let dialer = WsDialer::for_routes(offer.direct.iter().chain(&offer.relays))
        .map_err(|error| LinkError::Connect(error.to_string()))?;
    let device_name = hostname::get()
        .ok()
        .and_then(|name| name.into_string().ok())
        .unwrap_or_else(|| "device".to_owned());
    let (paired, route) = pair_over(&dialer, offer, &device_name, RouteTimeouts::default())
        .await
        .map_err(|error| LinkError::Connect(error.to_string()))?;
    let path = DeviceCredential::path_in(&config, offer.node);
    let entry = BookEntry {
        address: NodeAddress::Secure(SecureTarget {
            key: offer.key,
            relays: offer.relays.clone(),
            direct: offer.direct.clone(),
        }),
        token: None,
        credential: Some(CredentialSource::File(path.clone())),
        id: Some(offer.node),
    };
    let node_name = paired.node.name.clone();
    tokio::task::spawn_blocking(move || -> Result<(), LinkError> {
        paired
            .credential
            .write(&path)
            .map_err(|error| LinkError::Credential(error.to_string()))?;
        let mut file =
            BookFile::open(&book_path).map_err(|error| LinkError::Book(error.to_string()))?;
        file.add(&name, &entry)
            .map_err(|error| LinkError::Book(error.to_string()))
    })
    .await
    .map_err(|error| LinkError::Io(error.to_string()))??;
    Ok(format!("paired with {node_name} via {route}"))
}
