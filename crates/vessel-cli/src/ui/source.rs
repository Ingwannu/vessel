//! `vessel ui`'s nodes: the book's, and this machine's (ADR 0017).
//!
//! The bridge (`vessel_web`) names nodes and asks this source for a way to
//! reach each; everything that needs the CLI's knowledge stays here:
//! the book, token files, SSH, device credentials, pinned ids, pairing.
//!
//! [Decision Log]
//! - Only book names and this machine's node are reached. `vessel
//!   --node` also takes an address; the UI does not, so nothing a page
//!   sends can make this process dial a host (or start `ssh` to one) that
//!   the user did not put in the book.
//! - `--node` and `--token` (and `VESSEL_NODE`, `VESSEL_TOKEN`) are
//!   ignored: they name one node for one command, and a token given for
//!   one node must not be sent to every node the UI lists.
//! - The book is read again on every request, so a node added with `vessel
//!   nodes add` while the UI runs shows up without a restart.
//! - The page's edits (pair, forget, rename, default, and the id pinned on
//!   first contact) are `BookFile`'s, the same calls `vessel nodes` makes,
//!   each on a book opened right before the edit: a change made with the
//!   CLI while the page is open is never written over from a stale copy.
//!   Within this process they take turns ([`BookSource::edits`]), so two
//!   tabs (or a tab and a first contact's pin) cannot lose each other's
//!   edit; against another `vessel` process the window is one
//!   read-edit-rename, as between two CLI commands.

use vessel_client::NodeAddress;
use vessel_client::book::{BookEntry, BookError, BookFile, NodeBook, NodeName, Origin, Resolved};
use vessel_proto::{NodeId, PairingLink};
use vessel_web::NodeSource;
use vessel_web::wire::{BookNode, NodeKey, SecretText, Transport};

use crate::args::Target;
use crate::connect::{self, NodeConnector};
use crate::error::CliError;
use crate::{nodes, pair};

/// The book and this machine's node, for `vessel ui`.
#[derive(Debug)]
pub(crate) struct BookSource {
    /// The data and configuration directories only (see the module's
    /// notes).
    target: Target,
    /// Held across each edit of the book (read, change, write), so this
    /// process's edits take turns. A `tokio` lock because pairing holds it
    /// across the node's handshake, which is awaited.
    edits: tokio::sync::Mutex<()>,
}

impl BookSource {
    pub(crate) fn new(target: &Target) -> Self {
        Self {
            target: Target {
                node: None,
                token: None,
                data_dir: target.data_dir.clone(),
                config_dir: target.config_dir.clone(),
            },
            edits: tokio::sync::Mutex::new(()),
        }
    }

    /// Open the book, apply `change` and write it, in turn with this
    /// process's other edits.
    async fn edit(
        &self,
        change: impl FnOnce(&mut BookFile) -> Result<(), BookError> + Send + 'static,
    ) -> Result<(), CliError> {
        let path = nodes::book_path(&self.target)?;
        let _turn = self.edits.lock().await;
        tokio::task::spawn_blocking(move || {
            let mut file = BookFile::open(&path)?;
            change(&mut file)
        })
        .await??;
        Ok(())
    }

    async fn book(&self) -> Result<(NodeBook, std::path::PathBuf), CliError> {
        let path = nodes::book_path(&self.target)?;
        let reading = path.clone();
        let file = tokio::task::spawn_blocking(move || BookFile::open(&reading)).await??;
        Ok((file.book().clone(), path))
    }
}

impl NodeSource for BookSource {
    type Connector = NodeConnector;
    type Error = CliError;

    async fn nodes(&self) -> Result<Vec<BookNode>, CliError> {
        let (book, _path) = self.book().await?;
        let mut listed = vec![BookNode {
            key: NodeKey::here(),
            label: "this machine".to_owned(),
            address: nodes::local_node()?.to_string(),
            transport: Transport::Ws,
            default: book.default_node.is_none(),
            id: None,
        }];
        listed.extend(
            book.nodes
                .iter()
                .map(|(name, entry)| listed_node(name, entry, &book)),
        );
        Ok(listed)
    }

    async fn connector(&self, node: &NodeKey) -> Result<NodeConnector, CliError> {
        let resolved = match node {
            NodeKey::Here => Resolved {
                name: None,
                address: NodeAddress::Ws(nodes::local_node()?),
                token: None,
                credential: None,
                pinned: None,
                origin: Origin::Default,
            },
            NodeKey::Book(name) => {
                let (book, _path) = self.book().await?;
                let entry = book
                    .nodes
                    .get(name)
                    .ok_or_else(|| CliError::NotInBook(name.to_string()))?;
                Resolved {
                    name: Some(name.clone()),
                    address: entry.address.clone(),
                    token: entry.token.clone(),
                    credential: entry.credential.clone(),
                    pinned: entry.id,
                    origin: Origin::Named,
                }
            }
        };
        connect::connector(&self.target, &resolved).await
    }

    async fn seen(&self, node: &NodeKey, id: NodeId) {
        let name = match node {
            NodeKey::Here => return,
            NodeKey::Book(name) => name,
        };
        // Pinned once, on first contact, as the CLI does.
        let Ok((book, path)) = self.book().await else {
            return;
        };
        if book.nodes.get(name).is_some_and(|entry| entry.id.is_none()) {
            let _turn = self.edits.lock().await;
            nodes::pin(&path, name, id).await;
        }
    }

