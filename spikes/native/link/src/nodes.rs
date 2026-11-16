//! The nodes in the user's book and how to reach each: the same book, the
//! same token files and device credentials the CLI uses, so a machine the
//! CLI reaches the app reaches too.

use std::path::{Path, PathBuf};

use vessel_client::book::{BookEntry, BookError, BookFile, NodeBook, NodeName};
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

/// A book error for the app: one that does not read becomes a typed fault
/// (the app words it; nothing of the file's text is carried), any other
/// keeps the client's sentence (none of them quotes the file).
fn book_error(error: BookError) -> LinkError {
    match error {
        BookError::Syntax { path, fault } | BookError::ProjectSyntax { path, fault } => {
            LinkError::BookFault(toml_file_fault(&path, fault))
        }
        other @ (BookError::Io { .. }
        | BookError::Entry { .. }
        | BookError::UnknownDefault { .. }
        | BookError::Unknown { .. }
        | BookError::Exists { .. }
        | BookError::Missing { .. }
        | BookError::PinnedElsewhere { .. }) => LinkError::Book(other.to_string()),
    }
}

/// One projection for pairing, fleet refresh and the Machines editor. It
/// preserves the parser's safe location without ever reformatting its input.
pub(crate) fn toml_file_fault(
    path: &Path,
    fault: vessel_client::file_fault::TomlFault,
) -> vessel_app_view::files::FileFault {
    use vessel_app_view::files::{FileFault, FileProblem};
    use vessel_client::file_fault::TomlProblem;
    FileFault {
        path: path.display().to_string(),
        at: fault.at.map(|at| (at.line, at.column)),
        problem: match fault.problem {
            TomlProblem::NotToml => FileProblem::NotToml,
            TomlProblem::UnknownKey => FileProblem::UnknownKey,
            TomlProblem::DuplicateKey => FileProblem::DuplicateKey,
            TomlProblem::MissingKey => FileProblem::MissingKey,
            TomlProblem::WrongKind => FileProblem::WrongKind,
        },
        field: fault.field.map(str::to_owned),
    }
}

/// The book's nodes, in its order.
pub(crate) fn read_book() -> Result<Vec<(NodeName, BookEntry)>, LinkError> {
    read_book_at(&NodeBook::path_in(&config_dir()?))
}

fn read_book_at(path: &Path) -> Result<Vec<(NodeName, BookEntry)>, LinkError> {
    let file = BookFile::open(path).map_err(book_error)?;
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
#[derive(Clone)]
pub struct Offer {
    /// The decoded offer.
    pub(crate) decoded: vessel_proto::PairingOffer,
    /// The key's fingerprint, in its groups of four.
    pub key: String,
    /// The node's id.
    pub node: String,
    /// Its routes, direct ones first.
    pub routes: vessel_app_view::pairing::OfferRoutes,
    /// When the link stops working, UTC.
    pub expires: String,
}

impl std::fmt::Debug for Offer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The decoded effect input can carry private URL components. Only
        // projected public facts belong in diagnostic output.
        f.debug_struct("Offer")
            .field("key", &self.key)
            .field("node", &self.node)
            .field("routes", &self.routes)
            .field("expires", &self.expires)
            .finish_non_exhaustive()
    }
}

/// Decode a pasted pairing link.
///
/// # Errors
/// The text is not a pairing link, or it does not decode.
pub fn read_link(text: &str) -> Result<Offer, vessel_proto::LinkError> {
    let offer = text
        .trim()
        .parse::<PairingLink>()
        .and_then(|link| link.decode())?;
    let routes = vessel_app_view::pairing::OfferRoutes::of(&offer);
    Ok(Offer {
        key: vessel_client::fingerprint(&offer.key),
        node: offer.node.to_string(),
        routes,
        expires: offer.expires.format("%Y-%m-%d %H:%M UTC").to_string(),
        decoded: offer,
    })
}

