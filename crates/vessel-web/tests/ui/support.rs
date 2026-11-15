//! The fixtures: a running UI, a browser tab on it, real nodes in this
//! process with the fake harnesses, and a loopback proxy in front of a
//! node that a test can cut.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_util::sync::{CancellationToken, DropGuard};
use vessel_client::book::{BookEntry, BookError, BookFile, NodeName, TokenSource};
use vessel_client::{BearerToken, ConnectError, Connector, NodeAddress, NodeClient, WsConnector};
use vessel_harness::Lookup;
use vessel_harness::fakes::{Role, install};
use vessel_node::{Config, DataDir, Node};
use vessel_proto::{Call, ClientInfo, HarnessKind, NodeId, RawJson};
use vessel_testkit::Fake;
use vessel_web::wire::{
    AuthMode, BookNode, CallId, ClientFrame, NodeKey, Nonce, Op, PresentedToken, SecretText,
    ServerFrame, Transport,
};
use vessel_web::{LaunchToken, NodeSource, PageFiles, UiConfig};

/// Only turns a hang into a failure.
pub(crate) const SAFETY: Duration = Duration::from_secs(60);

/// `work` under the safety net, counted from after the fakes are built:
/// the first `Fake::path` of a process runs a nested `cargo build` that can
/// take a minute, which must not count against the deadline
/// (`vessel_testkit::within`).
pub(crate) async fn within<T>(work: impl Future<Output = T>) -> T {
    vessel_testkit::within(SAFETY, Box::pin(work))
        .await
        .unwrap()
}

/// A running UI and its token.
pub(crate) struct Ui {
    pub(crate) address: SocketAddr,
    pub(crate) token: LaunchToken,
    _stop: DropGuard,
    pub(crate) session: tokio::sync::Mutex<Option<String>>,
}

impl Ui {
    /// A UI offering `source`'s nodes, with the product's hello deadline and
    /// fast redials.
    pub(crate) async fn start(source: Source) -> Self {
        Self::configured(source, |_| {}).await
    }

    pub(crate) async fn configured(source: Source, configure: impl FnOnce(&mut UiConfig)) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let token = LaunchToken::mint().unwrap();
        // The hello deadline stays the product's: every tab here
        // authenticates, and a short deadline shared by every test made
        // each of them fail when a busy machine took longer than it to
        // answer the challenge. A test of the deadline itself sets its own.
        let mut config = UiConfig::new(token.clone());
        config.backoff = vessel_client::Backoff {
            first: Duration::from_millis(20),
            max: Duration::from_millis(100),
        };
        configure(&mut config);
        let stop = CancellationToken::new();
        drop(tokio::spawn(vessel_web::serve(
            listener,
            config,
            source,
            stop.clone(),
        )));
        Self {
            address,
            token,
            _stop: stop.drop_guard(),
            session: tokio::sync::Mutex::new(None),
        }
    }

    pub(crate) fn host(&self) -> String {
        self.address.to_string()
    }

    pub(crate) fn origin(&self) -> String {
        format!("http://{}", self.address)
    }

    /// The token as the page has it (from the URL's fragment).
    pub(crate) fn secret(&self) -> String {
        let url = self.token.url(self.address);
        url.split_once("#bootstrap=").unwrap().1.to_owned()
    }

    /// A tab: a websocket with our origin, welcomed.
    pub(crate) async fn tab(&self) -> Tab {
        let mut request = format!("ws://{}/api/ws", self.address)
            .into_client_request()
            .unwrap();
        let _none = request
            .headers_mut()
            .insert("origin", self.origin().parse().unwrap());
        let (socket, _response) = within(tokio_tungstenite::connect_async(request))
            .await
            .unwrap();
        let mut tab = Tab { socket, next_id: 0 };
        let mut session = self.session.lock().await;
        let (mode, key) = match session.as_ref() {
            Some(key) => (AuthMode::Resume, key.clone()),
            None => (AuthMode::Bootstrap, self.secret()),
        };
        let challenge = tab.next().await;
        let ServerFrame::Challenge {
            launch,
            nonce: server,
        } = challenge
        else {
            panic!("expected challenge")
        };
        let client = Nonce::try_from("a".repeat(64)).unwrap();
        let proof = sign(
            &key,
            &format!(
                "vessel-ui-v1\nclient\n{}\n{}\n{}\n",
                launch.as_str(),
                server.as_str(),
                client.as_str()
            ),
        );
        tab.send(&ClientFrame::Hello {
            mode,
            nonce: client.clone(),
            proof: PresentedToken::new(proof),
        })
        .await;
        let frame = tab.next().await;
        let ServerFrame::Welcome {
            version,
            launch: confirmed,
            proof,
            session: issued,
        } = frame
        else {
            panic!("expected welcome")
        };
        assert_eq!(launch, confirmed);
        let expected = sign(
            &key,
            &format!(
                "vessel-ui-v1\nserver\n{}\n{}\n{}\n{}\n{}",
                launch.as_str(),
                server.as_str(),
                client.as_str(),
                version,
                issued.as_ref().map_or("", PresentedToken::as_str)
            ),
        );
        assert_eq!(proof.as_str(), expected);
        if let Some(issued) = issued {
            *session = Some(issued.as_str().to_owned());
        }
        tab
    }
}