    async fn pair(&self, name: &NodeName, link: &SecretText) -> Result<BookNode, CliError> {
        let offer = link
            .expose()
            .trim()
            .parse::<PairingLink>()
            .and_then(|link| link.decode())
            .map_err(CliError::PairingLink)?;
        let turn = self.edits.lock().await;
        let _joined = pair::with_offer(&self.target, name, &offer, false).await?;
        drop(turn);
        let (book, _path) = self.book().await?;
        let (name, entry) = book
            .get(name.as_str())
            .ok_or_else(|| CliError::NotInBook(name.to_string()))?;
        Ok(listed_node(name, entry, &book))
    }

    async fn forget(&self, node: &NodeName) -> Result<(), CliError> {
        let name = node.clone();
        self.edit(move |file| file.remove(name.as_str())).await
    }

    async fn rename(&self, node: &NodeName, to: &NodeName) -> Result<(), CliError> {
        let (from, to) = (node.clone(), to.clone());
        self.edit(move |file| file.rename(from.as_str(), &to)).await
    }

    async fn set_default(&self, node: &NodeKey) -> Result<(), CliError> {
        // This machine's node is the default exactly when the book names
        // none (`nodes` lists it so), as `vessel nodes default --none`.
        let name = match node {
            NodeKey::Here => None,
            NodeKey::Book(name) => Some(name.clone()),
        };
        self.edit(move |file| file.set_default(name.as_ref().map(NodeName::as_str)))
            .await
    }
}

/// A book entry as the UI lists it.
fn listed_node(name: &NodeName, entry: &BookEntry, book: &NodeBook) -> BookNode {
    let (address, transport) = match &entry.address {
        NodeAddress::Ws(url) => (url.to_string(), Transport::Ws),
        NodeAddress::Ssh(target) => (target.to_string(), Transport::Ssh),
        NodeAddress::Secure(target) => (
            format!("key {}", vessel_client::fingerprint(&target.key)),
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

    use vessel_client::NodeAddress;
    use vessel_client::book::{BookEntry, BookError, BookFile, NodeBook, NodeName};
    use vessel_web::NodeSource as _;
    use vessel_web::wire::NodeKey;

    use super::BookSource;
    use crate::args::Target;
    use crate::error::CliError;

    const BOOK: &str = r#"# Mine.
default_node = "box"

[nodes.box]
ssh = "me@box"   # the big one

[nodes.lab]
ssh = "me@lab"
"#;

    /// A source over a book holding `BOOK`, in a temporary configuration
    /// directory; the book's path.
    fn source(dir: &Path) -> (BookSource, std::path::PathBuf) {
        let path = NodeBook::path_in(dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, BOOK).unwrap();
        let target = Target {
            node: None,
            token: None,
            data_dir: Some(dir.join("data")),
            config_dir: Some(dir.to_path_buf()),
        };
        (BookSource::new(&target), path)
    }

    fn ssh(host: &str) -> BookEntry {
        BookEntry {
            address: NodeAddress::Ssh(host.parse().unwrap()),
            token: None,
            credential: None,
            id: None,
        }
    }

    fn name(text: &str) -> NodeName {
        text.parse().unwrap()
    }

    fn key(text: &str) -> NodeKey {
        text.parse().unwrap()
    }

    #[tokio::test]
    async fn the_pages_edits_are_the_books_and_keep_what_the_cli_wrote_meanwhile() {
        let dir = tempfile::tempdir().unwrap();
        let (source, path) = source(dir.path());

        // The CLI adds a node while the page is open; the page's next edit
        // reads the file again and keeps it.
        BookFile::open(&path)
            .unwrap()
            .add(&name("desk"), &ssh("me@desk"))
            .unwrap();
        source.rename(&name("box"), &name("big")).await.unwrap();
        let book = BookFile::open(&path).unwrap();
        let names: Vec<_> = book.book().nodes.keys().map(ToString::to_string).collect();
        assert_eq!(names, ["big", "desk", "lab"]);
        assert_eq!(book.book().default_node, Some(name("big")));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("# Mine.") && text.contains("# the big one"),
            "{text}"
        );

        source.set_default(&key("lab")).await.unwrap();
        let book = BookFile::open(&path).unwrap();
        assert_eq!(book.book().default_node, Some(name("lab")));
        source.set_default(&NodeKey::here()).await.unwrap();
        let book = BookFile::open(&path).unwrap();
        assert_eq!(book.book().default_node, None);
        let listed = source.nodes().await.unwrap();
        assert!(listed.iter().any(|node| node.key.is_here() && node.default));

        source.forget(&name("desk")).await.unwrap();
        let book = BookFile::open(&path).unwrap();
        assert!(book.book().get("desk").is_none());
        assert!(book.book().get("big").is_some());
    }

    /// `vessel nodes rename a a`, and the page saving a name it did not
    /// change: nothing to do, so it succeeds and the file is not written.
    #[tokio::test]
    async fn renaming_a_node_to_its_own_name_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (source, path) = source(dir.path());
        source.rename(&name("box"), &name("box")).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), BOOK);
    }

    #[tokio::test]
    async fn the_books_own_checks_refuse_what_it_would_not_take() {
        let dir = tempfile::tempdir().unwrap();
        let (source, path) = source(dir.path());
        assert!(matches!(
            source.forget(&name("ghost")).await,
            Err(CliError::Book(BookError::Missing { .. }))
        ));
        assert!(matches!(
            source.set_default(&key("ghost")).await,
            Err(CliError::Book(BookError::Missing { .. }))
        ));
        assert!(matches!(
            source.rename(&name("ghost"), &name("spirit")).await,
            Err(CliError::Book(BookError::Missing { .. }))
        ));
        assert!(matches!(
            source.rename(&name("box"), &name("lab")).await,
            Err(CliError::Book(BookError::Exists { .. }))
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), BOOK);
    }
}
