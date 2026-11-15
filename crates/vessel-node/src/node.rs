//! Starting a node and stopping it.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::parking::ParkAfter;
use vessel_proto::ports::PortOwner;
use vessel_proto::{NodeId, NodeInfo, RelayToken, RouteUrl};
use vessel_relay::address::ClientAddress;
use vessel_relay::{LinkState, TlsConfig};
use vessel_secure::NodeKeypair;
use vessel_session::{AttachmentLoader, RuntimeConfig, SessionRuntime};
use vessel_store::{DataDir, NodeLock, Opened, Store};

use crate::agents::Agents;
use crate::automation;
use crate::automation::schedules::node_runner::NodeRunner;
use crate::automation::schedules::{Schedules, SystemClock};
use crate::config::Config;
use crate::devices::{Devices, Routes};
use crate::error::NodeError;
use crate::fleet::{self, FleetFiles};
use crate::forge::Forges;
use crate::guard::Guarded;
use crate::identity;
use crate::ipc::{self, AgentGateway, Endpoint, LocalGateway, LocalListener};
use crate::lifecycle::Lifecycle;
use crate::models::proxy::serve as serve_models;
use crate::probes::Probes;
use crate::relays::{self, Relays};
use crate::secure::SecureGateway;
use crate::server::{self, Gateway};
use crate::service::{NodeService, Parts};
use crate::sessions::Sessions;
use crate::shutdown::Teardown;
use crate::terminals::{Ports, Runner, Terminals};
use crate::tidying::Tidying;
use crate::transfers::{self, Transfers};
use crate::workspace::{Opening, Workspaces};

mod attachment_setup;
mod frontends;
mod prepare;
mod setup;

use self::attachment_setup::{ATTACHMENT_LOADS, attachments};
use self::frontends::Frontends;
use self::prepare::{Prepared, prepare_off_thread};
use self::setup::{SessionSetup, model_proxy};

/// A running node. Stop it with [`Node::stop`]; dropping it leaves it
/// running until the process exits.
#[derive(Debug)]
pub struct Node {
    address: Option<SocketAddr>,
    local: Endpoint,
    lifecycle: Lifecycle,
    /// The gateway `/secure` and the relay links serve; tests reach it
    /// directly with in-memory carriers.
    #[cfg(test)]
    pub(crate) secure: SecureGateway,
    /// Its relay links.
    relays: Arc<Relays>,
    stop: CancellationToken,
    task: JoinHandle<Result<(), NodeError>>,
    provider_registrations: crate::ProviderRegistrations,
}

impl Node {
    /// Prepare the data directory, take its node lock, open the event log
    /// (closing sessions a previous run left live as `Interrupted`), bind
    /// the local socket and the listen address (if any), and start
    /// serving. Returns once the node accepts connections.
    ///
    /// # Errors
    /// The data directory or event log is unusable, another node holds the
    /// data directory, or the local socket or the address cannot be bound.
    pub async fn start(config: Config) -> Result<Self, NodeError> {
        // Checked before anything starts, so a misconfigured relay or
        // proxy is an error at once rather than a node that is unreachable
        // (or limited as one client) later.
        let clients = client_address(&config)?;
        let relaying = relaying(&config)?;
        let Prepared {
            id,
            token,
            keypair,
            lock,
            profiles,
            models,
            dirs,
            providers,
        } = prepare_off_thread(&config).await?;
        let store = open_store(&config).await?;
        let (models, proxy) = model_proxy(models).await?;
        let session_setup = SessionSetup::new(&config, id, profiles, Arc::clone(&models), dirs);

        let core = core(
            &config,
            id,
            &store,
            &keypair,
            relaying,
            session_setup,
            providers,
        )
        .await?;
        drop(tokio::spawn(serve_models(models, proxy, core.stop.clone())));
        let Frontends {
            rpc,
            secure,
            teardown,
            connections,
            agents,
            relays,
            lifecycle,
            stop,
            provider_registrations,
        } = frontends::prepare(&config, core, keypair, store);
        let token = Arc::new(token);

        let (local, tcp) = bind(&config, id).await?;
        let local_endpoint = local.endpoint().clone();
        let address = tcp.as_ref().map(|(_, address)| *address);

        let router = tcp.map(|(listener, _)| {
            (
                listener,
                server::router(Gateway {
                    rpc: Arc::clone(&rpc),
                    token: Arc::clone(&token),
                    secure: secure.clone(),
                    keepalive: config.keepalive,
                    connections: connections.clone(),
                    clients,
                }),
            )
        });
        relays.serve(secure.clone(), connections.clone());
        let gateway = LocalGateway {
            rpc,
            token,
            connections,
            agents: AgentGateway::new(agents, &secure, config.rpc.clone()),
        };
        let task = tokio::spawn(serve(
            router,
            (local, gateway),
            stop.clone(),
            teardown,
            lock,
        ));
        listening(id, address, &local_endpoint);
        Ok(Self {
            address,
            local: local_endpoint,
            lifecycle,
            #[cfg(test)]
            secure,
            relays,
            stop,
            task,
            provider_registrations,
        })
    }