pub(crate) fn sign(key: &str, message: &str) -> String {
    use hmac::{Hmac, KeyInit as _, Mac as _};
    use std::fmt::Write as _;
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(key.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
}

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

/// A browser tab, as the page's bridge client would drive it.
pub(crate) struct Tab {
    socket: Socket,
    next_id: u64,
}

impl Tab {
    pub(crate) async fn send(&mut self, frame: &ClientFrame) {
        let text = serde_json::to_string(frame).unwrap();
        within(self.socket.send(Message::Text(text.into())))
            .await
            .unwrap();
    }

    /// Send text as a frame, whatever it holds.
    pub(crate) async fn send_text(&mut self, text: &str) {
        within(self.socket.send(Message::Text(text.into())))
            .await
            .unwrap();
    }

    /// The next frame from the bridge.
    pub(crate) async fn next(&mut self) -> ServerFrame {
        loop {
            match within(self.socket.next()).await {
                Some(Ok(Message::Text(text))) => return serde_json::from_str(&text).unwrap(),
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                other => panic!("the bridge closed: {other:?}"),
            }
        }
    }

    /// Read frames until `pick` takes one.
    pub(crate) async fn until<T>(&mut self, mut pick: impl FnMut(ServerFrame) -> Option<T>) -> T {
        loop {
            if let Some(picked) = pick(self.next().await) {
                return picked;
            }
        }
    }

    /// Send `call` to `node`; its id.
    pub(crate) async fn call(&mut self, node: &str, call: &Call) -> CallId {
        let id = self.id();
        let call = RawJson::from_serialize(call).unwrap();
        self.send(&ClientFrame::Call {
            expected_node: None,
            id,
            node: node.parse().unwrap(),
            call,
        })
        .await;
        id
    }

    /// A prepared call must keep its authenticated recipient across book changes.
    pub(crate) async fn prepared_call(
        &mut self,
        node: &str,
        expected_node: NodeId,
        call: &Call,
    ) -> CallId {
        let id = self.id();
        self.send(&ClientFrame::Call {
            id,
            node: node.parse().unwrap(),
            expected_node: Some(expected_node),
            call: RawJson::from_serialize(call).unwrap(),
        })
        .await;
        id
    }

    /// Send a call as raw text (what a broken page might send); its id.
    pub(crate) async fn call_text(&mut self, node: &str, call: &str) -> CallId {
        let id = self.id();
        self.send(&ClientFrame::Call {
            expected_node: None,
            id,
            node: node.parse().unwrap(),
            call: RawJson::from_text(call).unwrap(),
        })
        .await;
        id
    }

    /// Ask the bridge; the op's id.
    pub(crate) async fn op(&mut self, op: Op) -> CallId {
        let id = self.id();
        self.send(&ClientFrame::Op { id, op }).await;
        id
    }

    /// Send a frame written as JSON text with `{id}` standing for a fresh
    /// call id; that id.
    pub(crate) async fn frame_text(&mut self, frame: &str) -> CallId {
        let id = self.id();
        let id_text = serde_json::to_string(&id).unwrap();
        self.send_text(&frame.replace("{id}", &id_text)).await;
        id
    }

    fn id(&mut self) -> CallId {
        self.next_id = self.next_id.checked_add(1).unwrap();
        CallId::new(self.next_id)
    }
}

/// A node in this process with `fake-tui` as `claude` and `fake-acp` as
/// its ACP adapter, under a temporary home.
pub(crate) struct TestNode {
    data: TempDir,
    _bin: TempDir,
    node: Node,
}

impl TestNode {
    pub(crate) async fn start() -> Self {
        let data = tempfile::tempdir().unwrap();
        let bin = tempfile::tempdir().unwrap();
        let tui = Fake::Tui.path().unwrap();
        drop(install(bin.path(), HarnessKind::ClaudeCode, Role::Terminal, &tui).unwrap());
        let acp = Fake::Acp.path().unwrap();
        drop(install(bin.path(), HarnessKind::ClaudeCode, Role::Structured, &acp).unwrap());
        let user_home = data.path().join("user-home");
        std::fs::create_dir_all(&user_home).unwrap();
        let mut config = Config::new(
            DataDir::new(data.path()),
            vessel_harness::HarnessEnv::new(user_home),
        );
        config.bind = Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
        config.harness_lookup = Lookup::new([bin.path().to_path_buf()]);
        let node = Node::start(config).await.unwrap();
        Self {
            data,
            _bin: bin,
            node,
        }
    }

    pub(crate) fn address(&self) -> SocketAddr {
        self.node.address().unwrap()
    }

    pub(crate) fn id(&self) -> NodeId {
        self.node.info().id
    }

    pub(crate) fn dir(&self) -> &Path {
        self.data.path()
    }

    pub(crate) fn token(&self) -> BearerToken {
        let token = DataDir::new(self.data.path()).read_token().unwrap();
        token.expose().parse().unwrap()
    }
}

/// A loopback proxy in front of `upstream` whose connections can be cut
/// at once (a laptop's network dropping), while new ones still pass.
pub(crate) struct Proxy {
    pub(crate) address: SocketAddr,
    current: Arc<Mutex<CancellationToken>>,
    _stop: DropGuard,
}

impl Proxy {
    pub(crate) async fn start(upstream: SocketAddr) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let current = Arc::new(Mutex::new(CancellationToken::new()));
        let stop = CancellationToken::new();
        let accepting = Arc::clone(&current);
        let stopped = stop.clone();
        drop(tokio::spawn(async move {
            loop {
                let accepted = tokio::select! {
                    accepted = listener.accept() => accepted,
                    () = stopped.cancelled() => return,
                };
                let Ok((inbound, _from)) = accepted else {
                    return;
                };
                let cut = accepting.lock().unwrap().clone();
                drop(tokio::spawn(pipe(inbound, upstream, cut)));
            }
        }));
        Self {
            address,
            current,
            _stop: stop.drop_guard(),
        }
    }

    /// Close every connection through the proxy now.
    pub(crate) fn cut(&self) {
        let mut current = self.current.lock().unwrap();
        current.cancel();
        *current = CancellationToken::new();
    }
}

