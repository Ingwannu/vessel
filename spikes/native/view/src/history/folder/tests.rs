//! Paseo's directory labels over the host's projects.
use super::folder_label;
use chrono::Utc;
use std::path::{Path, PathBuf};
use vessel_proto::workspace::{ProjectInfo, ProjectState};

fn project(id: u8, name: &str, root: &str) -> ProjectInfo {
    ProjectInfo {
        id: format!("00000000-0000-4000-8000-0000000000{id:02}")
            .parse()
            .unwrap(),
        name: name.parse().unwrap(),
        root: PathBuf::from(root),
        git: true,
        default_branch: None,
        state: ProjectState::Ready,
        created_at: Utc::now(),
        revision: 1,
    }
}

fn label(cwd: &str) -> String {
    let projects = [
        project(1, "shop", "/work/shop"),
        project(2, "atlas", "/work/atlas/"),
        project(3, "ui-kit", "/work/atlas/packages/ui"),
    ];
    folder_label(Path::new(cwd), &projects)
}

#[test]
fn a_folder_inside_a_project_names_the_project_and_the_rest() {
    assert_eq!(label("/work/shop/src/app"), "shop \u{b7} src/app");
}

#[test]
fn a_project_root_reads_as_its_name_with_or_without_a_trailing_slash() {
    assert_eq!(label("/work/shop"), "shop");
    assert_eq!(label("/work/shop/"), "shop");
    assert_eq!(label("/work/atlas"), "atlas");
}

#[test]
fn the_deepest_project_wins() {
    assert_eq!(label("/work/atlas/packages/ui/src"), "ui-kit \u{b7} src");
    assert_eq!(label("/work/atlas/docs"), "atlas \u{b7} docs");
}

#[test]
fn any_other_folder_keeps_its_full_path() {
    assert_eq!(label("/work/shop2/src"), "/work/shop2/src");
    assert_eq!(label("/elsewhere"), "/elsewhere");
    assert_eq!(folder_label(Path::new("/work/shop/x"), &[]), "/work/shop/x");
}
