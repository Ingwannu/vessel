use super::*;
use std::ffi::OsString;
pub(super) const TEST_BUDGET: Duration = Duration::from_secs(30);
pub(super) struct Fixture {
    pub(super) sources: Sources,
    pub(super) global: Observation,
    root: tempfile::TempDir,
    stop: CancellationToken,
    tasks: TaskTracker,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
impl Fixture {
    pub(super) async fn start(
        reader: impl Fn(&str) -> Option<OsString> + Send + Sync + 'static,
    ) -> Self {
        let root = tempfile::tempdir().unwrap();
        let stop = CancellationToken::new();
        let tasks = TaskTracker::new();
        let body = root.path().join("usage-body.json");
        std::fs::write(&body, br#"{"copilot_plan":"Example plan"}"#).unwrap();
        let mut config = crate::Config::new(
            DataDir::new(root.path()),
            vessel_harness::HarnessEnv::new(root.path().to_owned()),
        );
        config.usage_sources = AccountUsageProbe::Recorded {
            root: root.path().to_owned(),
            access: vessel_harness::AccountSourceAccess::new(root.path().to_owned(), reader),
            source: vessel_proto::usage_reports::BuiltinUsageSource::Copilot,
            body,
            status: 200,
            retry_after: None,
            format: vessel_harness::RecordedUsageFormat::Vendor,
        };
        let sources = Sources::start(&config, stop.clone(), &tasks, LimitContext::new()).await;
        Self {
            sources,
            global: Observation::fixture(CancellationToken::new()),
            root,
            stop,
            tasks,
        }
    }
    pub(super) fn request(policy: ExternalLookup) -> SetUsageSourceLookup {
        SetUsageSourceLookup {
            source: "copilot".parse().unwrap(),
            policy,
            disclosure_version: UsageDisclosureVersion::INITIAL,
        }
    }
    pub(super) async fn allow(&self, policy: ExternalLookup) -> UsageSourceDescriptor {
        self.sources
            .set(Self::request(policy), &self.global)
            .await
            .unwrap()
    }
    pub(super) fn dir(&self) -> DataDir {
        DataDir::new(self.root.path())
    }
    pub(super) fn body(&self) -> std::path::PathBuf {
        self.root.path().join("usage-body.json")
    }
    pub(super) async fn finish(&self) {
        self.stop.cancel();
        let _closed = self.tasks.close();
        tokio::time::timeout(TEST_BUDGET, self.tasks.wait())
            .await
            .unwrap();
    }
}