async fn pipe(mut inbound: TcpStream, upstream: SocketAddr, cut: CancellationToken) {
    let Ok(mut outbound) = TcpStream::connect(upstream).await else {
        return;
    };
    tokio::select! {
        _copied = tokio::io::copy_bidirectional(&mut inbound, &mut outbound) => {}
        () = cut.cancelled() => {}
    }
}

/// The nodes the tests' UI offers: a real node book (`BookFile`, the
/// CLI's own writer) in a temporary directory, with each websocket node's
/// token in a file beside it, and every dial counted. Its edits are the
/// book's, so the bridge's tests see what the CLI's source does; what the
/// CLI adds on top (this machine's node, SSH, pairing) is tested in
/// `vessel-cli`.
#[derive(Clone)]
pub(crate) struct Source {
    dir: Arc<TempDir>,
    dials: Arc<AtomicUsize>,
    files: Files,
}

/// Where the tests' source says the page's preferences and drafts go.
#[derive(Clone)]
enum Files {
    /// Nowhere: the trait's default (memory, for this launch).
    Default,
    /// In these files.
    At {
        preferences: PathBuf,
        drafts: PathBuf,
    },
    /// It cannot say (as a CLI without a home directory).
    Unknown,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            dir: Arc::new(tempfile::tempdir().unwrap()),
            dials: Arc::new(AtomicUsize::new(0)),
            files: Files::Default,
        }
    }
}

impl Source {
    /// Keep the page's files in `dir` (`config/vessel/ui-preferences.json`
    /// and `data/ui-drafts.json`, neither directory made yet).
    pub(crate) fn keeping_in(mut self, dir: &Path) -> Self {
        self.files = Files::At {
            preferences: Self::preferences_in(dir),
            drafts: Self::drafts_in(dir),
        };
        self
    }

    /// Answer the page's files with an error.
    pub(crate) fn not_knowing_where(mut self) -> Self {
        self.files = Files::Unknown;
        self
    }

    /// Where [`Source::keeping_in`] keeps the preferences.
    pub(crate) fn preferences_in(dir: &Path) -> PathBuf {
        dir.join("config")
            .join("vessel")
            .join("ui-preferences.json")
    }

    /// Where [`Source::keeping_in`] keeps the drafts.
    pub(crate) fn drafts_in(dir: &Path) -> PathBuf {
        dir.join("data").join("ui-drafts.json")
    }

