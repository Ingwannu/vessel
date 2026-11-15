use super::*;
use crate::shell::language_tests;

#[gpui::test]
fn a_language_change_keeps_both_form_values_carets_selections_and_focus(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let targets: [fn(&Shell) -> &TextField; 2] = [
        |shell| &shell.areas.workspace.page.path,
        |shell| &shell.areas.workspace.page.url,
    ];
    for target in targets {
        language_tests::preserves(
            cx,
            |shell, window, cx| {
                shell.open_add(AddForm::Clone, window, cx);
            },
            target,
            |shell| {
                assert_eq!(shell.areas.workspace.page.adding, Some(AddForm::Clone));
                assert!(!shell.areas.workspace.page.busy);
            },
        );
    }
}
