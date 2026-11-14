//! ProviderRow/StatusIndicator from Paseo's
//! `screens/settings/providers-section.tsx`: labels/counts on wide rows,
//! just the marker on compact rows; only known sign-out changes the row.
use super::*;
use serde_json::{Value, json};
use vessel_proto::{HarnessKind, OptionCategory, OptionId, OptionKind, OptionValue, OptionValueId};

fn found() -> Installed {
    serde_json::from_value(
        json!({"state":"found","path":"/opt/agent","structured":{"state":"available"}}),
    )
    .unwrap()
}
fn models(count: usize) -> Vec<SessionOption> {
    vec![SessionOption {
        id: OptionId::new("model"),
        name: "Model".into(),
        description: None,
        category: OptionCategory::Model,
        kind: OptionKind::Select {
            current: OptionValueId::new("m0"),
            values: (0..count)
                .map(|i| OptionValue {
                    id: OptionValueId::new(format!("m{i}")),
                    name: format!("Model {i}"),
                    description: None,
                    group: None,
                })
                .collect(),
        },
    }]
}
fn project(
    lang: Lang,
    installed: &Installed,
    signed_in: Option<&SignedIn>,
    options: Option<&[SessionOption]>,
    activity: Activity,
) -> Row {
    row(
        lang,
        Facts {
            agent: &HarnessRef::Known(HarnessKind::Codex),
            installed,
            signed_in,
            options,
            activity,
        },
    )
}
fn cases() -> Vec<(Value, &'static str, &'static str, Variant)> {
    let mut cases = vec![
        (
            json!({"state":"found","path":"/opt/agent","structured":{"state":"available"}}),
            "Available",
            "사용 가능",
            Variant::Success,
        ),
        (
            json!({"state":"found","path":"/opt/agent","structured":{"state":"unavailable","reason":"machine detail"}}),
            "Needs attention",
            "확인 필요",
            Variant::Warning,
        ),
        (
            json!({"state":"missing","program":"example"}),
            "Not installed",
            "미설치",
            Variant::Warning,
        ),
        (
            json!({"state":"too_old","path":"/opt/agent","speaks":0,"needs":1}),
            "Needs attention",
            "확인 필요",
            Variant::Warning,
        ),
        (
            json!({"state":"too_new","path":"/opt/agent","speaks":2,"needs":1}),
            "Needs attention",
            "확인 필요",
            Variant::Warning,
        ),
    ];
    for reason in [
        "not_installed",
        "not_checked",
        "in_use",
        "not_said",
        "no_home",
        "unread",
    ] {
        cases.push((
            json!({"state":"unknown","reason":{"reason":reason}}),
            "Not known",
            "확인 전",
            Variant::Muted,
        ));
    }
    for step in ["spawn", "initialize", "session_new", "exited"] {
        cases.push((
            json!({"state":"unknown","reason":{"reason":"failed","step":step}}),
            "Error",
            "오류",
            Variant::Error,
        ));
    }
    cases.push((
        json!({"state":"unknown","reason":{"reason":"timed_out","after":30000}}),
        "Error",
        "오류",
        Variant::Error,
    ));
    cases
}
#[test]
fn every_installation_state_has_the_reference_indicator_and_only_errors_have_a_hint() {
    for (state, en, ko, tone) in cases() {
        let installed = serde_json::from_value(state).unwrap();
        for (lang, label, error) in [
            (Lang::En, en, "Could not check this agent."),
            (Lang::Ko, ko, "에이전트를 확인하지 못했습니다."),
        ] {
            let row = project(lang, &installed, None, None, Activity::Idle);
            assert_eq!(
                row.indicator(lang, Density::Expanded),
                Indicator::Expanded {
                    tone,
                    marker: Marker::Dot,
                    label,
                    models: None
                }
            );
            assert_eq!(
                row.indicator(lang, Density::Compact),
                Indicator::Compact {
                    tone,
                    marker: Marker::Dot
                }
            );
            assert_eq!(row.error(Density::Compact), None);
            assert_eq!(
                row.error(Density::Expanded),
                if tone == Variant::Error {
                    Some(error)
                } else {
                    None
                }
            );
        }
    }
}
#[test]
fn credentials_and_unknown_authentication_checks_stay_in_the_detail_page() {
    let installed = found();
    let options = models(3);
    for lang in [Lang::En, Lang::Ko] {
        for status in [json!({"state":"yes"}), json!({"state":"not_required"})] {
            let signed_in = serde_json::from_value(status).unwrap();
            assert!(matches!(
                project(
                    lang,
                    &installed,
                    Some(&signed_in),
                    Some(&options),
                    Activity::Idle
                ),
                Row::Available { models: Some(_) }
            ));
        }
        for reason in [
            "not_installed",
            "not_checked",
            "in_use",
            "not_said",
            "no_home",
            "unread",
        ] {
            let signed_in =
                serde_json::from_value(json!({"state":"unknown","reason":{"reason":reason}}))
                    .unwrap();
            assert!(matches!(
                project(
                    lang,
                    &installed,
                    Some(&signed_in),
                    Some(&options),
                    Activity::Idle
                ),
                Row::Available { models: Some(_) }
            ));
        }
        for step in ["spawn", "initialize", "session_new", "exited"] {
            let signed_in = serde_json::from_value(
                json!({"state":"unknown","reason":{"reason":"failed","step":step}}),
            )
            .unwrap();
            let row = project(
                lang,
                &installed,
                Some(&signed_in),
                Some(&options),
                Activity::Idle,
            );
            assert!(matches!(row, Row::Available { models: Some(_) }));
            assert_eq!(row.error(Density::Expanded), None);
        }
        let timed_out = serde_json::from_value(
            json!({"state":"unknown","reason":{"reason":"timed_out","after":30000}}),
        )
        .unwrap();
        assert!(matches!(
            project(
                lang,
                &installed,
                Some(&timed_out),
                Some(&options),
                Activity::Idle
            ),
            Row::Available { models: Some(_) }
        ));
    }
}
#[test]
fn known_sign_out_is_warning_has_no_model_count_and_never_overrides_a_missing_program() {
    let installed = found();
    let options = models(3);
    for lang in [Lang::En, Lang::Ko] {
        let signed_in =
            serde_json::from_value(json!({"state":"no","help":{"methods":[]}})).unwrap();
        let row = project(
            lang,
            &installed,
            Some(&signed_in),
            Some(&options),
            Activity::Idle,
        );
        assert_eq!(row, Row::SignedOut);
        assert_eq!(
            row.indicator(lang, Density::Expanded),
            Indicator::Expanded {
                tone: Variant::Warning,
                marker: Marker::Dot,
                label: if lang == Lang::En {
                    "Signed out"
                } else {
                    "로그아웃 상태"
                },
                models: None,
            }
        );
        let missing =
            serde_json::from_value(json!({"state":"missing","program":"example"})).unwrap();
        assert_eq!(
            project(
                lang,
                &missing,
                Some(&signed_in),
                Some(&options),
                Activity::Idle
            ),
            Row::NotInstalled
        );
    }
}
#[test]
fn positive_known_counts_use_the_same_words_as_the_model_selector_and_compact_omits_them() {
    // Paseo providers-section.test.tsx row-composition fixture: 3 models.
    let installed = found();
    for (count, en, ko) in [
        (1, "1 model", "모델 1개"),
        (3, "3 models", "모델 3개"),
        (12, "12 models", "모델 12개"),
    ] {
        let options = models(count);
        for (lang, text, label) in [(Lang::En, en, "Available"), (Lang::Ko, ko, "사용 가능")] {
            let row = project(lang, &installed, None, Some(&options), Activity::Idle);
            assert_eq!(
                row.indicator(lang, Density::Expanded),
                Indicator::Expanded {
                    tone: Variant::Success,
                    marker: Marker::Dot,
                    label,
                    models: Some(text.into())
                }
            );
            assert_eq!(
                row.indicator(lang, Density::Compact),
                Indicator::Compact {
                    tone: Variant::Success,
                    marker: Marker::Dot
                }
            );
        }
    }
    for options in [None, Some(&[][..]), Some(models(0).as_slice())] {
        assert_eq!(
            project(Lang::En, &installed, None, options, Activity::Idle),
            Row::Available { models: None }
        );
    }
}
#[test]
fn loading_is_neutral_and_has_the_activity_marker_in_both_densities() {
    let installed = found();
    let options = models(2);
    for (lang, label) in [(Lang::En, "Loading"), (Lang::Ko, "불러오는 중")] {
        let row = project(lang, &installed, None, Some(&options), Activity::Loading);
        assert_eq!(row, Row::Loading);
        assert_eq!(
            row.indicator(lang, Density::Expanded),
            Indicator::Expanded {
                tone: Variant::Muted,
                marker: Marker::Spinner,
                label,
                models: None
            }
        );
        assert_eq!(
            row.indicator(lang, Density::Compact),
            Indicator::Compact {
                tone: Variant::Muted,
                marker: Marker::Spinner
            }
        );
    }
}
#[test]
fn activity_is_scoped_to_the_requested_agent_except_for_a_list_refresh() {
    use vessel_proto::agents::{InstallAgent, InstallPart};
    let codex = HarnessRef::Known(HarnessKind::Codex);
    let added = HarnessRef::Added("example".parse().unwrap());
    assert_eq!(Activity::for_agent(&codex, None), Activity::Idle);
    assert_eq!(
        Activity::for_agent(&added, Some(&Operation::Discovery)),
        Activity::Loading
    );
    assert_eq!(
        Activity::for_agent(&codex, Some(&Operation::Check(added.clone()))),
        Activity::Idle
    );
    assert_eq!(
        Activity::for_agent(&added, Some(&Operation::Check(added.clone()))),
        Activity::Loading
    );
    let install = Operation::Install(InstallAgent {
        agent: HarnessKind::Codex,
        part: InstallPart::Adapter,
    });
    assert_eq!(
        Activity::for_agent(&codex, Some(&install)),
        Activity::Loading
    );
    assert_eq!(Activity::for_agent(&added, Some(&install)), Activity::Idle);
}
#[test]
fn section_heading_names_the_machine_without_a_changing_korean_particle() {
    assert_eq!(
        Lang::En.agents().row_section("long / 머신"),
        "Providers on long / 머신"
    );
    assert_eq!(
        Lang::Ko.agents().row_section("long / 머신"),
        "long / 머신의 프로바이더"
    );
}
