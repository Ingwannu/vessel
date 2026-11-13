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
        time::Duration,
    };

    use tokio::{
        io::{AsyncReadExt as _, AsyncWriteExt as _},
        net::TcpListener,
    };
    use vessel_app_updater::{
        Artifact, ByteCount, Channel, Envelope, FeedUrl, Installation, Manifest, Packaging,
        PublicKey, Published, SCHEMA, Settings, Sha256, Target, UpdateState, Updater,
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
            let path = format!("{label}/vessel-{label}-app");
            self.file(&path, NEW_BUILD);
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
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let root = root.clone();
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
                    let path = request
                        .split(' ')
                        .nth(1)
                        .unwrap_or("/")
                        .trim_start_matches('/');
                    let file = (!path.contains("..")).then(|| root.join(path));
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
}

use support::{Feed, NEW_BUILD, installed_app, serve, settings, settled};
use vessel_app_updater::{Label, OpenError, StageError, UpdateError, UpdateState, Updater};

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
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
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
    off.every = std::time::Duration::from_millis(100);

    let updater = Updater::start(
        off,
        installation.clone(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    // Several ticks pass with the preference off: none checks. A negative
    // check needs a bound; 500 ms is five ticks.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(matches!(*updater.state().borrow(), UpdateState::Idle));

    // On while running: the next tick checks, downloads and installs.
    updater.set_automatic(true);
    assert!(updater.automatic());
    let state = settled(&updater).await;
    assert!(matches!(state, UpdateState::Ready { .. }), "{state:?}");
    assert_eq!(std::fs::read(installation.executable()).unwrap(), NEW_BUILD);
}
