//! The updater end to end: a feed served over HTTP on this machine, a
//! copy of the "app" in a temporary directory, and the task run as the
//! app runs it. Each test publishes a feed the way `vessel-app-feed` does
//! (an envelope signed with a key made for the test) and checks what the
//! copy on disk is afterwards.

// Helpers live in a `cfg(test)` module, as in the main workspace's
// integration tests, so clippy's test allowances apply to them.
#[cfg(test)]
mod support {
    use std::{
        net::SocketAddr,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        time::Duration,
    };

    use tokio::{
        io::{AsyncReadExt as _, AsyncWriteExt as _},
        net::TcpListener,
    };
    use vessel_app_updater::{
        Artifact, ByteCount, Channel, Companion, Envelope, FeedUrl, Installation, Manifest,
        Packaging, PublicKey, Published, Role, SCHEMA, Settings, Sha256, Target, UpdateState,
        Updater,
    };

    /// The bytes of the "new build".
    pub(crate) const NEW_BUILD: &[u8] = b"the new build of the app";

    /// A feed on disk and the key that signs it.
    pub(crate) struct Feed {
        pub(crate) root: tempfile::TempDir,
        pair: minisign::KeyPair,
    }

    impl Feed {
        pub(crate) fn new() -> Self {
            Self {
                root: tempfile::tempdir().unwrap(),
                pair: minisign::KeyPair::generate_unencrypted_keypair().unwrap(),
            }
        }

        pub(crate) fn key(&self) -> PublicKey {
            PublicKey::from_base64(&self.pair.pk.to_base64()).unwrap()
        }

        /// Put `bytes` at `path` under the feed.
        pub(crate) fn file(&self, path: &str, bytes: &[u8]) {
            let full = self.root.path().join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, bytes).unwrap();
        }

        /// Publish `label` on `channel` with one artifact for this machine,
        /// described as `described` (its real bytes are `NEW_BUILD`).
        pub(crate) fn publish(&self, channel: &str, label: &str, described: &[u8]) {
            self.publish_set(channel, label, described, &[]);
        }

