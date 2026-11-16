//! A stale menu fallback must not programmatically edit a locked retained field.
use super::*;
use crate::{areas::session::Open, shell::language_tests};
use gpui::TestAppContext;
use vessel_app_kit::Editability;
use vessel_app_view::composer::recall::{Arrow, At};
use vessel_app_view::composer_shell::surface::{Access, Mode, Overlay, SubmitPolicy, Surface};
use vessel_app_view::layout::{HostAlias, LayoutKey, OpenIntent, Placement};
use vessel_proto::SessionId;

#[derive(Clone, Copy)]
enum Block {
    ReadOnly,
    Voice,
    Terminal,
}

fn session(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) -> DraftKey {
    let session = SessionId::random();
    shell.route = Route::Session;
    shell.areas.session.open = Some(Open {
        machine: "fixture".to_owned(),
        session,
        title: String::new(),
        agent: String::new(),
        structured: true,
        live: true,
        archived_at: None,
        binding: None,
    });
    let _mounted_tab = shell
        .layout_open_target(
            &LayoutKey::Session {
                machine: HostAlias::parse("fixture").unwrap(),
                session,
            },
            vessel_app_view::layout::Target::Agent(session),
            OpenIntent::Reveal,
            Placement::Focused,
            window,
            cx,
        )
        .unwrap();
    DraftKey::Session { session }
}

#[gpui::test]
fn recall_fallback_preserves_readonly_covered_and_terminal_input(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let target = session(shell, window, cx);
        let field = shell.areas.session.composer.clone();
        for block in [Block::ReadOnly, Block::Voice, Block::Terminal] {
            let recalled = shell.areas.composer.recall.press(
                Arrow::Up,
                At {
                    text: "",
                    selection: (0, 0),
                },
                || vec!["remembered"],
            );
            field.replace_all(&recalled.unwrap(), window, cx);
            match block {
                Block::ReadOnly => field.set_editability(Editability::ReadOnly, cx),
                Block::Voice => shell
                    .areas
                    .composer_shell
                    .surface_mut(target)
                    .cover(Overlay::Voice),
                Block::Terminal => {
                    *shell.areas.composer_shell.surface_mut(target) =
                        Surface::new(Mode::Terminal, Access::Editable, SubmitPolicy::Clear);
                }
            }
            let before = (field.text(cx), field.selection(cx));
            let recall = shell.areas.composer.recall.clone();
            assert_eq!(
                shell.slash_key(Over::Session, ComposerKey::Down, window, cx),
                KeyAnswer::Pass
            );
            assert_eq!((field.text(cx), field.selection(cx)), before);
            assert_eq!(shell.areas.composer.recall, recall);
            field.set_editability(Editability::Editable, cx);
            *shell.areas.composer_shell.surface_mut(target) = Surface::default();
            assert_eq!(
                shell.slash_key(Over::Session, ComposerKey::Down, window, cx),
                KeyAnswer::Taken
            );
            assert_eq!(field.text(cx), "");
        }
    });
}
