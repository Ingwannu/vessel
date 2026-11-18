//! The real copy/language path keeps clipboard data and input ownership; a copy
//! says so in Paseo's copied toast, not the frame notice (TRACKER 804).
use super::PreparedGeneral;
use crate::shell::language_tests;
use vessel_app_view::localization::{ENGLISH, Locale, Translator};
use vessel_app_view::prefs::{Kept, TurnDelivery};
use vessel_app_view::toasts::ToastTone;

const PATH: &str = "/isolated/prefs.json";

#[test]
fn refreshing_changed_storage_facts_replaces_the_prepared_hint() {
    let words = Translator::get(ENGLISH).unwrap();
    let mut prepared = PreparedGeneral::new(words, &Kept::default());
    assert_eq!(
        prepared.hint.as_ref(),
        "Kept until Vessel quits: this computer gives Vessel no place to keep them."
    );
    prepared.refresh(
        words,
        &Kept::File {
            path: PATH.to_owned(),
        },
    );
    assert_eq!(
        prepared.hint.as_ref(),
        "Kept by Vessel on this computer, so every launch opens with them.\n\u{2068}/isolated/prefs.json\u{2069}"
    );
}

#[gpui::test]
fn real_language_dispatch_refreshes_hint_without_changing_input_or_copy(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, _window, cx| {
            shell.prefs.kept = Kept::File {
                path: PATH.to_owned(),
            };
            shell.refresh_general_words();
            assert_delivery_labels_borrow_cached_text(shell);
            shell.copy_preferences_path(PATH, cx);
            let said = shell
                .areas
                .toasts
                .shown()
                .map(|toast| (toast.text.as_str(), toast.tone));
            assert_eq!(said, Some(("Copied Preferences path", ToastTone::Copied)));
            assert_eq!(shell.notice, None);
            assert_eq!(
                shell.areas.settings.general.hint.as_ref(),
                "Kept by Vessel on this computer, so every launch opens with them.\n\u{2068}/isolated/prefs.json\u{2069}"
            );
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            assert_delivery_labels_borrow_cached_text(shell);
            assert_eq!(
                shell.areas.settings.general.hint.as_ref(),
                "Vessel이 이 컴퓨터에 보관하므로, 실행할 때마다 그대로 열립니다.\n\u{2068}/isolated/prefs.json\u{2069}"
            );
            assert_eq!(
                shell.prefs.kept,
                Kept::File {
                    path: PATH.to_owned()
                }
            );
        },
    );
    cx.update(|cx| {
        assert!(
            cx.read_from_clipboard()
                .is_some_and(|item| item.text().as_deref() == Some(PATH))
        );
    });
}

#[test]
fn changed_language_uses_the_prepared_safe_position_fallback() {
    use vessel_app_view::files::{FileFault, FileProblem};
    use vessel_app_view::prefs::LaunchOnly;
    if let Ok(max) = usize::try_from(u32::MAX)
        && let Some(line) = max.checked_add(1)
    {
        let kept = Kept::ThisLaunch {
            why: LaunchOnly::Unreadable {
                fault: FileFault {
                    path: PATH.to_owned(),
                    at: Some((line, 1)),
                    problem: FileProblem::NotJson,
                    field: None,
                },
            },
        };
        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        let mut prepared = PreparedGeneral::new(en, &kept);
        assert_eq!(
            prepared.hint.as_ref(),
            "Preference storage information is unavailable."
        );
        prepared.refresh(ko, &kept, &mut None);
        assert_eq!(
            prepared.hint.as_ref(),
            "환경설정 보관 정보를 표시할 수 없습니다."
        );
    }
}

fn assert_delivery_labels_borrow_cached_text(shell: &crate::shell::Shell) {
    use crate::areas::settings::choices::{self, List};
    use vessel_app_kit::{Scheme, Tokens};
    let chosen = shell
        .words
        .general()
        .delivery(shell.prefs.settings.composer.delivery);
    assert_eq!(choices::delivery_label(shell).as_ptr(), chosen.as_ptr());
    let rows = choices::rows(List::Delivery, shell, &Tokens::of(Scheme::Dark));
    assert_eq!(
        rows.len(),
        [
            TurnDelivery::Interrupt,
            TurnDelivery::Steer,
            TurnDelivery::Queue
        ]
        .len()
    );
    for (row, delivery) in rows.iter().zip([
        TurnDelivery::Interrupt,
        TurnDelivery::Steer,
        TurnDelivery::Queue,
    ]) {
        assert_eq!(
            row.label.as_ptr(),
            shell.words.general().delivery(delivery).as_ptr()
        );
    }
}