        /// [`Feed::publish`] with companions (ADR 0094): each is served as
        /// `served` and described as `described`.
        pub(crate) fn publish_set(
            &self,
            channel: &str,
            label: &str,
            described: &[u8],
            companions: &[(Role, &[u8], &[u8])],
        ) {
            let path = format!("{label}/vessel-{label}-app");
            self.file(&path, NEW_BUILD);
            let companions = companions
                .iter()
                .map(|(role, served, described)| {
                    let path = format!("{label}/vessel-{label}-{role}");
                    self.file(&path, served);
                    Companion {
                        role: role.clone(),
                        path: path.parse().unwrap(),
                        size: ByteCount::try_from(u64::try_from(described.len()).unwrap()).unwrap(),
                        sha256: Sha256::of(described),
                    }
                })
                .collect();
            let manifest = Manifest {
                schema: SCHEMA,
                channel: channel.parse().unwrap(),
                label: label.parse().unwrap(),
                published: Published::try_from("2026-11-13T00:00:00Z".to_owned()).unwrap(),
                notes: None,
                artifacts: vec![Artifact {
                    target: Target::current().unwrap(),
                    packaging: Packaging::Executable,
                    path: path.parse().unwrap(),
                    size: ByteCount::try_from(u64::try_from(described.len()).unwrap()).unwrap(),
                    sha256: Sha256::of(described),
                    companions,
                }],
            };
            let text = serde_json::to_string(&manifest).unwrap();
            let signature = minisign::sign(
                Some(&self.pair.pk),
                &self.pair.sk,
                text.as_bytes(),
                None,
                None,
            )
            .unwrap()
            .into_string();
            let envelope = Envelope {
                manifest: text,
                signature,
            };
            self.file(
                &format!("{channel}.json"),
                &serde_json::to_vec(&envelope).unwrap(),
            );
        }
    }

    /// Serve `root` over HTTP/1.1 on a loopback port: `GET /<path>` answers
    /// the file or 404. Enough of HTTP for reqwest, nothing more.
    pub(crate) async fn serve(root: PathBuf) -> SocketAddr {
        serve_under(root, "", Seen::default()).await
    }

    /// The paths a server was asked for, in order.
    pub(crate) type Seen = Arc<Mutex<Vec<String>>>;

    /// [`serve`] with the feed under `/<prefix>/` (the real feed's path
    /// carries a token), noting each path asked for in `seen`.
    pub(crate) async fn serve_under(root: PathBuf, prefix: &str, seen: Seen) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!("{prefix}/")
        };
        drop(tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let root = root.clone();
                let prefix = prefix.clone();
                let seen = Arc::clone(&seen);
                drop(tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 1024];
                    while !request.windows(4).any(|end| end == b"\r\n\r\n") && request.len() < 8192
                    {
                        let read = stream.read(&mut buffer).await.unwrap();
                        if read == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..read]);
                    }
                    let request = String::from_utf8_lossy(&request);
                    let asked = request
                        .split(' ')
                        .nth(1)
                        .unwrap_or("/")
                        .trim_start_matches('/');
                    seen.lock().unwrap().push(asked.to_owned());
                    let file = asked
                        .strip_prefix(prefix.as_str())
                        .filter(|path| !path.contains(".."))
                        .map(|path| root.join(path));
                    let reply = match file.map(std::fs::read) {
                        Some(Ok(body)) => {
                            let mut reply = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            )
                            .into_bytes();
                            reply.extend_from_slice(&body);
                            reply
                        }
                        _ => {
                            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                                .to_vec()
                        }
                    };
                    let _written = stream.write_all(&reply).await.is_ok();
                }));
            }
        }));
        address
    }

    /// The file a companion of `role` is installed as on this machine.
    pub(crate) fn companion_file(dir: &Path, role: &Role) -> PathBuf {
        dir.join(role.file_name(Target::current().unwrap()).unwrap())
    }

    /// The names in `dir`, sorted.
    pub(crate) fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// A copy of the "app" of build `running` in a temporary directory.
    pub(crate) fn installed_app(dir: &Path) -> Installation {
        let app = dir.join("vessel-app");
        std::fs::write(&app, b"the running build").unwrap();
        Installation::detect(Packaging::Executable, app, None).unwrap()
    }

    pub(crate) fn settings(address: SocketAddr, key: PublicKey, running: &str) -> Settings {
        Settings {
            feed: FeedUrl::parse(&format!("http://{address}/")).unwrap(),
            key,
            channel: Channel::preview(),
            running: running.parse().unwrap(),
            target: Target::current().unwrap(),
            every: Duration::from_secs(3600),
            first_after: Duration::ZERO,
            automatic: true,
            retry_after: vec![Duration::from_secs(3600)],
        }
    }

    /// Wait until the updater leaves the states it passes through.
    pub(crate) async fn settled(updater: &Updater) -> UpdateState {
        let mut state = updater.state();
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let now = state.borrow_and_update().clone();
                match now {
                    UpdateState::Idle | UpdateState::Checking | UpdateState::Downloading { .. } => {
                    }
                    UpdateState::UpToDate { .. }
                    | UpdateState::Ready { .. }
                    | UpdateState::Failed { .. } => return now,
                }
                if state.changed().await.is_err() {
                    return state.borrow().clone();
                }
            }
        })
        .await
        .expect("the updater did not settle within 30 s")
    }

    /// What the updater logs, captured for the whole test binary (a global
    /// subscriber: the updater logs from tokio's worker and blocking threads).
    pub(crate) fn captured_log() -> &'static Arc<Mutex<Vec<u8>>> {
        static LOG: std::sync::OnceLock<Arc<Mutex<Vec<u8>>>> = std::sync::OnceLock::new();
        LOG.get_or_init(|| {
            let log = Arc::new(Mutex::new(Vec::new()));
            let writer = Arc::clone(&log);
            let subscriber = tracing_subscriber::fmt()
                .with_ansi(false)
                // The app's file log: everything at info, the updater at debug;
                // trace here to be stricter than the app.
                .with_env_filter(tracing_subscriber::EnvFilter::new(
                    "info,vessel_app_updater=trace",
                ))
                .with_writer(move || Capture(Arc::clone(&writer)))
                .finish();
            tracing::subscriber::set_global_default(subscriber).unwrap();
            log
        })
    }

    /// A `Write` into the captured log.
    pub(crate) struct Capture(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .map_err(|_poisoned| std::io::Error::other("the captured log is poisoned"))?
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}

