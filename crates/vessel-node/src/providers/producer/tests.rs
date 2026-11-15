//! Only testkit processes and explicit synthetic homes/launchers are admitted.

use super::super::cache::{Admission, Cache, Refresh};
use super::*;
use crate::{
    profiles::launch::NoMcpServers,
    providers::owner::{MAX_JOBS, PER_PROVIDER},
};
use vessel_harness::providers::ProviderRegistry;
use vessel_proto::providers::{
    CommandEdit, DisallowedTools, ProviderEnvironment, ProviderOverride, ProviderOverrides,
};

fn work(overrides: &ProviderOverrides, provider: &str) -> Work {
    let mut cache = Cache::new(ProviderRegistry::build(overrides, &[]).unwrap());
    let Admission::Start(work) = cache
        .begin(
            &provider.parse().unwrap(),
            &CatalogueScope::Global,
            Refresh::Force,
        )
        .unwrap()
    else {
        panic!("expected admitted synthetic work")
    };
    work
}

#[tokio::test]
async fn admitted_codex_probe_uses_its_override_and_never_starts_a_thread() {
    vessel_testkit::within(Duration::from_secs(30), async {
        let root = tempfile::tempdir().unwrap();
        let audit = root.path().join("requests");
        let fake = vessel_testkit::Fake::CodexAppServer.path().unwrap();
        let overrides = ProviderOverrides::new(BTreeMap::from([(
            "codex".parse().unwrap(),
            ProviderOverride {
                command: Some(CommandEdit::Replace {
                    program: fake.to_str().unwrap().parse().unwrap(),
                    args: Arguments::default(),
                }),
                env: ProviderEnvironment::new(BTreeMap::from([(
                    "FAKE_CODEX_CATALOGUE_AUDIT".parse().unwrap(),
                    ConfigValue::Text(audit.to_str().unwrap().into()),
                )]))
                .unwrap(),
                ..ProviderOverride::default()
            },
        )]))
        .unwrap();
        let producer = Producer::new(
            Lookup::new([root.path().to_path_buf()]),
            HarnessEnv::new(root.path().to_path_buf()),
            root.path().to_path_buf(),
            Arc::new(NoMcpServers),
        );
        let observed = producer
            .observe(
                work(&overrides, "codex"),
                CancellationToken::new(),
                lease().await,
            )
            .await;
        let Observation::Ready { catalogue, version } = observed else {
            panic!("expected fake catalogue, got {observed:?}")
        };
        assert_eq!(catalogue.models.all().len(), 2);
        assert!(version.is_some());
        assert_eq!(
            std::fs::read_to_string(audit).unwrap(),
            "initialize\nmodel/list\nmodel/list\ngetUserSavedConfig\nconfig/read\n"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn unsupported_driver_missing_launcher_and_denied_policy_are_distinct_before_spawn() {
    let root = tempfile::tempdir().unwrap();
    let producer = Producer::new(
        Lookup::new([root.path().to_path_buf()]),
        HarnessEnv::new(root.path().to_path_buf()),
        root.path().to_path_buf(),
        Arc::new(NoMcpServers),
    );
    assert!(matches!(
        producer
            .observe(
                work(&ProviderOverrides::default(), "pi"),
                CancellationToken::new(),
                lease().await,
            )
            .await,
        Observation::Unavailable(UnavailableReason::DriverMissing)
    ));
    assert!(matches!(
        producer
            .observe(
                work(&ProviderOverrides::default(), "codex"),
                CancellationToken::new(),
                lease().await,
            )
            .await,
        Observation::Unavailable(UnavailableReason::NotInstalled)
    ));
    let fake = vessel_testkit::Fake::CodexAppServer.path().unwrap();
    let audit = root.path().join("must-not-exist");
    let overrides = ProviderOverrides::new(BTreeMap::from([(
        "codex".parse().unwrap(),
        ProviderOverride {
            command: Some(CommandEdit::Replace {
                program: fake.to_str().unwrap().parse().unwrap(),
                args: Arguments::default(),
            }),
            env: ProviderEnvironment::new(BTreeMap::from([(
                "FAKE_CODEX_CATALOGUE_AUDIT".parse().unwrap(),
                ConfigValue::Text(audit.to_str().unwrap().into()),
            )]))
            .unwrap(),
            disallowed_tools: DisallowedTools::new(vec!["Bash(*)".to_owned()]).unwrap(),
            ..ProviderOverride::default()
        },
    )]))
    .unwrap();
    assert!(matches!(
        producer
            .observe(
                work(&overrides, "codex"),
                CancellationToken::new(),
                lease().await
            )
            .await,
        Observation::Failed(DiscoveryFailure::ConfigurationUnavailable)
    ));
    assert!(!audit.exists());
}

async fn lease() -> WorkLease {
    WorkLease::new(
        Arc::new(tokio::sync::Semaphore::new(MAX_JOBS))
            .acquire_owned()
            .await
            .unwrap(),
        Arc::new(tokio::sync::Semaphore::new(PER_PROVIDER))
            .acquire_owned()
            .await
            .unwrap(),
        tokio_util::task::TaskTracker::new(),
    )
}

#[derive(Debug)]
pub(super) struct PreparationBarrier {
    entered: tokio::sync::mpsc::Sender<()>,
    open: std::sync::Mutex<bool>,
    changed: std::sync::Condvar,
}

impl PreparationBarrier {
    pub(super) fn enter(&self) {
        let _received = self.entered.blocking_send(());
        let mut open = self.open.lock().unwrap();
        while !*open {
            open = self.changed.wait(open).unwrap();
        }
    }

    fn release(&self) {
        *self.open.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

struct Gate(Arc<PreparationBarrier>);
impl Drop for Gate {
    fn drop(&mut self) {
        self.0.release();
    }
}

fn gate() -> (Gate, tokio::sync::mpsc::Receiver<()>) {
    let (entered, calls) = tokio::sync::mpsc::channel(MAX_JOBS);
    (
        Gate(Arc::new(PreparationBarrier {
            entered,
            open: std::sync::Mutex::new(false),
            changed: std::sync::Condvar::new(),
        })),
        calls,
    )
}

mod retained;
