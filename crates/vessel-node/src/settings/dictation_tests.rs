//! Whole-candidate refusal and actual atomic-writer failure, never a fake writer.
use super::*;
use vessel_proto::dictation::{
    DictationProvider, EndpointModel, LocalModelId, SecretReference, SetDictation,
};
use vessel_proto::settings::SetAgentDrivers;

const TEST_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);
struct Fixture {
    root: tempfile::TempDir,
    data: DataDir,
    settings: Arc<Settings>,
    reader: Reader,
    files: crate::fleet::FleetFiles,
    name: vessel_proto::SecretName,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path().join("data"));
        std::fs::create_dir(data.root()).unwrap();
        let settings = test_fixture::open(data.clone());
        let reader = settings.dictation_reader();
        let files = crate::fleet::FleetFiles::new(&data);
        let name = "TRANSCRIPTION".parse().unwrap();
        crate::fleet::set_secret(&files, &name, "synthetic-selected-key").unwrap();
        Self {
            root,
            data,
            settings,
            reader,
            files,
            name,
        }
    }
    fn endpoint(&self) -> SetNodeSettings {
        SetNodeSettings {
            park_after: Some(ParkAfter::Never),
            drivers: Some(SetAgentDrivers {
                codex: Some(AgentDriver::Acp),
                ..SetAgentDrivers::default()
            }),
            dictation: Some(SetDictation {
                provider: Some(DictationChange::Set(DictationProvider::Endpoint)),
                key: Some(DictationChange::Set(SecretReference::from(
                    self.name.clone(),
                ))),
                ..SetDictation::default()
            }),
        }
    }
    async fn establish(&self) {
        let _configured = self
            .settings
            .change_dictation(self.endpoint(), &self.reader)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn a_provider_switch_with_any_bad_field_or_new_key_changes_nothing() {
    vessel_testkit::within(TEST_LIMIT, async {
        let f = Fixture::new();
        f.establish().await;
        let before = std::fs::read(f.data.settings_path()).unwrap();
        let current = f.settings.current_dictation(&f.reader).await.unwrap();
        let missing = "MISSING".parse::<vessel_proto::SecretName>().unwrap();
        let blank = "BLANK".parse::<vessel_proto::SecretName>().unwrap();
        crate::fleet::set_secret(&f.files, &blank, " \t ").unwrap();
        let bad = [
            SetDictation {
                provider: Some(DictationChange::Set(DictationProvider::Local)),
                endpoint_model: Some(DictationChange::Set(EndpointModel::whisper())),
                ..SetDictation::default()
            },
            SetDictation {
                provider: Some(DictationChange::Set(DictationProvider::Local)),
                local_model: Some(DictationChange::Set(
                    "future-model".parse::<LocalModelId>().unwrap(),
                )),
                ..SetDictation::default()
            },
            SetDictation {
                key: Some(DictationChange::Set(SecretReference::from(missing))),
                ..SetDictation::default()
            },
            SetDictation {
                key: Some(DictationChange::Set(SecretReference::from(blank))),
                ..SetDictation::default()
            },
        ];
        for patch in bad {
            let change = SetNodeSettings {
                park_after: Some(ParkAfter::DEFAULT),
                dictation: Some(patch),
                ..SetNodeSettings::default()
            };
            assert!(
                f.settings
                    .change_dictation(change, &f.reader)
                    .await
                    .is_err()
            );
            assert_eq!(std::fs::read(f.data.settings_path()).unwrap(), before);
            assert_eq!(
                f.settings.current_dictation(&f.reader).await.unwrap(),
                current
            );
        }
        assert!(crate::fleet::delete_secret(&f.files, &f.name).unwrap());
        // Same explicit reference still has to resolve, despite an otherwise no-op patch.
        assert!(matches!(
            f.settings.change_dictation(f.endpoint(), &f.reader).await,
            Err(SettingsError::DictationKeyAbsent)
        ));
        assert_eq!(std::fs::read(f.data.settings_path()).unwrap(), before);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn switching_to_an_endpoint_with_a_missing_or_empty_key_keeps_the_previous_provider() {
    vessel_testkit::within(TEST_LIMIT, async {
        let f = Fixture::new();
        f.establish().await;
        let _local = f
            .settings
            .change_dictation(
                SetNodeSettings {
                    dictation: Some(SetDictation {
                        provider: Some(DictationChange::Set(DictationProvider::Local)),
                        language: Some(DictationChange::Set("ko".parse().unwrap())),
                        ..SetDictation::default()
                    }),
                    ..SetNodeSettings::default()
                },
                &f.reader,
            )
            .await
            .unwrap();
        let before = std::fs::read(f.data.settings_path()).unwrap();
        let current = f.settings.current_dictation(&f.reader).await.unwrap();
        let missing = "SWITCH_MISSING"
            .parse::<vessel_proto::SecretName>()
            .unwrap();
        let empty = "SWITCH_EMPTY".parse::<vessel_proto::SecretName>().unwrap();
        crate::fleet::set_secret(&f.files, &empty, " \t ").unwrap();
        for name in [missing, empty] {
            let result = f
                .settings
                .change_dictation(
                    SetNodeSettings {
                        park_after: Some(ParkAfter::DEFAULT),
                        dictation: Some(SetDictation {
                            provider: Some(DictationChange::Set(DictationProvider::Endpoint)),
                            key: Some(DictationChange::Set(SecretReference::from(name))),
                            ..SetDictation::default()
                        }),
                        ..SetNodeSettings::default()
                    },
                    &f.reader,
                )
                .await;
            assert!(matches!(result, Err(SettingsError::DictationKeyAbsent)));
            assert_eq!(std::fs::read(f.data.settings_path()).unwrap(), before);
            assert_eq!(
                f.settings.current_dictation(&f.reader).await.unwrap(),
                current
            );
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn failed_presence_read_refuses_before_an_unrelated_setting_is_written() {
    vessel_testkit::within(TEST_LIMIT, async {
        let f = Fixture::new();
        f.establish().await;
        let before = std::fs::read(f.data.settings_path()).unwrap();
        let current = f.settings.current().await;
        std::fs::write(f.files.secrets(), b"not valid TOML = [").unwrap();
        let result = f
            .settings
            .change_dictation(
                SetNodeSettings {
                    park_after: Some(ParkAfter::DEFAULT),
                    ..SetNodeSettings::default()
                },
                &f.reader,
            )
            .await;
        assert!(matches!(result, Err(SettingsError::DictationSecret(_))));
        assert_eq!(std::fs::read(f.data.settings_path()).unwrap(), before);
        assert_eq!(f.settings.current().await, current);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn actual_failed_write_preserves_the_saved_file_and_all_settings_in_force() {
    vessel_testkit::within(TEST_LIMIT, async {
        let mut f = Fixture::new();
        f.establish().await;
        let before = std::fs::read(f.data.settings_path()).unwrap();
        let current = f.settings.current_dictation(&f.reader).await.unwrap();
        let gate = Arc::get_mut(&mut f.settings).unwrap().hold_writes();
        let patch = SetDictation {
            provider: Some(DictationChange::Set(DictationProvider::Local)),
            ..SetDictation::default()
        };
        let mut changing = Box::pin(f.settings.change_dictation(
            SetNodeSettings {
                park_after: Some(ParkAfter::DEFAULT),
                dictation: Some(patch),
                ..SetNodeSettings::default()
            },
            &f.reader,
        ));
        assert!(futures_util::poll!(changing.as_mut()).is_pending());
        gate.entered().await;
        let backup = f.root.path().join("original-directory");
        std::fs::rename(f.data.root(), &backup).unwrap();
        std::fs::write(f.data.root(), b"not a directory").unwrap();
        gate.release();
        let result = changing.await;
        // Restore the isolated fixture before assertions, even if the refusal regresses.
        std::fs::remove_file(f.data.root()).unwrap();
        std::fs::rename(&backup, f.data.root()).unwrap();
        assert!(matches!(result, Err(SettingsError::Write(_))));
        assert_eq!(std::fs::read(f.data.settings_path()).unwrap(), before);
        assert_eq!(
            f.settings.current_dictation(&f.reader).await.unwrap(),
            current
        );
    })
    .await
    .unwrap();
}

#[derive(serde::Deserialize)]
struct BeforeDictation {
    park_after: Option<ParkAfter>,
    drivers: Option<SetAgentDrivers>,
}

#[tokio::test]
async fn a_pre_dictation_writer_forgets_the_active_block_but_keeps_the_secret_and_later_copy() {
    vessel_testkit::within(TEST_LIMIT, async {
        let f = Fixture::new();
        f.establish().await;
        let original = std::fs::read(f.data.settings_path()).unwrap();
        // Typed historical shape: unknown top-level dictation is ignored, as
        // the earlier SetNodeSettings did. This is no old-binary runtime claim.
        let old: BeforeDictation = serde_json::from_slice(&original).unwrap();
        let known = SetNodeSettings {
            park_after: old.park_after,
            drivers: old.drivers,
            dictation: None,
        };
        assert!(f.data.settings_hold_more_than(&known).unwrap());
        let aside = f
            .data
            .set_aside_settings(chrono::Utc::now(), SetAside::Later)
            .unwrap();
        f.data.write_settings(&known).unwrap();
        assert!(f.data.read_settings().unwrap().unwrap().dictation.is_none());
        assert_eq!(std::fs::read(aside).unwrap(), original);
        assert!(
            f.files.resolved_secret(&f.name).unwrap().unwrap().expose() == "synthetic-selected-key"
        );
    })
    .await
    .unwrap();
}