use std::{sync::Arc, time::Duration};
use support::{
    Feed, NEW_BUILD, Seen, captured_log, companion_file, installed_app, names, serve, serve_under,
    settings, settled,
};

use vessel_app_updater::{
    FeedUrl, Label, OpenError, Role, StageError, UpdateError, UpdateState, Updater,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_newer_build_is_downloaded_verified_and_put_in_place() {
    let feed = Feed::new();
    feed.publish("preview", "0.0.0.0.2", NEW_BUILD);
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    let installation = installed_app(app_dir.path());
    std::fs::write(
        app_dir.path().join(".vessel-update-0.0.0.0.1.9.part"),
        b"left",
    )
    .unwrap();

    let updater = Updater::start(
        settings(address, feed.key(), "0.0.0.0.1"),
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let state = settled(&updater).await;

    let UpdateState::Ready { label, .. } = state else {
        panic!("not ready: {state:?}");
    };
    assert_eq!(label, "0.0.0.0.2".parse::<Label>().unwrap());
    assert_eq!(std::fs::read(installation.executable()).unwrap(), NEW_BUILD);
    let mut names: Vec<_> = std::fs::read_dir(app_dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    // Windows cannot remove the running executable, only rename it: the
    // old build stays aside until the next start's clean-up.
    let expected: &[&str] = if cfg!(windows) {
        &["vessel-app", "vessel-app.old-0.0.0.0.1"]
    } else {
        &["vessel-app"]
    };
    assert_eq!(names, expected, "leftovers or staging files remain");
    // Installed, the task stops: no second update over the first.
    assert!(!updater.check_now() || matches!(*updater.state().borrow(), UpdateState::Ready { .. }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_or_an_older_build_is_not_an_update() {
    let feed = Feed::new();
    feed.publish("preview", "0.0.0.0.1", NEW_BUILD);
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    let installation = installed_app(app_dir.path());

    let updater = Updater::start(
        settings(address, feed.key(), "0.0.0.0.1"),
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();

    let state = settled(&updater).await;
    assert!(matches!(state, UpdateState::UpToDate { .. }), "{state:?}");
    assert_eq!(
        std::fs::read(installation.executable()).unwrap(),
        b"the running build"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bytes_other_than_the_signed_ones_are_never_installed() {
    let feed = Feed::new();
    // The manifest describes other bytes than the file the host serves:
    // a replaced artifact.
    feed.publish("preview", "0.0.0.0.2", b"the build the key signed for!");
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    let installation = installed_app(app_dir.path());

    let updater = Updater::start(
        settings(address, feed.key(), "0.0.0.0.1"),
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();

    let state = settled(&updater).await;
    let UpdateState::Failed { error } = state else {
        panic!("not refused: {state:?}");
    };
    assert!(
        matches!(
            &*error,
            UpdateError::Stage(StageError::Short { .. } | StageError::Digest)
        ),
        "{error:?}"
    );
    assert_eq!(
        std::fs::read(installation.executable()).unwrap(),
        b"the running build"
    );
    assert_eq!(
        std::fs::read_dir(app_dir.path()).unwrap().count(),
        1,
        "the download was left"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_feed_signed_by_another_key_is_refused() {
    let feed = Feed::new();
    feed.publish("preview", "0.0.0.0.2", NEW_BUILD);
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    let installation = installed_app(app_dir.path());
    let other_key = Feed::new().key();

    let updater = Updater::start(
        settings(address, other_key, "0.0.0.0.1"),
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();

    let state = settled(&updater).await;
    let UpdateState::Failed { error } = state else {
        panic!("not refused: {state:?}");
    };
    assert!(
        matches!(&*error, UpdateError::Open(OpenError::Signature(_))),
        "{error:?}"
    );
    assert_eq!(
        std::fs::read(installation.executable()).unwrap(),
        b"the running build"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_feed_is_a_failure_to_retry() {
    let feed = Feed::new();
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();

    let updater = Updater::start(
        settings(address, feed.key(), "0.0.0.0.1"),
        installed_app(app_dir.path()),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();

    let state = settled(&updater).await;
    let UpdateState::Failed { error } = state else {
        panic!("not a failure: {state:?}");
    };
    assert!(matches!(&*error, UpdateError::Fetch(_)), "{error:?}");
    // The task still runs and takes a request to check again.
    assert!(updater.check_now());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn with_automatic_updates_off_only_a_check_asked_for_runs() {
    let feed = Feed::new();
    feed.publish("preview", "0.0.0.0.2", NEW_BUILD);
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    let installation = installed_app(app_dir.path());
    let mut off = settings(address, feed.key(), "0.0.0.0.1");
    off.automatic = false;

    let updater = Updater::start(
        off,
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    // The timer's first tick (due at once) has had time to fire; with
    // automatic updates off it must not have checked. A negative check
    // needs a bound; 300 ms is far past the tick on any machine.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(matches!(*updater.state().borrow(), UpdateState::Idle));
    assert!(!updater.automatic());

    assert!(updater.check_now());
    let state = settled(&updater).await;
    assert!(matches!(state, UpdateState::Ready { .. }), "{state:?}");
    assert_eq!(std::fs::read(installation.executable()).unwrap(), NEW_BUILD);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turning_automatic_updates_on_changes_a_running_updater() {
    let feed = Feed::new();
    feed.publish("preview", "0.0.0.0.2", NEW_BUILD);
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    let installation = installed_app(app_dir.path());
    let mut off = settings(address, feed.key(), "0.0.0.0.1");
    off.automatic = false;
    off.every = Duration::from_millis(100);

    let updater = Updater::start(
        off,
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    // Several ticks pass with the preference off: none checks. A negative
    // check needs a bound; 500 ms is five ticks.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(matches!(*updater.state().borrow(), UpdateState::Idle));

    // On while running: the next tick checks, downloads and installs.
    updater.set_automatic(true);
    assert!(updater.automatic());
    let state = settled(&updater).await;
    assert!(matches!(state, UpdateState::Ready { .. }), "{state:?}");
    assert_eq!(std::fs::read(installation.executable()).unwrap(), NEW_BUILD);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_check_is_retried_on_the_schedule_and_then_hourly() {
    let feed = Feed::new();
    let seen = Seen::default();
    let address = serve_under(feed.root.path().to_path_buf(), "", Arc::clone(&seen)).await;
    let app_dir = tempfile::tempdir().unwrap();
    // The schedule scaled down: three quick retries, then the hour.
    let mut quick = settings(address, feed.key(), "0.0.0.0.1");
    quick.retry_after = vec![Duration::from_millis(100); 3];

    let updater = Updater::start(
        quick,
        installed_app(app_dir.path()),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    // No manifest: every check fails. The first and three retries come
    // within a second; the fifth waits for the hour.
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let asked = seen.lock().unwrap().len();
    assert_eq!(asked, 4, "{:?}", seen.lock().unwrap());
    assert!(matches!(
        *updater.state().borrow(),
        UpdateState::Failed { .. }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_retry_installs_a_build_the_failed_check_could_not_reach() {
    let feed = Feed::new();
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    let installation = installed_app(app_dir.path());
    let mut quick = settings(address, feed.key(), "0.0.0.0.1");
    quick.retry_after = vec![Duration::from_millis(300)];

    let updater = Updater::start(
        quick,
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    assert!(matches!(
        settled(&updater).await,
        UpdateState::Failed { .. }
    ));
    // The feed comes back before the retry; nobody asks for a check.
    feed.publish("preview", "0.0.0.0.2", NEW_BUILD);
    let mut state = updater.state();
    tokio::time::timeout(Duration::from_secs(10), async {
        while !matches!(*state.borrow_and_update(), UpdateState::Ready { .. }) {
            state.changed().await.unwrap();
        }
    })
    .await
    .expect("the retry did not install the build");
    assert_eq!(std::fs::read(installation.executable()).unwrap(), NEW_BUILD);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_log_never_names_the_feeds_path() {
    // The real feed's address carries a token in its path; the owner sends
    // the log file, so it must never hold the token, whatever happens.
    const TOKEN: &str = "feed-token-7d1c9a4e";
    let log = captured_log();

    // An update that works, under the token.
    let feed = Feed::new();
    feed.publish("preview", "0.0.0.0.2", NEW_BUILD);
    let address = serve_under(feed.root.path().to_path_buf(), TOKEN, Seen::default()).await;
    let mut under_token = settings(address, feed.key(), "0.0.0.0.1");
    under_token.feed = FeedUrl::parse(&format!("http://{address}/{TOKEN}/")).unwrap();
    assert!(!format!("{under_token:?}").contains(TOKEN));
    assert!(!under_token.feed.to_string().contains(TOKEN));
    let app_dir = tempfile::tempdir().unwrap();
    let updater = Updater::start(
        under_token.clone(),
        installed_app(app_dir.path()),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    assert!(matches!(settled(&updater).await, UpdateState::Ready { .. }));

    // A download the feed does not have (404 after a good manifest).
    let missing = Feed::new();
    missing.publish("preview", "0.0.0.0.2", NEW_BUILD);
    std::fs::remove_file(missing.root.path().join("0.0.0.0.2/vessel-0.0.0.0.2-app")).unwrap();
    let address = serve_under(missing.root.path().to_path_buf(), TOKEN, Seen::default()).await;
    let mut gone = settings(address, missing.key(), "0.0.0.0.1");
    gone.feed = FeedUrl::parse(&format!("http://{address}/{TOKEN}/")).unwrap();
    let app_dir = tempfile::tempdir().unwrap();
    let updater = Updater::start(
        gone,
        installed_app(app_dir.path()),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    assert!(matches!(
        settled(&updater).await,
        UpdateState::Failed { .. }
    ));

    // No server at all: reqwest's own error names the address it tried.
    let nobody = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let closed = nobody.local_addr().unwrap();
    drop(nobody);
    let mut unreachable = settings(closed, feed.key(), "0.0.0.0.1");
    unreachable.feed = FeedUrl::parse(&format!("http://{closed}/{TOKEN}/")).unwrap();
    let app_dir = tempfile::tempdir().unwrap();
    let updater = Updater::start(
        unreachable,
        installed_app(app_dir.path()),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    assert!(matches!(
        settled(&updater).await,
        UpdateState::Failed { .. }
    ));

    let text = String::from_utf8_lossy(&log.lock().unwrap()).into_owned();
    assert!(
        !text.contains(TOKEN),
        "the log names the feed's path:\n{text}"
    );
    // The records are there, with the causes under the short errors.
    for step in [
        "the updater runs",
        "downloading the update",
        "the feed answers the download",
        "the new build is in place",
        "the feed answered 404",
        "the feed could not be reached: ",
    ] {
        assert!(text.contains(step), "no {step:?} in the log:\n{text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_newer_set_replaces_the_app_and_its_companions_together() {
    let feed = Feed::new();
    feed.publish_set(
        "preview",
        "0.0.0.0.20",
        NEW_BUILD,
        &[
            (Role::Cli, b"new cli", b"new cli"),
            (Role::Node, b"new node", b"new node"),
            ("helper".parse().unwrap(), b"later", b"later"),
        ],
    );
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    let installation = installed_app(app_dir.path());
    // The old command line is there; the node is not (an older install).
    std::fs::write(companion_file(app_dir.path(), &Role::Cli), b"old cli").unwrap();

    let updater = Updater::start(
        settings(address, feed.key(), "0.0.0.0.19"),
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let state = settled(&updater).await;

    assert!(matches!(state, UpdateState::Ready { .. }), "{state:?}");
    assert_eq!(std::fs::read(installation.executable()).unwrap(), NEW_BUILD);
    assert_eq!(
        std::fs::read(companion_file(app_dir.path(), &Role::Cli)).unwrap(),
        b"new cli"
    );
    assert_eq!(
        std::fs::read(companion_file(app_dir.path(), &Role::Node)).unwrap(),
        b"new node"
    );
    if cfg!(unix) {
        assert_eq!(
            names(app_dir.path()),
            ["vessel", "vessel-app", "vesseld"],
            "leftovers, staging files, or the unknown role's file remain"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_companion_that_does_not_verify_leaves_the_whole_set_as_it_was() {
    let feed = Feed::new();
    // The node the host serves is not the one the key signed for.
    feed.publish_set(
        "preview",
        "0.0.0.0.20",
        NEW_BUILD,
        &[
            (Role::Cli, b"new cli", b"new cli"),
            (Role::Node, b"replaced!", b"new node!"),
        ],
    );
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    let installation = installed_app(app_dir.path());
    std::fs::write(companion_file(app_dir.path(), &Role::Cli), b"old cli").unwrap();

    let updater = Updater::start(
        settings(address, feed.key(), "0.0.0.0.19"),
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let state = settled(&updater).await;

    let UpdateState::Failed { error } = state else {
        panic!("not failed: {state:?}");
    };
    assert!(
        matches!(*error, UpdateError::Stage(StageError::Digest)),
        "{error:?}"
    );
    assert_eq!(
        std::fs::read(installation.executable()).unwrap(),
        b"the running build"
    );
    assert_eq!(
        std::fs::read(companion_file(app_dir.path(), &Role::Cli)).unwrap(),
        b"old cli"
    );
    assert_eq!(
        names(app_dir.path()),
        if cfg!(windows) {
            ["vessel-app", "vessel-cli.exe"]
        } else {
            ["vessel", "vessel-app"]
        },
        "a staged file or a half set remains"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_running_builds_missing_companions_are_filled_in() {
    let feed = Feed::new();
    feed.publish_set(
        "preview",
        "0.0.0.0.20",
        NEW_BUILD,
        &[(Role::Cli, b"cli", b"cli"), (Role::Node, b"node", b"node")],
    );
    let address = serve(feed.root.path().to_path_buf()).await;
    let app_dir = tempfile::tempdir().unwrap();
    // 0.0.0.0.20 installed by an app from before the set: its executable
    // only. The command line is already there and right; the node is not.
    let installation = installed_app(app_dir.path());
    std::fs::write(companion_file(app_dir.path(), &Role::Cli), b"cli").unwrap();

    let updater = Updater::start(
        settings(address, feed.key(), "0.0.0.0.20"),
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let state = settled(&updater).await;

    assert!(matches!(state, UpdateState::UpToDate { .. }), "{state:?}");
    assert_eq!(
        std::fs::read(installation.executable()).unwrap(),
        b"the running build",
        "the running app itself is not replaced"
    );
    assert_eq!(
        std::fs::read(companion_file(app_dir.path(), &Role::Node)).unwrap(),
        b"node"
    );
    assert_eq!(
        std::fs::read(companion_file(app_dir.path(), &Role::Cli)).unwrap(),
        b"cli"
    );
    // A repair does not stop the task: a later build still updates.
    assert!(updater.check_now());
}
