//! The actual narrow list must draw paths without reborrowing its active list.
use super::*;
use crate::areas::git::panel::Entry;
use crate::shell::language_tests;
use gpui::prelude::*;
use gpui::{AppContext as _, Render, TestAppContext};
use vessel_app_kit::ui::{Entity, IntoElement, size};
use vessel_app_view::layout::{HostAlias, LayoutKey};
use vessel_app_view::workspace::git::RequestKeys;
use vessel_app_view::workspace::git::panel::{Comparison, PanelReads, RepositoryKey};
use vessel_proto::workspace::{ChangeStatus, DiffSummary, FileChange, WorkspaceRoot};
use vessel_proto::{NodeId, ProjectId, SessionId};

struct ListProbe {
    shell: Entity<Shell>,
    address: PanelAddress,
}
impl Render for ListProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let address = self.address.clone();
        self.shell.update(cx, |shell, cx| {
            let list = shell
                .areas
                .git
                .panels
                .entries
                .get(&address)
                .unwrap()
                .files
                .clone();
            list.element(cx.processor(move |shell: &mut Shell, index, window, _cx| {
                let tokens = Tokens::of(vessel_app_kit::Scheme::Dark);
                div()
                    .w_full()
                    .debug_selector(|| "git-path-row".to_owned())
                    .child(file_row(shell, &address, index, &tokens, window))
                    .into_any_element()
            }))
        })
    }
}
fn model() -> Entry {
    let mut reads = PanelReads::new(
        RepositoryKey {
            node: NodeId::random(),
            root: WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
        },
        Comparison::default(),
    );
    let mut keys = RequestKeys::default();
    let ticket = reads.begin_summary(keys.issue().unwrap());
    assert!(reads.finish_summary(
        &ticket,
        Ok(DiffSummary {
            from: None,
            more: false,
            files: vec![FileChange {
                    path: format!("{}끝 파일.rs", "긴 경로/with spaces/".repeat(12))
                        .parse()
                        .unwrap(),
                    old_path: None,
                    status: ChangeStatus::Modified,
                    additions: Some(1),
                    deletions: Some(0),
                    binary: false,
                }]
        })
    ));
    let mut entry = Entry::new(PanelTarget::WorkingDiff, None);
    entry.reads = Some(reads);
    entry.files.reset(1);
    entry
}
#[gpui::test]
fn the_actual_narrow_list_draws_a_long_code_path_without_nested_list_borrows(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let address = PanelAddress {
            workspace: LayoutKey::Session {
                machine: HostAlias::parse("fixture").unwrap(),
                session: SessionId::random(),
            },
            tab: serde_json::from_str("1").unwrap(),
        };
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            let _old = shell
                .areas
                .git
                .panels
                .entries
                .insert(address.clone(), model());
            shell
        });
        ListProbe { shell, address }
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    cx.run_until_parked();
    let bounds = cx
        .debug_bounds("git-path-row")
        .expect("the real list drew its path row");
    let width = cx.update(|window, _| window.viewport_size().width);
    assert!(bounds.size.height > px(0.));
    assert!(bounds.size.width > px(0.));
    assert!(bounds.right() <= width);
    probe.read_with(cx, |probe, cx| {
        assert_eq!(
            probe
                .shell
                .read(cx)
                .areas
                .git
                .panels
                .entries
                .get(&probe.address)
                .unwrap()
                .files
                .rows(),
            1
        );
    });
}
