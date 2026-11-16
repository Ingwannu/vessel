//! Import is a normal registered root action, not a second modal controller.
use super::*;

#[gpui::test]
fn import_registration_retires_old_rows_on_close_and_caption_refresh(cx: &mut TestAppContext) {
    let (view, cx) = harness(cx);
    ready(cx);
    update(&view, cx, |shell, window, cx| {
        let projection = snapshot::project(shell);
        let old = projection
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .find(|row| {
                matches!(&row.id, snapshot::Identity::Registered(id)
                if id.source.as_str() == "native-import" && id.entry.as_str() == "import-session")
            })
            .unwrap()
            .activate
            .clone();
        let Activate::Registered(id, owner) = old.clone() else {
            panic!("Import must use the shared source registry");
        };
        assert!(shell.areas.palette.sources.resolve(&id, &owner).is_some());
        shell.close_palette(window, cx);
        assert!(shell.areas.palette.sources.resolve(&id, &owner).is_none());
        shell.open_palette(window, cx);
        let fresh = shell.areas.palette.root_import.as_ref().unwrap().clone();
        assert!(shell.areas.palette.sources.resolve(&id, &fresh).is_some());
        shell.activate_command(old, window, cx);
        assert!(
            shell.areas.palette.is_open(),
            "an old row cannot close the fresh center"
        );
        shell.refresh_command_sources(window, cx);
        assert!(shell.areas.palette.sources.resolve(&id, &fresh).is_none());
        let refreshed = shell.areas.palette.root_import.as_ref().unwrap();
        assert!(
            shell
                .areas
                .palette
                .sources
                .resolve(&id, refreshed)
                .is_some()
        );
    });
    ready(cx);
    assert!(
        cx.debug_bounds("history-cancel").is_none(),
        "no retired callback opened Import"
    );
}
