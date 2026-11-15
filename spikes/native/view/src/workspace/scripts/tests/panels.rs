use super::*;
use crate::workspace::git::{RequestKey, RequestKeys};

fn panel_surface(workspace: crate::layout::LayoutKey, instance: RequestKey) -> Surface {
    let mut layout = crate::layout::Layout::default();
    let tab = layout
        .open(
            crate::layout::Target::Scripts,
            crate::layout::OpenIntent::Reveal,
            crate::layout::Placement::Focused,
            crate::layout::AgentRole::Unknown,
            crate::layout::LayoutMode::SplitPanes,
        )
        .unwrap();
    Surface::Panel {
        workspace,
        tab,
        instance,
    }
}
#[test]
fn panel_bridge_uses_only_its_explicit_session_and_refuses_registry_or_other_alias() {
    let base = target();
    let session = SessionId::random();
    let instance = RequestKeys::default().issue().unwrap();
    let legacy = panel_surface(
        crate::layout::LayoutKey::Session {
            machine: crate::layout::HostAlias::parse("laptop").unwrap(),
            session,
        },
        instance,
    );
    let bridged = Target::legacy_panel(base.context.clone(), legacy).unwrap();
    assert_eq!(bridged.place, ScriptPlace::Session { session });
    let canonical = panel_surface(
        crate::layout::LayoutKey::Workspace {
            node: base.context.node(),
            workspace: vessel_proto::WorkspaceId::random(),
        },
        instance,
    );
    assert!(Target::legacy_panel(base.context.clone(), canonical).is_none());
    let different_alias = panel_surface(
        crate::layout::LayoutKey::Session {
            machine: crate::layout::HostAlias::parse("other").unwrap(),
            session,
        },
        instance,
    );
    assert!(Target::legacy_panel(base.context, different_alias).is_none());
}
#[test]
fn a_recreated_panel_rejects_receipts_even_when_tab_and_request_numbers_repeat() {
    let base = target();
    let owner = crate::layout::LayoutKey::Session {
        machine: crate::layout::HostAlias::parse("laptop").unwrap(),
        session: SessionId::random(),
    };
    let mut instances = RequestKeys::default();
    let old = Target::legacy_panel(
        base.context.clone(),
        panel_surface(owner.clone(), instances.issue().unwrap()),
    )
    .unwrap();
    let fresh = Target::legacy_panel(
        base.context.clone(),
        panel_surface(owner, instances.issue().unwrap()),
    )
    .unwrap();
    let mut old_model = Model::default();
    old_model.select(Some(old.clone()));
    let old_key = old_model.begin_read().unwrap();
    let mut new_model = Model::default();
    new_model.select(Some(fresh.clone()));
    let new_key = new_model.begin_read().unwrap();
    assert_eq!(old_key, new_key);
    assert!(!new_model.accept_read(&old, old_key, Ok(snapshot(&base))));
    assert!(new_model.accept_read(&fresh, new_key, Ok(snapshot(&base))));
}
#[test]
fn session_panels_cannot_draw_or_operate_other_sessions_runs_from_a_shared_root() {
    let base = target();
    let session = SessionId::random();
    let mut selected = base.clone();
    selected.place = ScriptPlace::Session { session };
    let mut mine = run(&base);
    mine.place = TerminalPlace::Session { session };
    let mut other = run(&base);
    other.place = TerminalPlace::Session {
        session: SessionId::random(),
    };
    let foreign = other.id;
    let mut data = snapshot(&base);
    data.scripts.runs = vec![other, mine.clone()];
    let mut model = Model::default();
    model.select(Some(selected.clone()));
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&selected, key, Ok(data)));
    assert_eq!(model.shown().unwrap().scripts.runs, vec![mine]);
    assert!(!model.permits(&Action::Stop(foreign)));
}

#[test]
fn refreshing_an_approved_command_does_not_retarget_an_already_drawn_run_button() {
    let (mut model, target) = ready();
    let old = model.action_ticket().unwrap();
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&target, key, Ok(snapshot(&target))));
    let action = Action::Run("dev".parse().unwrap());
    assert!(model.begin_write_at(&old, action.clone()).is_none());
    let fresh = model.action_ticket().unwrap();
    assert!(model.begin_write_at(&fresh, action).is_some());
}

#[test]
fn retained_runs_show_the_actual_directory_without_claiming_the_project_root() {
    let base = target();
    let mut retained = run(&base);
    retained.folder = "/project/actual-subfolder".into();
    retained.place = TerminalPlace::Session {
        session: SessionId::random(),
    };
    let entry = Entry::retained(&retained).unwrap();
    assert_eq!(
        entry.observed_dir,
        Some(std::path::Path::new("/project/actual-subfolder"))
    );
    let data = snapshot(&base);
    let crate::workspace::command_changes::Listed { scripts, .. } =
        crate::workspace::command_changes::Listed::of(&data.scripts.commands).unwrap();
    assert_eq!(Entry::of(&scripts[0]).observed_dir, None);
}
