//! Reference target cases plus stale/missing identity and remote-platform edges.
use super::*;
use chrono::Utc;
use vessel_proto::{SessionId, automation::AfterRun};
fn chat() -> Card {
    Card {
        session: "00000000-0000-4000-8000-000000000002".parse().unwrap(),
        structured: true,
        live: true,
        title: "  Fix build  ".into(),
        machine: "laptop".into(),
        agent: "Codex".into(),
        lane: None,
        phrase: crate::Phrase::Running,
        attention: crate::Attention::AtRest,
        at: chrono::DateTime::<Utc>::UNIX_EPOCH,
        detail: crate::board::CardDetail {
            titled: true,
            harness: Some(HarnessRef::Known(vessel_proto::HarnessKind::Codex)),
            ..Default::default()
        },
    }
}
#[test]
fn reference_session_title_blank_title_and_missing_provider_cases() {
    let mut chat = chat();
    let target = ScheduleTarget::Session {
        session: chat.session,
    };
    assert_eq!(
        resolve(&target, Some(&chat), None, None),
        Target {
            line: Line::Chat("Fix build"),
            agent: chat.detail.harness.as_ref()
        }
    );
    chat.title = "  ".into();
    assert_eq!(
        resolve(&target, Some(&chat), None, None).line,
        Line::UntitledChat
    );
    chat.title = "folder fallback".into();
    chat.detail.titled = false;
    assert_eq!(
        resolve(&target, Some(&chat), None, None).line,
        Line::UntitledChat
    );
    assert_eq!(
        resolve(&target, None, None, None),
        Target {
            line: Line::Unavailable(Missing::Chat),
            agent: None
        }
    );
    let other: SessionId = "00000000-0000-4000-8000-000000000009".parse().unwrap();
    assert_eq!(
        resolve(
            &ScheduleTarget::Session { session: other },
            Some(&chat),
            None,
            None
        )
        .line,
        Line::Unavailable(Missing::Chat)
    );
}
#[test]
fn a_missing_project_cache_preserves_the_node_path_without_claiming_deletion() {
    let spec =
        serde_json::from_str(r#"{"harness":"codex","mode":"structured","cwd":"C:\\work\\api"}"#)
            .unwrap();
    let target = ScheduleTarget::NewSession {
        spec,
        after: AfterRun::Archive,
    };
    assert_eq!(
        resolve(&target, None, None, None).line,
        Line::Path(Path::new(r"C:\work\api"))
    );
    assert_eq!(
        display_path(Path::new(r"C:\Users\sam\api")),
        r"C:\Users\sam\api"
    );
    assert_eq!(
        display_path(Path::new(r"\\node\share\api")),
        r"\\node\share\api"
    );
}
#[test]
fn reference_unmatched_home_paths_are_shortened_as_display_text_only() {
    // schedule-derivation.test.ts and schedule-project-targets.test.ts inputs.
    for (path, expected) in [
        ("/Users/alex/work/api", "~/work/api"),
        ("/Users/sam/api", "~/api"),
        ("/home/sam/api", "~/api"),
        ("/tmp/project", "/tmp/project"),
        ("/home/sam", "~"),
        ("/home/sam/", "~/"),
        ("/Users//work", "/Users//work"),
    ] {
        assert_eq!(display_path(Path::new(path)), expected);
    }
}

fn project() -> ProjectInfo {
    serde_json::from_str(r#"{"id":"00000000-0000-4000-8000-000000000003","name":"My Project","root":"/tmp/project","git":true,"state":{"state":"ready"},"created_at":"2026-01-01T00:00:00Z","revision":1}"#).unwrap()
}
#[test]
fn reference_matched_root_prefers_project_name_and_missing_explicit_ids_are_truthful() {
    let projects = [project()];
    let spec =
        serde_json::from_str(r#"{"harness":"codex","mode":"structured","cwd":"/tmp/project"}"#)
            .unwrap();
    let target = ScheduleTarget::NewSession {
        spec,
        after: AfterRun::Archive,
    };
    assert_eq!(
        resolve(&target, None, Some(&projects), None).line,
        Line::Project {
            name: &projects[0].name,
            dir: None
        }
    );
    let spec = serde_json::from_str(r#"{"harness":"codex","mode":"structured","place":{"kind":"project","project":"00000000-0000-4000-8000-000000000003","dir":"한글/long-subfolder"}}"#).unwrap();
    let target = ScheduleTarget::NewSession {
        spec,
        after: AfterRun::Archive,
    };
    let expected_dir: RelPath = "한글/long-subfolder".parse().unwrap();
    assert_eq!(
        resolve(&target, None, Some(&projects), None).line,
        Line::Project {
            name: &projects[0].name,
            dir: Some(&expected_dir)
        }
    );
    assert_eq!(
        resolve(&target, None, None, None).line,
        Line::Unavailable(Missing::Project)
    );
    let spec = serde_json::from_str(r#"{"harness":"codex","mode":"structured","place":{"kind":"worktree","worktree":"00000000-0000-4000-8000-000000000004"}}"#).unwrap();
    let target = ScheduleTarget::NewSession {
        spec,
        after: AfterRun::Archive,
    };
    assert_eq!(
        resolve(&target, None, None, None).line,
        Line::Unavailable(Missing::Worktree)
    );
}
#[test]
fn worktree_node_path_and_default_folder_never_use_the_viewers_home_or_platform() {
    let trees:Vec<WorktreeInfo>=serde_json::from_str(r#"[{"id":"00000000-0000-4000-8000-000000000004","project":"00000000-0000-4000-8000-000000000003","path":"C:\\work\\api","created_at":"2026-01-01T00:00:00Z","sessions":[],"setup":{"state":"none"}}]"#).unwrap();
    let spec = serde_json::from_str(r#"{"harness":"codex","mode":"structured","place":{"kind":"worktree","worktree":"00000000-0000-4000-8000-000000000004"}}"#).unwrap();
    let target = ScheduleTarget::NewSession {
        spec,
        after: AfterRun::Keep,
    };
    assert_eq!(
        resolve(&target, None, None, Some(&trees)).line,
        Line::Path(Path::new(r"C:\work\api"))
    );
    let spec = serde_json::from_str(r#"{"harness":"codex","mode":"structured"}"#).unwrap();
    let target = ScheduleTarget::NewSession {
        spec,
        after: AfterRun::Archive,
    };
    assert_eq!(resolve(&target, None, None, None).line, Line::DefaultFolder);
    assert_eq!(
        resolve(&ScheduleTarget::Unknown, None, None, None),
        Target {
            line: Line::Unknown,
            agent: None
        }
    );
}
