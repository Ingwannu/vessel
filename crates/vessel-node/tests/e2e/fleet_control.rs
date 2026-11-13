//! Fleet controls over real authenticated connections. The lifecycle test
//! keeps both nodes and their source/target homes together so manual sync,
//! persisted membership, reports and leave are checked against one history.
//! This module is just over 400 lines because cancellation, post-redemption
//! failure and restricted reads share the same real authentication helpers;
//! the lifecycle stays one history rather than independently seeded claims.
use crate::agents::{as_agent, code, credential, structured};
use crate::fleet::{fleet_config, until, write};
use crate::secure::{Relay, device_via, dialer, relayed_with};
use crate::support::{Fixture, client, within};
use std::path::Path;
use std::time::Duration;
use vessel_client::secure::{RouteTimeouts, pair_over};
use vessel_client::{Connector as _, NodeClient};
use vessel_node::fleet::{self, FleetFiles, FleetInit};
use vessel_proto::{
    ErrorCode, ErrorDetail, FleetControlError, FleetRole, GetFleetStatus, HarnessKind, LeaveFleet,
    Scope, SyncFleet,
};
use vessel_testkit::fleet_fixtures::{INSTRUCTIONS, MEMBER_NAME};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_partial_leave_revokes_the_home_and_restart_keeps_only_history() {
    within(async {
        let relay = Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let member = crate::across::member_granting(
            &relay,
            &home,
            "partial-leave",
            vec![Scope::Sync],
            vec![Scope::Read, Scope::Operate],
        )
        .await;
        let files = FleetFiles::new(&member.fixture.data_dir());
        let credential = files.dir().join("member.credential");
        std::fs::remove_file(&credential).unwrap();
        std::fs::create_dir(&credential).unwrap();
        let error = member.owner.leave_fleet().await.unwrap_err();
        let vessel_client::CallError::Remote(error) = error else {
            panic!("expected partial leave refusal")
        };
        assert_eq!(
            error.detail,
            Some(ErrorDetail::FleetControl {
                reason: FleetControlError::CannotStore
            })
        );
        assert!(!files.dir().join("member.json").exists());
        assert!(credential.is_dir());
        assert_eq!(
            member.owner.fleet_status().await.unwrap().role,
            FleetRole::Standalone
        );
        let label = format!("fleet home {}", home.id());
        let previous = member
            .owner
            .devices()
            .await
            .unwrap()
            .into_iter()
            .find(|device| device.label == label)
            .unwrap();
        assert!(
            previous.revoked_at.is_some(),
            "partial leave must revoke before returning its error"
        );
        member.node.stop().await.unwrap();
        // No membership survives restart, even while orphan cleanup still
        // fails. Cached history cannot authorize another connection or pull.
        let restarted = relayed_with(&member.fixture, &[&relay.url], |_| {}).await;
        let owner = member.fixture.connect(&restarted).await;
        assert_eq!(
            owner.fleet_status().await.unwrap().role,
            FleetRole::Standalone
        );
        std::fs::remove_dir(&credential).unwrap();
        owner.leave_fleet().await.unwrap();
        assert!(!credential.exists());
        restarted.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

fn contents(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn admin_status_refuses_an_incomplete_device_window_without_affecting_read_only_history() {
    within(async {
        let relay = Relay::start().await;
        let fixture = Fixture::new();
        let store =
            vessel_store::Store::open(&fixture.data_dir(), vessel_store::StoreConfig::default())
                .await
                .unwrap()
                .store;
        for _ in 0..=vessel_store::DeviceWindow::MAX_ITEMS {
            let info = vessel_proto::DeviceInfo {
                id: vessel_proto::DeviceId::random(),
                label: "bounded fixture".to_owned(),
                created_at: chrono::Utc::now(),
                last_seen: None,
                scopes: vec![Scope::Sync],
                revoked_at: None,
            };
            store
                .add_device(
                    info,
                    vessel_store::CredentialHash::of(&vessel_proto::DeviceSecret::from_bytes(
                        [7; 32],
                    )),
                )
                .await
                .unwrap();
        }
        store.close().await;
        drop(store);
        let source = fixture.cwd().join("shared-instructions");
        write(&source, "bounded status fixture\n");
        let _sources = fleet::init(
            &FleetFiles::new(&fixture.data_dir()),
            &FleetInit {
                instructions: Some(source),
                skills: None,
                skip: Vec::new(),
            },
        )
        .unwrap();
        let node = relayed_with(&fixture, &[&relay.url], |config| {
            config.fleet.targets = vessel_node::FleetTargets::Only(Vec::new());
        })
        .await;
        let owner = fixture.connect(&node).await;
        let error = owner.fleet_status().await.unwrap_err();
        let vessel_client::CallError::Remote(error) = error else {
            panic!("expected bounded status refusal")
        };
        assert_eq!(error.code, ErrorCode::Unavailable);
        assert_eq!(
            error.detail,
            Some(ErrorDetail::FleetControl {
                reason: FleetControlError::StatusTooLarge
            })
        );
        let reader = device(&owner, &relay, vec![Scope::Read]).await;
        let history = reader.fleet_status().await.unwrap();
        assert_eq!(history.role, FleetRole::Home);
        assert_eq!(history.members, None);
        owner.sync_fleet().await.unwrap();
        node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

async fn device(owner: &NodeClient, relay: &Relay, scopes: Vec<Scope>) -> NodeClient {
    let offer = owner
        .invite_with_scopes(None, 600, scopes)
        .await
        .unwrap()
        .link
        .decode()
        .unwrap();
    let (paired, _) = pair_over(
        &dialer(),
        &offer,
        "control reader",
        RouteTimeouts::default(),
    )
    .await
    .unwrap();
    device_via(dialer(), &paired, &[], &[&relay.url])
        .connect(client())
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wire_join_manual_sync_reports_and_leave_share_the_existing_engine() {
    within(async {
        let relay = Relay::start().await;
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source/AGENTS.md");
        let [before, after] = INSTRUCTIONS;
        assert_eq!(before.len(), after.len());
        write(&source, before);
        let home_fixture = Fixture::new();
        let home_files = FleetFiles::new(&home_fixture.data_dir());
        let _sources = fleet::init(
            &home_files,
            &FleetInit {
                instructions: Some(source.clone()),
                skills: None,
                skip: Vec::new(),
            },
        )
        .unwrap();
        let home_target = temp.path().join("home-claude");
        let home = relayed_with(&home_fixture, &[&relay.url], |config| {
            config.name = Some("control-home".to_owned());
            config.fleet = fleet_config(HarnessKind::ClaudeCode, home_target.clone());
            config.fleet.poll = Duration::from_secs(3600);
            config.fleet.watch_limit = Duration::from_secs(3600);
        })
        .await;
        let home_owner = home_fixture.connect(&home).await;
        until("the first home publication", || {
            contents(&home_target.join("CLAUDE.md")) == before
        })
        .await;
        let first = home_owner.fleet_status().await.unwrap();
        assert_eq!(first.role, FleetRole::Home);
        let original = published_bundle(&home_owner).await;

        let member_fixture = Fixture::new();
        let member_target = temp.path().join("member-codex");
        let later_target = temp.path().join("later-claude");
        let member = relayed_with(&member_fixture, &[&relay.url], |config| {
            config.name = Some(MEMBER_NAME.to_owned());
            config.fleet = fleet_config(HarnessKind::Codex, member_target.clone());
            config.fleet.targets = vessel_node::FleetTargets::Only(vec![
                (HarnessKind::Codex, member_target.clone()),
                (HarnessKind::ClaudeCode, later_target.clone()),
            ]);
            config.fleet.poll = Duration::from_secs(3600);
            config.fleet.watch_limit = Duration::from_secs(3600);
        })
        .await;
        let owner = member_fixture.connect(&member).await;
        let member_files = FleetFiles::new(&member_fixture.data_dir());
        confirm_join(&home_owner, &owner, &member_files, home.info().id).await;
        until("member applies", || {
            fleet::status(&member_files).unwrap().state.generation == Some(original)
        })
        .await;
        until("home keeps member report", || {
            home_files.dir().join("members.json").is_file()
        })
        .await;
        let status = owner.fleet_status().await.unwrap();
        assert!(matches!(status.role,FleetRole::Member {home: id,..} if id==home.info().id));
        assert_eq!(status.local.generation, Some(original));
        assert_eq!(status.members, None);
        let reports = home_owner.fleet_status().await.unwrap().members.unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].applied, Some(original));
        assert_eq!(reports[0].label, MEMBER_NAME);

        check_manual_sync(
            &home_owner,
            &owner,
            original,
            &source,
            &later_target,
            &member_target,
        )
        .await;
        owner.leave_fleet().await.unwrap();
        assert_eq!(
            owner.fleet_status().await.unwrap().role,
            FleetRole::Standalone
        );
        assert!(!member_files.dir().join("member.json").exists());
        assert!(!member_files.dir().join("member.credential").exists());
        assert_eq!(contents(&member_target.join("AGENTS.md")), after);
        owner.leave_fleet().await.unwrap();
        member.stop().await.unwrap();
        home.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn read_only_and_agent_status_do_not_disclose_admin_member_reports() {
    within(async {
        let relay = Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let reader = device(&home.owner, &relay, vec![Scope::Read]).await;
        let no_read = device(&home.owner, &relay, vec![Scope::Sync]).await;
        let status = reader.fleet_status().await.unwrap();
        assert_eq!(status.role, FleetRole::Home);
        assert_eq!(status.members, None);
        assert_eq!(code(no_read.fleet_status().await), ErrorCode::Forbidden);
        assert_eq!(code(reader.sync_fleet().await), ErrorCode::Forbidden);
        assert_eq!(code(reader.leave_fleet().await), ErrorCode::Forbidden);
        let link = home
            .owner
            .invite_with_scopes(None, 600, vec![Scope::Sync])
            .await
            .unwrap()
            .link;
        assert_eq!(
            code(reader.join_fleet(link, Vec::new()).await),
            ErrorCode::Forbidden
        );
        let mut session = structured(&home.owner, &home.fixture).await;
        let (_, socket, auth) = credential(&home.owner, &mut session).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(agent.fleet_status().await.unwrap().members, None);
        assert_eq!(code(agent.sync_fleet().await), ErrorCode::Forbidden);
        home.owner.kill(session.id).await.unwrap();

        // Even an explicitly Admin-granted member's delegated read gets
        // only local history. Administration stays on its own connection.
        let member = crate::across::member_granting(
            &relay,
            &home,
            "status-admin",
            Scope::ALL.to_vec(),
            vec![Scope::Read],
        )
        .await;
        let delegated = member
            .owner
            .call_on(home.id(), GetFleetStatus {})
            .await
            .unwrap();
        assert_eq!(delegated.role, FleetRole::Home);
        assert_eq!(delegated.members, None);
        assert_eq!(
            code(home.owner.call_on(member.id(), SyncFleet {}).await),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            code(home.owner.call_on(member.id(), LeaveFleet {}).await),
            ErrorCode::InvalidRequest
        );
        member.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

async fn published_bundle(owner: &NodeClient) -> vessel_proto::Generation {
    // A materialized file precedes publication. The public watch refuses
    // before the first bundle, so pace this read-only readiness probe and
    // keep the existing STEP deadline; never infer readiness from a file.
    crate::support::step("the first published fleet bundle", async {
        let mut tick = tokio::time::interval(Duration::from_millis(25));
        loop {
            let _tick = tick.tick().await;
            match owner.fleet_manifest().await {
                Ok(manifest) => return manifest.generation,
                Err(vessel_client::CallError::Remote(error))
                    if error.code == ErrorCode::Unavailable => {}
                Err(error) => panic!("cannot read the initial published bundle: {error}"),
            }
        }
    })
    .await
}

async fn confirm_join(
    home_owner: &NodeClient,
    owner: &NodeClient,
    member_files: &FleetFiles,
    home_id: vessel_proto::NodeId,
) {
    assert_eq!(
        owner.fleet_status().await.unwrap().role,
        FleetRole::Standalone
    );
    assert_eq!(code(owner.sync_fleet().await), ErrorCode::Conflict);
    let link = home_owner
        .invite_with_scopes(None, 600, vec![Scope::Sync])
        .await
        .unwrap()
        .link;
    let self_refusal = home_owner
        .join_fleet(link.clone(), Vec::new())
        .await
        .unwrap_err();
    let vessel_client::CallError::Remote(self_refusal) = self_refusal else {
        panic!("expected self-join refusal")
    };
    assert_eq!(
        self_refusal.detail,
        Some(ErrorDetail::FleetControl {
            reason: FleetControlError::SelfHome
        })
    );
    // The refused self-join did not redeem the single-use invitation.
    // A real post-redemption disk failure is distinct: this directory
    // cannot be replaced by the credential file. No membership appears.
    let credential_path = member_files.dir().join("member.credential");
    std::fs::create_dir_all(&credential_path).unwrap();
    let failed = owner
        .join_fleet(link.clone(), Vec::new())
        .await
        .unwrap_err();
    let vessel_client::CallError::Remote(failed) = failed else {
        panic!("expected post-redemption refusal")
    };
    assert_eq!(
        failed.detail,
        Some(ErrorDetail::FleetControl {
            reason: FleetControlError::JoinUncertain,
        })
    );
    assert_eq!(
        owner.fleet_status().await.unwrap().role,
        FleetRole::Standalone
    );
    assert!(!member_files.dir().join("member.json").exists());
    let spent = home_owner
        .devices()
        .await
        .unwrap()
        .into_iter()
        .find(|device| device.label == MEMBER_NAME)
        .unwrap();
    home_owner.revoke_device(spent.id).await.unwrap();
    std::fs::remove_dir(&credential_path).unwrap();
    // Never automatically retry an uncertain join. This explicit probe
    // proves its invitation was consumed, then a new invitation is used.
    assert_eq!(
        code(owner.join_fleet(link, Vec::new()).await),
        ErrorCode::Unavailable
    );
    let link = home_owner
        .invite_with_scopes(None, 600, vec![Scope::Sync])
        .await
        .unwrap()
        .link;
    let joined = owner.join_fleet(link.clone(), Vec::new()).await.unwrap();
    assert_eq!(joined.home.id, home_id);
    let refused = owner.join_fleet(link, Vec::new()).await.unwrap_err();
    let vessel_client::CallError::Remote(refused) = refused else {
        panic!("expected membership refusal")
    };
    assert_eq!(
        refused.detail,
        Some(ErrorDetail::FleetControl {
            reason: FleetControlError::AlreadyMember
        })
    );
}

async fn check_manual_sync(
    home_owner: &NodeClient,
    owner: &NodeClient,
    original: vessel_proto::Generation,
    source: &Path,
    later_target: &Path,
    member_target: &Path,
) {
    let [before, after] = INSTRUCTIONS;
    // No publication changed. A new harness home is seen only by a
    // fresh pull; the automatic pull/watch deadlines are an hour away.
    std::fs::create_dir_all(later_target).unwrap();
    owner.sync_fleet().await.unwrap();
    until("manual pull sees a new profile home", || {
        contents(&later_target.join("CLAUDE.md")) == before
    })
    .await;

    // Same bytes length and mtime defeat the cheap stamp. A manual
    // home sync still rebundles; its real generation watch is the receipt.
    let modified = std::fs::metadata(source).unwrap().modified().unwrap();
    std::fs::write(source, after).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(source)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    home_owner.sync_fleet().await.unwrap();
    let changed = home_owner.fleet_watch(original).await.unwrap();
    assert!(changed > original);
    assert_eq!(
        home_owner.fleet_status().await.unwrap().local.generation,
        Some(changed)
    );
    until("member follows the new publication", || {
        contents(&member_target.join("AGENTS.md")) == after
    })
    .await;
}