    /// The TCP address the node listens on (with the real port when the
    /// config asked for port 0); `None` when it listens on its local socket
    /// only.
    #[must_use]
    pub const fn address(&self) -> Option<SocketAddr> {
        self.address
    }

    /// The node's local socket (Unix) or named pipe (Windows), which
    /// `vesseld attach` connects to.
    #[must_use]
    pub fn local_endpoint(&self) -> &Path {
        self.local.path()
    }

    /// Each configured relay and where the node's link to it stands
    /// (connecting, registered, refused), to read or wait on.
    #[must_use]
    pub fn relay_links(&self) -> Vec<(RouteUrl, watch::Receiver<LinkState>)> {
        self.relays.states()
    }

    /// The node's current description.
    #[must_use]
    pub fn info(&self) -> NodeInfo {
        self.lifecycle.info()
    }

    /// Trusted host-side plugin supervision can publish inert provider
    /// contributions. This handle grants no plugin execution or RPC authority.
    #[must_use]
    pub fn provider_registrations(&self) -> crate::ProviderRegistrations {
        self.provider_registrations.clone()
    }

    /// Resolves once the node was asked to stop, by [`Node::stop`] or by a
    /// client (`node.stop`, protocol 3). The caller then calls
    /// [`Node::stop`] to wait for the shutdown to finish.
    pub fn stop_requested(&self) -> impl Future<Output = ()> + Send + 'static {
        self.stop.clone().cancelled_owned()
    }

    /// Stop accepting, end every session as `Interrupted`, commit their
    /// logs, close connections and the store, and release the data
    /// directory; returns when all of that is done.
    ///
    /// # Errors
    /// The HTTP server had failed.
    pub async fn stop(self) -> Result<(), NodeError> {
        self.stop.cancel();
        self.task.await.map_err(NodeError::Task)?
    }
}

/// Say where the node listens.
fn listening(id: NodeId, address: Option<SocketAddr>, local: &Endpoint) {
    tracing::info!(
        node = %id,
        address = ?address,
        local = %local.path().display(),
        "node listening"
    );
}

/// Open the event log, closing sessions a previous run left live.
async fn open_store(config: &Config) -> Result<Store, NodeError> {
    let Opened { store, interrupted } = Store::open(&config.data_dir, config.store)
        .await
        .map_err(NodeError::Store)?;
    if !interrupted.is_empty() {
        tracing::info!(
            sessions = interrupted.len(),
            "sessions from the previous run are now interrupted"
        );
    }
    Ok(store)
}

/// The node's parts below its transports.
struct Core {
    runtime: Arc<SessionRuntime>,
    lifecycle: Lifecycle,
    /// Cancelled to stop the node.
    stop: CancellationToken,
    /// Session creations and persist tasks.
    tasks: TaskTracker,
    /// Subscription forwarders.
    forwarders: TaskTracker,
    devices: Arc<Devices>,
    /// Sessions' agents (ADR 0013), admitted on the local socket.
    agents: Arc<Agents>,
    /// The service every transport serves, behind the scope check.
    service: Arc<Guarded>,
    /// The relay links, run once the gateway exists.
    relays: Arc<Relays>,
}

