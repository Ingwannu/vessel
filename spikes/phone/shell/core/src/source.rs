//! [`PhoneSource`]: the nodes the phone app offers its page.
//!
//! [Decision Log]
//! - Purpose: the bridge names nodes and asks its source how to reach
//!   each (`vessel_web::NodeSource`); on a desktop that is the CLI's book.
//! - Constraint: a phone has no CLI book, no `vesseld` of its own, no
//!   `ssh` and no home directory; what it can keep is the app's private
//!   storage, which Android gives no other app.
//! - Choice: a book of its own, in the CLI's format and written by the
//!   CLI's `BookFile` (so the page's edits are the same edits), under the
//!   app's private files directory; device credentials beside it, where
//!   `vessel nodes add --pair` would put them under a configuration
//!   directory; the page's preferences beside the book and its drafts in
//!   the data directory, as `vessel ui` keeps them. Pairing is the CLI's
//!   (`pair_over` over the link's routes); the first node paired becomes
//!   the default, since there is no node "here" to fall back on.
//! - Not listed: this machine's node (`NodeKey::Here`), which a phone does
//!   not run; asking to reach it is an error that says so.
//! - Cost: a book copied from a desktop with SSH entries lists them, and
//!   reaching one says why it cannot. Credentials are files with mode 0600
//!   in the app's sandbox, not in Android's keystore: the sandbox is what
//!   keeps other apps out; the keystore would also keep a rooted phone's
//!   backups from carrying them (backups are off in the manifest).

use std::path::PathBuf;

use vessel_client::book::{BookEntry, BookError, BookFile, NodeBook, NodeName};
use vessel_client::secure::{DialError, ReachError, RouteTimeouts, SecureConnector, pair_over};
use vessel_client::{
    BearerToken, CredentialError, CredentialMismatch, CredentialSource, DeviceCredential,
    NodeAddress, Pinned, SecureTarget, WsConnector, fingerprint,
};
use vessel_proto::{LinkError, NodeId, PairingLink, PairingOffer};
use vessel_web::wire::{BookNode, NodeKey, SecretText, Transport};
use vessel_web::{NodeSource, PageFiles};

use crate::connect::NodeConnector;
use crate::roots::{Roots, RootsError};

/// The page's preferences, beside the book (as `vessel ui` keeps them).
const PREFERENCES_FILE: &str = "ui-preferences.json";

/// The page's drafts, in the data directory (as `vessel ui` keeps them).
const DRAFTS_FILE: &str = "ui-drafts.json";

/// The app's private directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDirs {
    /// The book, the device credentials and the page's preferences
    /// (`<config>/vessel/…`, the layout of a desktop's configuration
    /// directory).
    pub config: PathBuf,
    /// The page's drafts.
    pub data: PathBuf,
}

impl AppDirs {
    /// The book's file.
    #[must_use]
    pub fn book(&self) -> PathBuf {
        NodeBook::path_in(&self.config)
    }
}

/// The phone app's nodes: its own book, and pairing into it.
#[derive(Debug)]
pub struct PhoneSource {
    dirs: AppDirs,
    roots: Roots,
    /// The name a node shows for this device once paired.
    device: String,
    /// Held across each edit of the book (read, change, write), so the
    /// page's edits and a first contact's pin take turns. A `tokio` lock:
    /// pairing holds it across the node's handshake.
    edits: tokio::sync::Mutex<()>,
}

