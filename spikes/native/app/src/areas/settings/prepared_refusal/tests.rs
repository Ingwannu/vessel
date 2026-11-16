//! Exercise the actual rejected-change path, then the actual language dispatch.
use crate::shell::language_tests;
use vessel_app_view::prefs::{
    ActionId, Bindings, MAX_ACTIONS, MAX_MACHINES, MachineDefaults, PreferenceChange, SendKey,
    ShortcutEdit, ShortcutTarget,
};
use vessel_proto::NodeId;

#[gpui::test]
fn a_machine_limit_refusal_keeps_its_cause_and_changes_language_without_losing_input(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            for _ in 0..MAX_MACHINES {
                let _previous = shell
                    .prefs
                    .settings
                    .defaults
                    .insert(NodeId::random(), MachineDefaults::default());
            }
            let before = shell.prefs.settings.clone();
            shell.prefer(
                &PreferenceChange::DefaultAgent {
                    node: NodeId::random(),
                    agent: None,
                },
                window,
                cx,
            );
            assert_eq!(shell.prefs.settings, before);
            assert_eq!(
                shell
                    .areas
                    .settings
                    .refused
                    .as_ref()
                    .unwrap()
                    .words
                    .as_ref(),
                "That change was not kept. Defaults can be kept for at most \u{2068}32\u{2069} machines."
            );
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            assert_eq!(
                shell
                    .areas
                    .settings
                    .refused
                    .as_ref()
                    .unwrap()
                    .words
                    .as_ref(),
                "그 변경은 보관되지 않았습니다. 기본값은 최대 \u{2068}32\u{2069}대의 머신에 대해서만 보관할 수 있습니다."
            );
            assert_eq!(shell.prefs.settings.defaults.len(), MAX_MACHINES);
        },
    );
}

#[gpui::test]
fn an_action_limit_refusal_is_cleared_by_the_same_success_path_as_before(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            for index in 0..MAX_ACTIONS {
                let _previous = shell.prefs.settings.shortcuts.insert(
                    format!("a{index}").parse::<ActionId>().unwrap(),
                    Bindings::default(),
                );
            }
            let before = shell.prefs.settings.clone();
            let change = PreferenceChange::Shortcuts {
                edits: vec![ShortcutEdit {
                    action: "extra".parse().unwrap(),
                    to: ShortcutTarget::Bindings {
                        keys: Bindings::default(),
                    },
                }]
                .try_into()
                .unwrap(),
            };
            shell.prefer(&change, window, cx);
            assert_eq!(shell.prefs.settings, before);
            assert_eq!(
                shell
                    .areas
                    .settings
                    .refused
                    .as_ref()
                    .unwrap()
                    .words
                    .as_ref(),
                "That change was not kept. Keys can be kept for at most \u{2068}64\u{2069} actions."
            );
            shell.prefer(&PreferenceChange::Send(SendKey::ModEnter), window, cx);
            assert!(shell.areas.settings.refused.is_none());
        },
        |shell| &shell.areas.session.composer,
        |shell| assert!(shell.areas.settings.refused.is_none()),
    );
}

#[gpui::test]
fn a_sidebar_limit_refusal_reprepares_its_retained_cause_when_language_changes(
    cx: &mut gpui::TestAppContext,
) {
    use super::Reason;
    use vessel_app_view::sidebar::navigation::TooMany;
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, cx| {
            let before = shell.prefs.clone();
            shell.sidebar_navigation_refused(TooMany, cx);
            assert_eq!(shell.prefs, before);
            let refused = shell.areas.settings.refused.as_ref().unwrap();
            assert_eq!(refused.reason, Reason::SidebarNavigation(TooMany));
            assert_eq!(
                refused.words.as_ref(),
                "The sidebar settings could not be saved because the item limit was reached."
            );
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            let refused = shell.areas.settings.refused.as_ref().unwrap();
            assert_eq!(refused.reason, Reason::SidebarNavigation(TooMany));
            assert_eq!(
                refused.words.as_ref(),
                "항목 수가 한도에 도달해 사이드바 설정을 저장하지 못했습니다."
            );
        },
    );
}
