//! Actual placement and committed Begin inputs for staged common-owner tests.
//! No raw metadata guard, root constructor, SQL marker or launch is fabricated.
use super::execution_capture::owner::OwnedFixture;
use super::*;
use crate::agents::creation_launch::LaunchInput;
use crate::agents::{Agents, CapturedParentExecution, Registration};
use crate::principal::Principal;
use crate::workspace::{Opening, Workspaces};
use futures_util::FutureExt as _;
use std::{future::Future, panic::AssertUnwindSafe, time::Duration};
use vessel_proto::creation::{
    Advance, Fingerprint, FirstInput, Key, Kind, Receipt, Reservation, Resource, Stage,
    WorkspaceClaim,
};
use vessel_proto::registry::{
    RegistryRevision, WorkspaceAutomation, WorkspaceBacking, WorkspaceDescriptor,
    WorkspaceLifecycle, WorkspacePin, WorkspaceResource, WorkspaceVisibility,
};
use vessel_proto::workspace::ProjectInfo;
use vessel_proto::{AgentAuth, BlobHash, Creator, WorkspaceId, WorkspaceOperationId};
use vessel_store::{CreationAdmission, CreationIdentity, RegistryRecord};

const PHASE_BOUND: Duration = Duration::from_secs(15);

pub(crate) async fn phase<F: Future>(name: &'static str, work: F) -> F::Output {
    tokio::time::timeout(PHASE_BOUND, Box::pin(work))
        .await
        .unwrap_or_else(|error| panic!("creation fixture {name}: {error}"))
}

pub(crate) struct CreationFixture {
    inner: Arc<Fixture>,
    owner: std::sync::Mutex<Option<OwnedFixture>>,
    workspaces: Arc<Workspaces>,
    project: ProjectInfo,
    workspace: WorkspaceId,
    parent: SessionId,
    stop: CancellationToken,
}

impl CreationFixture {
    /// Setup has no fake process; parent creation occurs inside run's retained
    /// failure owner, so an assertion/phase failure cannot drop a live harness.
    pub(crate) async fn new() -> Self {
        let owner = OwnedFixture::new().await;
        let inner = owner.shared();
        // The same retained owner keeps this root alive even if the cleanup
        // observer times out; the inherited fake-bin TempDir owns its parent.
        let root = inner._bin.path().join("creation-project");
        std::fs::create_dir(&root).unwrap();
        let dir = DataDir::new(inner.data.path());
        let home = inner.data.path().join("test-home");
        std::fs::create_dir(&home).unwrap();
        let stop = CancellationToken::new();
        let workspaces = phase(
            "workspace owner",
            Workspaces::start(
                Opening {
                    data: dir.root().to_path_buf(),
                    workspace: dir.prepare_workspace_dir().unwrap(),
                    home,
                    git: None,
                    git_env: Vec::new(),
                    removed: crate::TerminalsConfig::default().removed.into(),
                },
                inner.store.clone(),
                inner.tasks.clone(),
                stop.clone(),
            ),
        )
        .await
        .unwrap();
        let project = phase("project registration", workspaces.add_project(root, None))
            .await
            .unwrap();
        Self {
            inner,
            owner: std::sync::Mutex::new(Some(owner)),
            workspaces,
            project,
            workspace: WorkspaceId::random(),
            parent: SessionId::random(),
            stop,
        }
    }

    pub(crate) fn agents(&self) -> &Arc<Agents> {
        &self.inner.sessions.agents
    }

    pub(crate) fn store(&self) -> &Store {
        &self.inner.store
    }

    pub(crate) fn workspace(&self) -> WorkspaceId {
        self.workspace
    }

    pub(crate) fn parent_id(&self) -> SessionId {
        self.parent
    }

    pub(crate) async fn close_runtime(&self) {
        phase("explicit runtime close", self.inner.runtime.shutdown()).await;
    }