/// Why the phone's source could not answer. Shown to the user as text,
/// with its sources.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    /// The book could not be read or changed.
    #[error(transparent)]
    Book(#[from] BookError),
    /// The page named a node the book does not have.
    #[error("{0} is not in this app's list of machines")]
    NotInBook(NodeName),
    /// The page asked for this device's own node; a phone runs none.
    #[error("no Vessel node runs on this phone; add a machine with a pairing link")]
    NoNodeHere,
    /// An SSH entry (a book copied from a desktop).
    #[error("{0} is reached over SSH, which this app cannot start; pair with it instead")]
    Ssh(NodeName),
    /// A websocket entry without a token file.
    #[error("{0} is a websocket address without a token; pair with it instead")]
    TokenRequired(NodeName),
    /// A websocket entry's token file could not be read.
    #[error("the token file of {node} could not be read")]
    TokenFile {
        /// Whose.
        node: NodeName,
        /// Why.
        #[source]
        source: std::io::Error,
    },
    /// A websocket entry's token file holds no token.
    #[error("the token file of {0} does not hold a token")]
    UnusableToken(NodeName),
    /// A paired entry without its credential.
    #[error("{0} has no device credential; pair with it again")]
    NoCredential(NodeName),
    /// A credential file could not be read or written.
    #[error(transparent)]
    Credential(#[from] CredentialError),
    /// A credential that is another node's.
    #[error(transparent)]
    Mismatch(#[from] CredentialMismatch),
    /// The pasted text is not a pairing link (never echoed: it may be one).
    #[error("this is not a Vessel pairing link")]
    PairingLink(#[source] LinkError),
    /// No dialer for the node's routes.
    #[error(transparent)]
    Roots(#[from] RootsError),
    /// No route paired, or the node refused the invitation.
    #[error("pairing did not go through")]
    Pair(#[source] ReachError<DialError>),
    /// Work moved off the runtime's workers did not finish.
    #[error("the app's file work stopped")]
    Join(#[from] tokio::task::JoinError),
}

impl PhoneSource {
    /// A source keeping its files under `dirs`, checking `wss://` relays
    /// against `roots`, and introducing itself to nodes as `device`.
    #[must_use]
    pub fn new(dirs: AppDirs, roots: Roots, device: String) -> Self {
        Self {
            dirs,
            roots,
            device,
            edits: tokio::sync::Mutex::new(()),
        }
    }

    /// The book as it is on disk now.
    async fn book(&self) -> Result<NodeBook, SourceError> {
        let path = self.dirs.book();
        let file = tokio::task::spawn_blocking(move || BookFile::open(&path)).await??;
        Ok(file.book().clone())
    }

    /// Open the book, apply `change` and write it, in turn with this
    /// process's other edits.
    async fn edit(
        &self,
        change: impl FnOnce(&mut BookFile) -> Result<(), BookError> + Send + 'static,
    ) -> Result<(), SourceError> {
        let path = self.dirs.book();
        let _turn = self.edits.lock().await;
        tokio::task::spawn_blocking(move || {
            let mut file = BookFile::open(&path)?;
            change(&mut file)
        })
        .await??;
        Ok(())
    }

    /// The connector for a websocket entry: its token file.
    async fn websocket(
        name: &NodeName,
        entry: &BookEntry,
        url: &vessel_client::NodeUrl,
    ) -> Result<NodeConnector, SourceError> {
        let source = entry
            .token
            .as_ref()
            .ok_or_else(|| SourceError::TokenRequired(name.clone()))?;
        let path = source.path(None);
        let text = tokio::task::spawn_blocking(move || std::fs::read_to_string(path))
            .await?
            .map_err(|source| SourceError::TokenFile {
                node: name.clone(),
                source,
            })?;
        let token: BearerToken = text
            .trim()
            .parse()
            .map_err(|_invalid| SourceError::UnusableToken(name.clone()))?;
        Ok(NodeConnector::Ws(Pinned::new(
            WsConnector::new(url.clone(), token),
            entry.id,
        )))
    }

    /// The connector for a paired entry: its credential, bound to the
    /// node's key and id (the book's pinned id, or else the credential's
    /// own), checked before anything is dialled.
    async fn paired(
        &self,
        name: &NodeName,
        entry: &BookEntry,
        target: &SecureTarget,
    ) -> Result<NodeConnector, SourceError> {
        let source = entry
            .credential
            .as_ref()
            .ok_or_else(|| SourceError::NoCredential(name.clone()))?;
        let path = source.path(None);
        let credential =
            tokio::task::spawn_blocking(move || DeviceCredential::read(&path)).await??;
        let id = entry.id.unwrap_or(credential.node);
        let dialer = self.roots.dialer(target.routes())?;
        let connector = SecureConnector::new(dialer, target.clone(), id, credential)?;
        Ok(NodeConnector::Secure(Box::new(Pinned::new(
            connector,
            Some(id),
        ))))
    }

    /// Write the credential `pairing` returned and add the node to the
    /// book as `name`; the first node becomes the default.
    async fn keep(
        &self,
        name: &NodeName,
        offer: &PairingOffer,
        credential: DeviceCredential,
    ) -> Result<(), SourceError> {
        let path = DeviceCredential::path_in(&self.dirs.config, offer.node);
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
        let book = self.dirs.book();
        let name = name.clone();
        tokio::task::spawn_blocking(move || -> Result<(), SourceError> {
            // The credential first: an entry without it would be unusable,
            // while a credential without an entry is an unused file (and
            // the device can be revoked on the node), as the CLI does.
            credential.write(&path)?;
            let mut file = BookFile::open(&book)?;
            file.add(&name, &entry)?;
            if file.book().default_node.is_none() {
                file.set_default(Some(name.as_str()))?;
            }
            Ok(())
        })
        .await?
    }
}

impl NodeSource for PhoneSource {
    type Connector = NodeConnector;
    type Error = SourceError;

    async fn nodes(&self) -> Result<Vec<BookNode>, SourceError> {
        let book = self.book().await?;
        Ok(book
            .nodes
            .iter()
            .map(|(name, entry)| listed_node(name, entry, &book))
            .collect())
    }

    async fn connector(&self, node: &NodeKey) -> Result<NodeConnector, SourceError> {
        let name = match node {
            NodeKey::Here => return Err(SourceError::NoNodeHere),
            NodeKey::Book(name) => name,
        };
        let book = self.book().await?;
        let entry = book
            .nodes
            .get(name)
            .ok_or_else(|| SourceError::NotInBook(name.clone()))?;
        match &entry.address {
            NodeAddress::Ws(url) => Self::websocket(name, entry, url).await,
            NodeAddress::Ssh(_) => Err(SourceError::Ssh(name.clone())),
            NodeAddress::Secure(target) => self.paired(name, entry, target).await,
        }
    }

    async fn seen(&self, node: &NodeKey, id: NodeId) {
        let name = match node {
            NodeKey::Here => return,
            NodeKey::Book(name) => name.clone(),
        };
        // Pinned once, on first contact, as the CLI does.
        let Ok(book) = self.book().await else {
            return;
        };
        if book.nodes.get(&name).is_some_and(|entry| entry.id.is_none()) {
            let pinned = self.edit(move |file| file.pin(name.as_str(), id)).await;
            if let Err(error) = pinned {
                tracing::warn!(%error, "could not record the node's id in the book");
            }
        }
    }

    async fn pair(&self, name: &NodeName, link: &SecretText) -> Result<BookNode, SourceError> {
        let offer = link
            .expose()
            .trim()
            .parse::<PairingLink>()
            .and_then(|link| link.decode())
            .map_err(SourceError::PairingLink)?;
        let turn = self.edits.lock().await;
        // A taken name is refused before an invitation is spent.
        if self.book().await?.get(name.as_str()).is_some() {
            return Err(BookError::Exists {
                path: self.dirs.book(),
                name: name.clone(),
            }
            .into());
        }
        let dialer = self
            .roots
            .dialer(offer.direct.iter().chain(&offer.relays))?;
        let (paired, route) = pair_over(&dialer, &offer, &self.device, RouteTimeouts::default())
            .await
            .map_err(SourceError::Pair)?;
        tracing::info!(node = %paired.node.id, %route, key = %fingerprint(&offer.key), "paired");
        self.keep(name, &offer, paired.credential).await?;
        drop(turn);
        let book = self.book().await?;
        let (name, entry) = book
            .get(name.as_str())
            .ok_or_else(|| SourceError::NotInBook(name.clone()))?;
        Ok(listed_node(name, entry, &book))
    }

    async fn forget(&self, node: &NodeName) -> Result<(), SourceError> {
        let name = node.clone();
        self.edit(move |file| file.remove(name.as_str())).await
    }

    async fn rename(&self, node: &NodeName, to: &NodeName) -> Result<(), SourceError> {
        let (from, to) = (node.clone(), to.clone());
        self.edit(move |file| file.rename(from.as_str(), &to)).await
    }

    async fn set_default(&self, node: &NodeKey) -> Result<(), SourceError> {
        let name = match node {
            NodeKey::Here => None,
            NodeKey::Book(name) => Some(name.clone()),
        };
        self.edit(move |file| file.set_default(name.as_ref().map(NodeName::as_str)))
            .await
    }

    fn preferences(&self) -> Result<PageFiles, SourceError> {
        Ok(page_files(&self.dirs))
    }
}

/// Where the page's files go under the app's directories.
fn page_files(dirs: &AppDirs) -> PageFiles {
    PageFiles {
        preferences: Some(dirs.book().with_file_name(PREFERENCES_FILE)),
        drafts: Some(dirs.data.join(DRAFTS_FILE)),
    }
}

/// A book entry as the page lists it (the CLI's listing, without this
/// machine's node).
fn listed_node(name: &NodeName, entry: &BookEntry, book: &NodeBook) -> BookNode {
    let (address, transport) = match &entry.address {
        NodeAddress::Ws(url) => (url.to_string(), Transport::Ws),
        NodeAddress::Ssh(target) => (target.to_string(), Transport::Ssh),
        NodeAddress::Secure(target) => (
            format!("key {}", fingerprint(&target.key)),
            Transport::Paired,
        ),
    };
    BookNode {
        key: NodeKey::from(name.clone()),
        label: name.to_string(),
        address,
        transport,
        default: book.default_node.as_ref() == Some(name),
        id: entry.id,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn dirs(root: &Path) -> AppDirs {
        AppDirs {
            config: root.join("files"),
            data: root.join("data"),
        }
    }

    #[test]
    fn the_pages_files_are_beside_the_book_and_in_the_data_directory() {
        let root = tempfile::tempdir().unwrap();
        let dirs = dirs(root.path());
        let files = page_files(&dirs);
        assert_eq!(
            files.preferences.unwrap(),
            root.path()
                .join("files")
                .join("vessel")
                .join("ui-preferences.json")
        );
        assert_eq!(
            files.drafts.unwrap(),
            root.path().join("data").join("ui-drafts.json")
        );
        // The app's paths are absolute: `~` is never written into the book
        // and never expanded (a phone has no home in the CLI's sense).
        assert!(dirs.book().is_absolute());
    }

    #[tokio::test]
    async fn no_node_runs_here_and_an_empty_book_lists_nothing() {
        let root = tempfile::tempdir().unwrap();
        let source = PhoneSource::new(dirs(root.path()), Roots::none(), "phone".to_owned());
        assert!(source.nodes().await.unwrap().is_empty());
        assert!(matches!(
            source.connector(&NodeKey::here()).await,
            Err(SourceError::NoNodeHere)
        ));
        let ghost: NodeKey = "ghost".parse().unwrap();
        assert!(matches!(
            source.connector(&ghost).await,
            Err(SourceError::NotInBook(_))
        ));
    }

    #[tokio::test]
    async fn an_ssh_entry_from_a_desktops_book_says_why_it_cannot_be_reached() {
        let root = tempfile::tempdir().unwrap();
        let dirs = dirs(root.path());
        std::fs::create_dir_all(dirs.book().parent().unwrap()).unwrap();
        std::fs::write(dirs.book(), "[nodes.box]\nssh = \"me@box\"\n").unwrap();
        let source = PhoneSource::new(dirs, Roots::none(), "phone".to_owned());
        let listed = source.nodes().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed.first().unwrap().transport, Transport::Ssh);
        let key: NodeKey = "box".parse().unwrap();
        assert!(matches!(
            source.connector(&key).await,
            Err(SourceError::Ssh(_))
        ));
    }

    #[tokio::test]
    async fn text_that_is_not_a_link_is_refused_without_being_echoed() {
        let root = tempfile::tempdir().unwrap();
        let source = PhoneSource::new(dirs(root.path()), Roots::none(), "phone".to_owned());
        let secret = "vessel://pair#not-really-a-link-but-secret";
        let error = source
            .pair(
                &"box".parse().unwrap(),
                &SecretText::new(secret.to_owned()),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, SourceError::PairingLink(_)));
        assert!(!format!("{error} {error:?}").contains("secret"));
    }
}
