//! The real display picker rewords its trigger and rows without changing the
//! chosen preference, the selected row, or a live composer input.
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};
use vessel_app_view::prefs::ConversationDisplay;

use super::{List, label, rows};
use crate::shell::language_tests;

#[gpui::test]
fn display_choices_reword_while_the_preference_and_input_stay(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            shell.prefs.settings.appearance.conversation = ConversationDisplay::CollapseTools;
            assert_eq!(label(List::Display, shell), "Collapse tool calls");
            let items = rows(List::Display, shell, &Tokens::of(Scheme::Dark));
            let actual: Vec<_> = items
                .iter()
                .map(|row| (row.label.as_ref(), row.checked))
                .collect();
            assert_eq!(actual, [("Full", false), ("Collapse tool calls", true)]);
            // Legacy table labels change for Korean but cannot expose an
            // untranslated picker in pseudo mode. Exercise the real dispatcher.
            let pseudo = Translator::get(resolve_language(
                LanguagePreference::System,
                None,
                Some("en-XA"),
            ))
            .unwrap();
            shell.apply_language(pseudo, window, cx);
            assert_ne!(label(List::Display, shell), "Collapse tool calls");
            let pseudo_rows = rows(List::Display, shell, &Tokens::of(Scheme::Dark));
            assert!(pseudo_rows.iter().all(|row| {
                row.label.as_ref() != "Full" && row.label.as_ref() != "Collapse tool calls"
            }));
            shell.apply_language(Translator::get(ENGLISH).unwrap(), window, cx);
        },
        |shell| &shell.areas.session.composer,
        |shell| {
            assert_eq!(
                shell.prefs.settings.appearance.conversation,
                ConversationDisplay::CollapseTools
            );
            assert_eq!(label(List::Display, shell), "도구 호출 접기");
            let items = rows(List::Display, shell, &Tokens::of(Scheme::Dark));
            let actual: Vec<_> = items
                .iter()
                .map(|row| (row.label.as_ref(), row.checked))
                .collect();
            assert_eq!(actual, [("전체", false), ("도구 호출 접기", true)]);
        },
    );
}
