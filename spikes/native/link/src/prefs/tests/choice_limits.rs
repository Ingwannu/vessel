//! Settings' offered opaque ids cross the same validator and real file writer.
use super::*;
use vessel_app_view::agent_status::defaults::{SaveProblem, prepare};
use vessel_app_view::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};
use vessel_proto::{
    OptionCategory, OptionId, OptionKind, OptionValue, OptionValueId, SessionOption,
};

// The actual value offered by fake-acp's `FAKE_ACP_OPTIONS=real` fixture.
const OFFERED_MODEL: &str = "research-preview-model-with-a-name-far-longer-than-any-list-is-wide";

struct PreferenceFile(PathBuf);
impl PreferenceFile {
    fn new(name: &str) -> std::io::Result<Self> {
        scratch(name).map(Self)
    }
}
impl Drop for PreferenceFile {
    fn drop(&mut self) {
        if let Some(dir) = self.0.parent() {
            drop(std::fs::remove_dir_all(dir));
        }
    }
}

fn choice(value: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new("model"),
        value: ChoiceValue::Select {
            value: OptionValueId::new(value),
        },
    }
}
fn offered(value: &str) -> [SessionOption; 1] {
    [SessionOption {
        id: OptionId::new("model"),
        name: "Model".into(),
        description: None,
        category: OptionCategory::Model,
        kind: OptionKind::Select {
            current: OptionValueId::new("default"),
            values: ["default", value]
                .into_iter()
                .map(|value| OptionValue {
                    id: OptionValueId::new(value),
                    name: value.into(),
                    description: None,
                    group: None,
                })
                .collect(),
        },
    }]
}

#[test]
fn a_67_byte_offered_model_survives_the_shared_validator_writer_and_real_file_round_trip() {
    assert_eq!(MAX_CHOICE_BYTES, 256);
    assert_eq!(OFFERED_MODEL.len(), 67);
    let file = PreferenceFile::new("opaque-model-roundtrip").unwrap();
    let agent = HarnessRef::Known(vessel_proto::HarnessKind::ClaudeCode);
    let proposed = choice(OFFERED_MODEL);
    let prepared = prepare(&offered(OFFERED_MODEL), &[], proposed.clone()).unwrap();
    let mut prefs = Prefs::load_at(&file.0);
    prefs.keep_choices("laptop", &agent, prepared.as_slice());
    assert_eq!(
        prefs.choices("laptop", &agent),
        Some([proposed.clone()].as_slice())
    );
    prefs.save_at(&file.0).unwrap();
    let reloaded = Prefs::load_at(&file.0);
    assert_eq!(reloaded, prefs);
    assert_eq!(
        reloaded.choices("laptop", &agent),
        Some([proposed].as_slice())
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(&file.0).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn a_257_byte_offered_model_has_visible_refusal_words_and_keeps_the_prior_file() {
    let file = PreferenceFile::new("oversized-model-refusal").unwrap();
    let agent = HarnessRef::Known(vessel_proto::HarnessKind::ClaudeCode);
    let mut prefs = Prefs::load_at(&file.0);
    prefs.keep_choices("laptop", &agent, &[choice("large")]);
    prefs.save_at(&file.0).unwrap();
    let original = std::fs::read(&file.0).unwrap();
    let oversized = "m".repeat(257);
    let previous = prefs.choices("laptop", &agent).unwrap();
    // Settings calls this production preflight before touching its preferences
    // or sending SavePrefs. The generic writer's other behavior is unchanged.
    let problem = prepare(&offered(&oversized), previous, choice(&oversized)).unwrap_err();
    assert_eq!(problem, SaveProblem::Storage(prefs::TooMany { max: 8 }));
    assert_eq!(problem.variant(), vessel_app_view::alert::Variant::Error);
    let en = Translator::get(ENGLISH).unwrap().agent_defaults();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_defaults();
    assert_eq!(
        en.problem(problem),
        "This device cannot save the selected defaults. Choose another option."
    );
    assert_eq!(
        ko.problem(problem),
        "이 기기에는 선택한 기본값을 저장할 수 없습니다. 다른 옵션을 선택하세요."
    );
    assert_eq!(
        prefs.choices("laptop", &agent),
        Some([choice("large")].as_slice())
    );
    assert_eq!(std::fs::read(&file.0).unwrap(), original);
    assert_eq!(Prefs::load_at(&file.0), prefs);
}