    /// Offer the node at `address` as `name`, with `token` in a file.
    pub(crate) fn with(self, name: &str, address: SocketAddr, token: &BearerToken) -> Self {
        let tokens = self.dir.path().join("tokens");
        std::fs::create_dir_all(&tokens).unwrap();
        let file = tokens.join(name);
        std::fs::write(&file, token.expose()).unwrap();
        let entry = BookEntry {
            address: NodeAddress::Ws(address.to_string().parse().unwrap()),
            token: Some(TokenSource::File(file)),
            credential: None,
            id: None,
        };
        self.file().add(&name.parse().unwrap(), &entry).unwrap();
        self
    }

    /// Empty the book, as `vessel nodes remove` for every node would.
    pub(crate) fn remove_all(&self) {
        std::fs::write(self.path(), "").unwrap();
    }

    /// Counts the dials made through this source.
    pub(crate) fn dials(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.dials)
    }

    /// The book's file.
    pub(crate) fn path(&self) -> PathBuf {
        self.dir.path().join("nodes.toml")
    }

    fn file(&self) -> BookFile {
        BookFile::open(&self.path()).unwrap()
    }
}

/// What the tests' source refuses: the book's own errors, and what it has
/// no way to reach.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Refused {
    #[error(transparent)]
    Book(#[from] BookError),
    #[error("no node named {0}")]
    Missing(NodeKey),
    #[error("the tests' book holds websocket nodes with token files only")]
    NotWebsocket,
    #[error("no home directory (the tests' source)")]
    NoHome,
}

impl NodeSource for Source {
    type Connector = Counted;
    type Error = Refused;

    async fn nodes(&self) -> Result<Vec<BookNode>, Refused> {
        let file = BookFile::open(&self.path())?;
        let book = file.book();
        Ok(book
            .nodes
            .iter()
            .map(|(name, entry)| BookNode {
                key: NodeKey::from(name.clone()),
                label: name.to_string(),
                address: entry.address.to_string(),
                transport: Transport::Ws,
                default: book.default_node.as_ref() == Some(name),
                id: None,
            })
            .collect())
    }

    async fn connector(&self, node: &NodeKey) -> Result<Counted, Refused> {
        let file = BookFile::open(&self.path())?;
        let entry = match node {
            NodeKey::Here => None,
            NodeKey::Book(name) => file.book().nodes.get(name),
        }
        .ok_or_else(|| Refused::Missing(node.clone()))?;
        let (NodeAddress::Ws(url), Some(token)) = (&entry.address, &entry.token) else {
            return Err(Refused::NotWebsocket);
        };
        let token = std::fs::read_to_string(token.path(None))
            .unwrap()
            .parse()
            .unwrap();
        Ok(Counted {
            inner: WsConnector::new(url.clone(), token),
            dials: Arc::clone(&self.dials),
        })
    }

    async fn seen(&self, _node: &NodeKey, _id: NodeId) {}

    async fn pair(&self, name: &NodeName, _link: &SecretText) -> Result<BookNode, Refused> {
        Err(Refused::Missing(NodeKey::from(name.clone())))
    }

    async fn forget(&self, node: &NodeName) -> Result<(), Refused> {
        Ok(self.file().remove(node.as_str())?)
    }

    async fn rename(&self, node: &NodeName, to: &NodeName) -> Result<(), Refused> {
        Ok(self.file().rename(node.as_str(), to)?)
    }

    async fn set_default(&self, node: &NodeKey) -> Result<(), Refused> {
        let name = match node {
            NodeKey::Here => None,
            NodeKey::Book(name) => Some(name.as_str()),
        };
        Ok(self.file().set_default(name)?)
    }

    fn preferences(&self) -> Result<PageFiles, Refused> {
        match &self.files {
            Files::Default => Ok(PageFiles::default()),
            Files::At {
                preferences,
                drafts,
            } => Ok(PageFiles {
                preferences: Some(preferences.clone()),
                drafts: Some(drafts.clone()),
            }),
            Files::Unknown => Err(Refused::NoHome),
        }
    }
}

/// A websocket connector that counts its dials.
pub(crate) struct Counted {
    inner: WsConnector,
    dials: Arc<AtomicUsize>,
}

impl Connector for Counted {
    type Error = ConnectError;

    async fn connect(&self, client: ClientInfo) -> Result<NodeClient, ConnectError> {
        let _before = self.dials.fetch_add(1, Ordering::SeqCst);
        self.inner.connect(client).await
    }

    fn is_transient(&self, error: &ConnectError) -> bool {
        self.inner.is_transient(error)
    }
}
