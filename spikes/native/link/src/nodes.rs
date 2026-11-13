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
/// node's key's fingerprint (printed on the node with the link), its id,
/// its routes and when the link expires.
#[derive(Debug, Clone)]
pub struct Offer {
    /// The decoded offer.
    pub(crate) decoded: vessel_proto::PairingOffer,
    /// The key's fingerprint, in its groups of four.
    pub key: String,
    /// The node's id.
    pub node: String,
    /// Its routes, direct ones first.
    pub routes: Vec<String>,
    /// When the link stops working, UTC.
    pub expires: String,
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
    Ok(Offer {
        key: vessel_client::fingerprint(&offer.key),
        node: offer.node.to_string(),
        routes,
        expires: offer.expires.format("%Y-%m-%d %H:%M UTC").to_string(),
        decoded: offer,
    })
}

/// Why a machine name will not do, in terms a person can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    /// Nothing typed.
    Empty,
    /// It is the node's key (groups of hexadecimal digits), typed where a
    /// name goes: what the first build's screen led its owner to do.
    LooksLikeKey,
    /// It is a pairing link.
    LooksLikeLink,
    /// Not a node name (letters, digits, `.`, `_`, `-`).
    Invalid,
    /// The book already has a machine of that name.
    Taken,
}

/// What is wrong with `text` as a new machine's name, if anything;
/// `taken` are the book's names.
#[must_use]
pub fn name_problem(text: &str, taken: &[String]) -> Option<NameProblem> {
    let text = text.trim();
    if text.is_empty() {
        return Some(NameProblem::Empty);
    }
    if text.starts_with("vessel://") {
        return Some(NameProblem::LooksLikeLink);
    }
    let hex = text.chars().filter(char::is_ascii_hexdigit).count();
    if text.contains(' ') && hex >= 8 && text.chars().all(|c| c.is_ascii_hexdigit() || c == ' ') {
        return Some(NameProblem::LooksLikeKey);
    }
    if text.parse::<NodeName>().is_err() {
        return Some(NameProblem::Invalid);
    }
    if taken.iter().any(|name| name == text) {
        return Some(NameProblem::Taken);
    }
    None
}

/// A node's own name made into a book name: a run of characters a book
/// name cannot hold becomes one `-`, it starts and ends with a letter or a
/// digit, and it is cut to the book's 64 characters; `None` when nothing
/// is left (a name in Hangul, say: the book's names are plain ASCII).
fn book_name(own: &str) -> Option<String> {
    let mut cleaned = String::with_capacity(own.len());
    for c in own.trim().chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            cleaned.push(c);
        } else if !cleaned.ends_with('-') {
            cleaned.push('-');
        }
    }
    let edge = |c: char| !c.is_ascii_alphanumeric();
    let mut cleaned = cleaned.trim_matches(edge).to_owned();
    cleaned.truncate(64);
    let cleaned = cleaned.trim_end_matches(edge);
    cleaned
        .parse::<NodeName>()
        .is_ok()
        .then(|| cleaned.to_owned())
}

/// A name for a new machine that the book does not have: `machine`, then
/// `machine-2`, `machine-3`… (the link carries no name of the node's own;
/// the node says its name only once paired).
#[must_use]
pub fn default_name(taken: &[String]) -> String {
    let free = |name: &String| !taken.contains(name);
    std::iter::once("machine".to_owned())
        .chain((2_u32..=999).map(|n| format!("machine-{n}")))
        .find(free)
        .unwrap_or_else(|| "machine".to_owned())
}

/// Pair with the node `offer` names, write the credential, and add the
/// node to the book as `name` (as `vessel nodes add NAME --pair` does: the
/// credential first, then the entry).
///
/// `prefer_node_name`: the person kept the name the app filled in, which
/// was only a placeholder (a link carries no name; the node says its own
/// only once paired), so the node's own name is used when it makes a
/// valid, free book name.
pub(crate) async fn pair(
    offer: &Offer,
    name: &str,
    prefer_node_name: bool,
) -> Result<(String, String), LinkError> {
    let name = name
        .trim()
        .parse::<NodeName>()
        .map_err(|error| LinkError::Name(error.to_string()))?;
    let config = config_dir()?;
    let book_path = NodeBook::path_in(&config);
    // A taken name is refused before the invitation is spent (as the CLI
    // does): the link works once.
    if read_book()?.iter().any(|(taken, _)| taken == &name) {
        return Err(LinkError::Name(format!("{name} is already in the book")));
    }
    let offer = &offer.decoded;
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
    let name = if prefer_node_name {
        let taken: Vec<String> = read_book()?
            .iter()
            .map(|(taken, _)| taken.as_str().to_owned())
            .collect();
        book_name(&node_name)
            .filter(|own| !taken.contains(own))
            .and_then(|own| own.parse::<NodeName>().ok())
            .unwrap_or(name)
    } else {
        name
    };
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
    Ok((node_name, route.to_string()))
}

#[cfg(test)]
mod tests {
    use super::book_name;

    #[test]
    fn a_node_name_becomes_a_book_name() {
        assert_eq!(book_name("build-server").as_deref(), Some("build-server"));
        assert_eq!(
            book_name("  JUN's laptop! ").as_deref(),
            Some("JUN-s-laptop")
        );
        assert_eq!(book_name("-.lab.-").as_deref(), Some("lab"));
        assert_eq!(book_name("노트북"), None);
        assert_eq!(book_name(""), None);
        let long = "a".repeat(70);
        assert_eq!(book_name(&long).map(|name| name.len()), Some(64));
    }
}