    /// Authenticate through the actual credential owner, then capture from the
    /// same registry that Sessions populated with the real fake actor handle.
    pub(crate) async fn parent(&self) -> (Principal, Registration) {
        self.descriptor().await;
        let start = phase(
            "parent start claim",
            self.store().claim_registry_start(
                self.workspace,
                WorkspaceResource::Session {
                    session: self.parent,
                },
                RegistryRevision::ZERO,
            ),
        )
        .await
        .unwrap();
        let secret = self
            .agents()
            .reserve(self.parent, &Principal::Owner, true)
            .unwrap()
            .unwrap();
        let info = phase(
            "parent create",
            self.inner.sessions.create(
                self.inner.structured().with_cwd(self.project.root.clone()),
                LaunchOptions {
                    id: Some(self.parent),
                    ..LaunchOptions::default()
                },
                None,
            ),
        )
        .await
        .unwrap();
        assert_eq!(info.id, self.parent);
        let Found::Live { handle, .. } = phase("parent lookup", self.inner.sessions.find(info.id))
            .await
            .unwrap()
        else {
            panic!("actual fake parent must be live");
        };
        let _running = phase("parent ready", ready(&handle)).await.unwrap();
        phase(
            "parent registration commit",
            self.store().complete_registry_start(start),
        )
        .await
        .unwrap();
        self.agents()
            .admit(
                &AgentAuth {
                    session: self.parent,
                    secret,
                },
                &self.stop,
            )
            .unwrap()
    }

    async fn descriptor(&self) {
        phase(
            "descriptor commit",
            self.store().put_registry_workspace(
                RegistryRecord {
                    descriptor: WorkspaceDescriptor {
                        id: self.workspace,
                        project: self.project.id,
                        path: self.project.root.clone(),
                        title: None,
                        automation: WorkspaceAutomation::Allowed,
                        pin: WorkspacePin::Unpinned,
                        backing: WorkspaceBacking::Local,
                        lifecycle: WorkspaceLifecycle::Active,
                        visibility: WorkspaceVisibility::Foreground,
                        created_at: chrono::Utc::now(),
                        revision: RegistryRevision::ZERO,
                    },
                    recovery: None,
                },
                None,
            ),
        )
        .await
        .unwrap();
    }

    pub(crate) async fn input(
        &self,
        capture: CapturedParentExecution,
        child: SessionId,
    ) -> LaunchInput {
        let project = phase(
            "project admission",
            self.workspaces.admit_project(self.project.id),
        )
        .await
        .unwrap();
        let expected = phase(
            "placement revision",
            self.store().registry_workspace(self.workspace),
        )
        .await
        .unwrap()
        .unwrap()
        .descriptor
        .revision;
        let placement = phase(
            "retained placement",
            self.inner.sessions.hold_creation_placement(
                &self.workspaces,
                project,
                super::super::BoundCreationPlace {
                    parent: self.parent,
                    child,
                    workspace: self.workspace,
                    expected,
                },
            ),
        )
        .await
        .unwrap();
        let creator = Creator::Agent {
            node: capture.stamp().node,
            session: capture.stamp().session,
        };
        LaunchInput::new(
            capture,
            placement,
            self.inner.structured().with_cwd(self.project.root.clone()),
            LaunchOptions {
                id: Some(child),
                created_by: creator,
                ..LaunchOptions::default()
            },
        )
    }

    pub(crate) async fn begin(&self, principal: &Principal, child: SessionId) -> Receipt {
        let (node, parent) = principal.agent().unwrap();
        let admitted = phase(
            "receipt admission commit",
            self.store().admit_creation(CreationAdmission {
                identity: CreationIdentity::for_agent(
                    node,
                    parent,
                    Key::parse(&format!("child-{child}")).unwrap(),
                ),
                receipt: Receipt::admitted(
                    Kind::Agent,
                    Reservation {
                        operation: WorkspaceOperationId::random(),
                        workspace: self.workspace,
                        workspace_claim: WorkspaceClaim::Existing,
                        resource: Some(Resource::Agent { session: child }),
                        first_input: FirstInput::Absent,
                    },
                    Fingerprint::new(BlobHash::from_bytes([7; 32])),
                )
                .unwrap(),
            }),
        )
        .await
        .unwrap();
        phase(
            "resource Begin commit acknowledgement",
            self.store().advance_creation(
                admitted.snapshot().reserved.operation,
                admitted.snapshot().revision,
                Advance::Begin(Stage::Resource),
            ),
        )
        .await
        .unwrap()
    }

    /// Observation deadlines never cancel actual cleanup: consume the SAME
    /// qualified retained owner after either success or assertion/phase panic.
    pub(crate) async fn run(&self, work: impl Future<Output = ()>) {
        let outcome = AssertUnwindSafe(Box::pin(work)).catch_unwind().await;
        self.stop.cancel();
        let owner = self
            .owner
            .lock()
            .unwrap()
            .take()
            .expect("fixture runs once");
        let cleanup = AssertUnwindSafe(owner.finish()).catch_unwind().await;
        for result in [outcome, cleanup] {
            if let Err(panic) = result {
                std::panic::resume_unwind(panic);
            }
        }
    }
}
