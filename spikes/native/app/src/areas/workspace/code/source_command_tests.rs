//! Real drawn receipt capture and stale callback refusal; writes remain held.
//! Synthetic menu status does not stand in for physical status/write authority.
use super::*;
use vessel_app_view::workspace::pull::MenuEntry;

#[gpui::test]
fn refreshed_drawn_source_never_restamps_an_old_menu_callback(cx: &mut TestAppContext) {
    let (shell, cx, mut scene) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        let captured = crate::areas::git::commands::source_projection(shell, cx);
        let chosen = captured
            .entries
            .iter()
            .find(|entry| matches!(entry, MenuEntry::Git(_)))
            .unwrap()
            .clone();
        let same = shell.git_command_entries().unwrap();
        assert_eq!(captured.target, same.target);
        shell.reread_changes();
        assert!(shell.git_command_entries().is_none());
        assert!(!shell.run_git_command(&captured.target, &chosen, window, cx));
        assert!(matches!(
            scene.request(shell.fixture_link()).unwrap(),
            Call::GitChanges(_)
        ));
        ready(&mut scene, shell);
        let successor = shell.git_command_entries().unwrap();
        assert_ne!(captured.target, successor.target);
        assert!(!shell.run_git_command(&captured.target, &chosen, window, cx));
        assert!(!shell.areas.git.writing.busy());
        assert!(
            scene
                .transcript(shell.fixture_link())
                .unwrap()
                .iter()
                .all(|call| { matches!(call, Call::GetSession(_) | Call::GitChanges(_)) })
        );
        scene.close(shell.fixture_link()).unwrap();
    });
}