/// The LSP manager's configuration with the operator's roots filled in:
/// executables and toolchains inside any configured root are not trusted
/// for conservative analysis, so the registry needs the full list.
fn lsp_config(config: &Config) -> vessel_lsp::Config {
    let mut lsp = config.lsp.clone();
    lsp.registry.configured_roots = config
        .lsp_roots
        .iter()
        .map(|root| root.path.clone())
        .collect();
    lsp
}

/// Build the session runtime, the lifecycle, the devices and the service.
#[expect(
    clippy::too_many_lines,
    reason = "one list of the node's parts, in the order they depend on each other"
)]
async fn core(
    config: &Config,
    id: NodeId,
    store: &Store,
    keypair: &Arc<NodeKeypair>,
    relaying: Option<(RelayToken, TlsConfig)>,
    setup: SessionSetup,
    provider_setup: crate::providers::PreparedProviders,
) -> Result<Core, NodeError> {
    let key = keypair.public();
    // Sessions ask the node for their prompts' attachments (protocol 10).
    let (loader, loads) = AttachmentLoader::channel(ATTACHMENT_LOADS);
    let runtime = runtime(config, loader, setup.private_dir.clone());
    // A value saved with `node.set_settings` wins over `--park-after`
    // (protocol 12, `settings`).
    let settings = Arc::new(crate::settings::Settings::open(
        config.data_dir.clone(),
        Arc::clone(&runtime),
        ParkAfter::from_duration(config.park_after),
        config.park_after_given,
    ));
    let stop = CancellationToken::new();
    // System zone detection may read OS configuration; keep startup I/O
    // off the async workers just as data-directory preparation is.
    let name = config.name.clone();
    let public_key = *key;
    let info = tokio::task::spawn_blocking(move || identity::node_info(id, name, public_key))
        .await
        .map_err(NodeError::Task)?;
    let lifecycle = Lifecycle::new(info, stop.clone(), Arc::clone(&runtime));
    let tasks = TaskTracker::new();
    let provider_source: Arc<dyn crate::profiles::launch::McpSource> =
        Arc::<fleet::FleetMcp>::clone(&setup.fleet_mcp);
    let providers = provider_setup.start(
        config,
        setup.probe_dir.clone(),
        provider_source,
        &tasks,
        stop.clone(),
    );
    let agents = Arc::new(Agents::new(id));
    let sessions = Arc::new(Sessions::new(
        Arc::clone(&runtime),
        store.clone(),
        tasks.clone(),
        Arc::clone(&agents),
        Arc::clone(&setup.models),
    ));
    let attachments = attachments(config, store, &sessions, loads, (&tasks, &stop));
    let forwarders = TaskTracker::new();
    let devices = Arc::new(Devices::new(
        store.clone(),
        id,
        *key,
        Routes {
            relays: config.relays.clone(),
            direct: config.public.clone(),
        },
    ));
    let relays = Arc::new(Relays::new(Arc::clone(keypair), stop.clone()));
    // A home mints its members' tickets from its relays' token
    // (`fleet.callback`).
    let home_relays = relaying.map(|(token, tls)| {
        relays.link_configured(&config.relays, &token, &tls);
        fleet::HomeRelays {
            relays: config.relays.clone(),
            token,
            first_window: config.fleet.first_ticket_window,
        }
    });
    // Fleet sync's tasks are session-adjacent work: shutdown waits for an
    // apply in progress before the data directory is released.
    let links = Arc::new(fleet::Links::new(
        id,
        lifecycle.info().name,
        FleetFiles::new(&config.data_dir),
        Arc::clone(&devices),
        Arc::clone(&relays),
        home_relays,
    ));
    let fleet = fleet::start(
        FleetFiles::new(&config.data_dir),
        &config.fleet,
        &setup.profiles,
        setup.fleet_mcp,
        Arc::clone(&links),
        &tasks,
        &stop,
    );
    let transfers = Arc::new(Transfers::new(
        transfers::root(&config.data_dir),
        transfers::IDLE,
    ));
    transfers.sweep(&tasks, &stop);
    let lsp = vessel_lsp::Manager::start(lsp_config(config), stop.child_token());
    let lsp_shutdown = lsp.clone();
    drop(tasks.spawn(async move {
        lsp_shutdown.stopped().await;
    }));
    // Clones and setups are the node's tasks: shutdown stops them through
    // `stop` and waits for them before the store closes.
    let workspaces = Workspaces::start(
        Opening {
            data: config.data_dir.root().to_path_buf(),
            workspace: setup.workspace_dir,
            home: config.harness_env.user_home().to_path_buf(),
            git: config.harness_lookup.find("git"),
            git_env: Vec::new(),
            removed: config.terminals.removed.clone().into(),
        },
        store.clone(),
        tasks.clone(),
        stop.clone(),
    )
    .await
    .map_err(NodeError::Store)?;
    // Worktrees whose sessions are gone are tidied at start and after
    // each prune (`tidying`); deletions tidy their own.
    // Terminals beside sessions (protocol 11, ADR 0031): the node's tasks,
    // so its stop ends them and waits for their ends; memory only.
    let terminals = Terminals::new(Runner {
        config: Arc::new(config.terminals.clone()),
        shutdown: stop.clone(),
        tasks: tasks.clone(),
    });
    let ports = {
        let (sessions, terminals) = (Arc::clone(&sessions), Arc::clone(&terminals));
        Ports::start(
            move || {
                let mut roots: Vec<(u32, PortOwner)> = sessions
                    .live_pids()
                    .into_iter()
                    .map(|(pid, session)| (pid, PortOwner::Session { session }))
                    .collect();
                roots.extend(
                    terminals
                        .roots()
                        .into_iter()
                        .map(|(pid, terminal)| (pid, PortOwner::Terminal { terminal })),
                );
                roots
            },
            stop.clone(),
            &tasks,
        )
    };
    let tidying = Tidying::new(
        Arc::clone(&sessions),
        Arc::clone(&workspaces),
        Arc::clone(&terminals),
    );
    Arc::clone(&tidying).watch(store.pruned(), &tasks, stop.clone());
    // Pull requests (protocol 11): the watcher is one of the node's tasks;
    // `gh` runs in the data folder, outside any repository.
    let forges = Forges::start(
        config.forge.clone(),
        config.data_dir.root().to_path_buf(),
        store.clone(),
        Arc::clone(&workspaces),
        &tasks,
        stop.clone(),
    );
    // The scheduler (protocol 11): its handle goes into the service, and
    // it makes runs through the service (`NodeRunner` holds it weakly).
    let (schedules, commands) = Schedules::channel();
    let ended = sessions.ends();
    let push = crate::push::Push::started(config, store.clone(), &tasks, stop.clone())
        .await
        .map_err(NodeError::Store)?;
    let limits = crate::limits::Limits::configured(config, stop.clone(), &tasks).await;
    let service = Arc::new(Guarded(NodeService::new(Parts {
        providers,
        lsp,
        lsp_roots: config.lsp_roots.clone(),
        lifecycle: lifecycle.clone(),
        lookup: config.harness_lookup.clone(),
        sessions,
        devices: Arc::clone(&devices),
        forwarders: forwarders.clone(),
        profiles: setup.profiles,
        models: setup.models,
        launcher: setup
            .launcher
            .with_limit_environment(config.limit_environment.clone()),
        agents: Arc::clone(&agents),
        fleet,
        links,
        transfers,
        attachments,
        probes: Probes::new(setup.probe_dir),
        workspaces,
        tidying,
        forges,
        terminals,
        ports,
        catalogue: crate::catalogue::Catalogues::new(&config.registry_url),
        schedules,
        push,
        settings,
        limits,
    })));
    automation::schedules::start(
        commands,
        automation::schedules::Parts {
            runner: Arc::new(NodeRunner::new(&service)),
            clock: SystemClock,
            store: store.clone(),
            tasks: tasks.clone(),
            stop: stop.clone(),
            ended,
        },
    )
    .await;
    Ok(Core {
        runtime,
        lifecycle,
        stop,
        tasks,
        forwarders,
        devices,
        agents,
        service,
        relays,
    })
}