/// Why a machine name will not do, in terms a person can act on.
pub use vessel_app_view::pairing::NameProblem;

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
/// `prefer_node_name`: the person left the name empty (a link carries no
/// name; the node says its own only once paired), so the node's own name
/// is used when it makes a valid, free book name, else `name` (a free name
/// the app chose).
pub(crate) async fn pair(
    offer: &Offer,
    name: &str,
    prefer_node_name: bool,
) -> Result<(String, vessel_app_view::Via, String), LinkError> {
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
    let name: NodeName = if prefer_node_name {
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
    // The status line names the machine as the sidebar does: by its name
    // in the book (the node's own name may differ when the person typed
    // another).
    let shown = name.as_str().to_owned();
    tokio::task::spawn_blocking(move || -> Result<(), LinkError> {
        paired
            .credential
            .write(&path)
            .map_err(|error| LinkError::Credential(error.to_string()))?;
        let mut file = BookFile::open(&book_path).map_err(book_error)?;
        file.add(&name, &entry).map_err(book_error)
    })
    .await
    .map_err(|error| LinkError::Io(error.to_string()))??;
    let (via, address) = match route {
        vessel_client::secure::Route::Direct(url) => {
            (vessel_app_view::Via::Direct, url.to_string())
        }
        vessel_client::secure::Route::Relay(url) => (vessel_app_view::Via::Relay, url.to_string()),
    };
    Ok((shown, via, address))
}

// One typed list of area lifecycle notices. Consumers add their own Event
// here, never another connection hook. The empty producer preserves behaviour.
const CONNECTION_NOTICE_COUNT: usize = 1;
fn connection_notices(
    name: &NodeName,
    node: vessel_proto::NodeId,
) -> [crate::Event; CONNECTION_NOTICE_COUNT] {
    [crate::Event::Usage(crate::usage::Answer::Connected {
        machine: vessel_app_view::devices::MachineKey::from_book(name.as_str()),
        node,
    })]
}
impl crate::State {
    /// Existing kept offers arrive first. A later area notice is tied to the
    /// real connection, independent of whether its Fleet snapshot changes.
    pub(super) async fn connection_ready(&self, name: &NodeName, client: &NodeClient) {
        self.hand_kept(name.as_str(), client).await;
        for event in connection_notices(name, client.node().id) {
            self.emit(event).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LinkError, book_name, read_book_at};
    use vessel_app_view::Lang;

    #[test]
    fn decoded_offer_exposes_only_safe_routes_but_preserves_effect_inputs() {
        let original = vessel_proto::PairingOffer {
            v: vessel_proto::OfferVersion,
            node: "00000000-0000-4000-8000-000000000099".parse().unwrap(),
            key: vessel_proto::NodePublicKey::from_bytes([4; 32]),
            direct: vec!["wss://hidden-user:private-password@host.invalid:7443/private-path?private-query#private-fragment".parse().unwrap()],
            relays: vec!["wss://relay.invalid:443/private-ticket".parse().unwrap()],
            invite: vessel_proto::InvitationSecret::from_bytes([7; 16]),
            expires: "2026-11-20T00:00:00Z".parse().unwrap(),
        };
        let link = vessel_proto::PairingLink::encode(&original).unwrap();
        let offer = super::read_link(link.expose()).unwrap();
        assert_eq!(offer.decoded, original);
        let [direct, relay] = offer.routes.rows() else {
            panic!("two route rows required");
        };
        assert_eq!(direct.kind(Lang::En), "TCP");
        assert_eq!(&**direct.endpoint(), "host.invalid:7443");
        assert_eq!(relay.kind(Lang::Ko), "릴레이");
        assert_eq!(&**relay.endpoint(), "relay.invalid:443");
        let diagnostic = format!("{offer:?}");
        for hidden in [
            "hidden-user",
            "private-password",
            "private-path",
            "private-query",
            "private-fragment",
            "private-ticket",
        ] {
            assert!(!diagnostic.contains(hidden));
        }
    }

    /// A token written into the book (it takes a file) reaches the app as
    /// a place and a kind only: the error's Display and Debug and the
    /// footer's words in both languages never hold it (the coordinator's
    /// review of 0.0.0.0.8, 2026-11-13).
    #[test]
    fn a_book_that_does_not_read_never_shows_its_text() {
        const TOKEN: &str = "vt_live_0c4d7e2a9f81b356";
        let dir = std::env::temp_dir().join(format!("vessel-app-book-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("nodes.toml");
        std::fs::write(
            &path,
            format!("[nodes.lab]\nurl = \"ws://127.0.0.1:9/rpc\"\ntoken = \"{TOKEN}\"\n"),
        )
        .unwrap();
        let error = read_book_at(&path).unwrap_err();
        assert!(!error.to_string().contains(TOKEN), "{error}");
        assert!(!format!("{error:?}").contains(TOKEN));
        let LinkError::BookFault(fault) = error else {
            panic!("{error:?}");
        };
        assert_eq!(fault.at, Some((3, 9)));
        for lang in [Lang::En, Lang::Ko] {
            let footer = lang.machines_unreadable(&fault);
            assert!(!footer.contains(TOKEN), "{footer}");
            assert!(footer.contains("token = { file = "), "{footer}");
            assert!(footer.contains("nodes.toml"), "{footer}");
        }
        drop(std::fs::remove_dir_all(&dir));
    }

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