/// The session runtime, whose structured sessions load their prompts'
/// attachments through `loader` and whose direct drivers write a launch's
/// private files only in `private_dir`.
fn runtime(config: &Config, loader: AttachmentLoader, private_dir: PathBuf) -> Arc<SessionRuntime> {
    Arc::new(
        SessionRuntime::new(
            RuntimeConfig {
                harness_lookup: config.harness_lookup.clone(),
                harness_env: config.harness_env.clone(),
                ring_capacity: config.ring_capacity,
                durable_capacity: config.durable_capacity,
                agent_handshake_timeout: config.agent_handshake_timeout,
                park_after: config.park_after,
                private_dir: Some(private_dir),
            },
            CancellationToken::new(),
        )
        .with_attachments(loader),
    )
}

/// Bind the local socket, and the TCP address if the config has one.
async fn bind(
    config: &Config,
    id: NodeId,
) -> Result<(LocalListener, Option<(TcpListener, SocketAddr)>), NodeError> {
    let endpoint = Endpoint::of(&config.data_dir, id);
    let path = endpoint.path().to_path_buf();
    let local =
        LocalListener::bind(endpoint).map_err(|source| NodeError::LocalSocket { path, source })?;
    let tcp = match config.bind {
        Some(bind) => Some(listen(bind, &config.data_dir).await?),
        None => None,
    };
    Ok((local, tcp))
}

/// Where a direct client counts: behind a trusted reverse proxy, by its
/// header (`--client-address-header`, `--trusted-proxy`), as a relay does.
fn client_address(config: &Config) -> Result<ClientAddress, NodeError> {
    let clients = ClientAddress::new(
        config.client_address_header.clone(),
        config.trusted_proxies.clone(),
    )?;
    if config.client_address_header.is_none() && !config.trusted_proxies.is_empty() {
        tracing::warn!("trusted proxies are ignored without a client address header");
    }
    Ok(clients)
}

/// The relays' registration token and the TLS setup for them, if any
/// relay is configured. The token file is read even without relays, so a
/// broken one shows before it is needed.
fn relaying(config: &Config) -> Result<Option<(RelayToken, TlsConfig)>, NodeError> {
    let token = match &config.relay_token_file {
        Some(path) => Some(vessel_relay::read_token_file(path)?),
        None => None,
    };
    if config.relays.is_empty() {
        return Ok(None);
    }
    let token = token.ok_or(NodeError::NoRelayToken)?;
    Ok(Some((token, relays::relay_tls(&config.relays)?)))
}

/// Bind the TCP listen address.
async fn listen(bind: SocketAddr, dir: &DataDir) -> Result<(TcpListener, SocketAddr), NodeError> {
    let listener = TcpListener::bind(bind)
        .await
        .map_err(|source| NodeError::Bind {
            address: bind,
            source,
        })?;
    let address = listener.local_addr().map_err(|source| NodeError::Bind {
        address: bind,
        source,
    })?;
    if !address.ip().is_loopback() {
        tracing::warn!(
            %address,
            token = %dir.token_path().display(),
            "listening beyond loopback: anyone who can reach this address and \
             read the token file can drive this node's harnesses"
        );
    }
    Ok((listener, address))
}

/// Serve the local socket and (if bound) HTTP until `stop`, then shut down
/// in order and release the data directory last.
async fn serve(
    tcp: Option<(TcpListener, axum::Router)>,
    (local, gateway): (LocalListener, LocalGateway),
    stop: CancellationToken,
    teardown: Teardown,
    lock: NodeLock,
) -> Result<(), NodeError> {
    let http = async {
        if let Some((listener, router)) = tcp {
            // The peer's address, for `/secure`'s per-address budget.
            let router = router.into_make_service_with_connect_info::<SocketAddr>();
            axum::serve(listener, router)
                .with_graceful_shutdown(stop.clone().cancelled_owned())
                .await
                .map_err(NodeError::Serve)
        } else {
            stop.cancelled().await;
            Ok(())
        }
    };
    let ((), served) = tokio::join!(ipc::serve(local, gateway, stop.clone()), http);
    teardown.run().await;
    drop(lock);
    served
}
