//! Markdown paths are relative to the captured document, never selected globals.
//! Lexical normalization grants no disk authority: the node rechecks links/root.

use vessel_app_kit::ui::Context;
use vessel_app_view::composer_shell::links::{Link, Origin, Workspace, classify};
use vessel_app_view::workspace::file_location::FileLocation;
use vessel_proto::workspace::RelPath;

use super::super::file_open::FileOpenContext;
use super::{FileKey, Mode};
use crate::shell::Shell;

impl Shell {
    fn markdown_link_is_current(&self, key: &FileKey, origin: &FileOpenContext) -> bool {
        let panel = &self.areas.workspace.code;
        self.file_tree_scope_is_current(origin)
            && panel.file_key().as_ref() == Some(key)
            && panel.files.viewer.as_ref().is_some_and(|viewer| {
                viewer.scope == *origin.scope()
                    && viewer
                        .markdown
                        .as_ref()
                        .is_some_and(|preview| preview.mode == Mode::Preview)
            })
    }
    pub(super) fn markdown_link(
        &mut self,
        key: &FileKey,
        origin: &FileOpenContext,
        href: &str,
        cx: &mut Context<'_, Self>,
    ) {
        // Buffer identity deliberately excludes connection epochs. A click
        // also carries its original admission: reconnecting the same file
        // cannot silently turn an old rendered link into new-connection I/O.
        if !self.markdown_link_is_current(key, origin) {
            return;
        }
        match classify(href, Origin::Href, Workspace::Known) {
            Some(Link::External(url)) => cx.open_url(url.as_str()),
            Some(Link::File(target) | Link::Lookup(target)) => {
                let path = relative_path(&key.path, target.path.as_str());
                let location = target
                    .lines
                    .and_then(|lines| FileLocation::lines(lines.start(), lines.end()).ok());
                if let Some(path) = path {
                    self.open_file_location(path, location, cx);
                } else {
                    self.areas.workspace.code.files.actions.refused =
                        Some(self.words().labels().file_link_refused.to_owned());
                    cx.notify();
                }
            }
            None => {
                self.areas.workspace.code.files.actions.refused =
                    Some(self.words().labels().file_link_refused.to_owned());
                cx.notify();
            }
        }
    }
}

/// Absolute/home/drive targets need an admitted workspace resolver. Until that
/// producer is available, refuse them rather than treating them as local paths.
fn relative_path(document: &RelPath, target: &str) -> Option<RelPath> {
    if target.starts_with(['/', '~']) || target.contains(['\\', ':']) {
        return None;
    }
    let parent = document.parent()?;
    let mut parts: Vec<&str> = parent.parts().collect();
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                let _parent = parts.pop()?;
            }
            component => parts.push(component),
        }
    }
    let path: RelPath = parts.join("/").parse().ok()?;
    (!path.is_root()).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_links_use_document_directory_and_cannot_escape_root() {
        let document = "docs/guide/start.md".parse().unwrap();
        assert_eq!(
            relative_path(&document, "../readme.md").unwrap().as_str(),
            "docs/readme.md"
        );
        assert_eq!(
            relative_path(&document, "./image.png").unwrap().as_str(),
            "docs/guide/image.png"
        );
        for target in [
            "../../../secret",
            "/etc/passwd",
            "~/secret",
            "C:/secret",
            "../../.git/config",
            "../../",
        ] {
            assert!(relative_path(&document, target).is_none(), "{target}");
        }
    }

    fn preview_shell(shell: &mut Shell, cx: &mut Context<'_, Shell>) -> FileKey {
        use crate::areas::workspace::code::io_tests;
        let root = io_tests::root();
        io_tests::target(shell, root);
        shell.route = crate::routes::Route::Session;
        shell.areas.workspace.code.open = true;
        shell.areas.workspace.code.tab = super::super::super::Tab::Files;
        let session = shell.areas.workspace.code.target.as_ref().unwrap().session;
        shell.areas.session.open = Some(crate::areas::session::Open {
            machine: "node".into(),
            session,
            title: "fixture".into(),
            agent: "fixture".into(),
            structured: true,
            live: true,
        });
        shell.open_file_at("docs/start.md".parse().unwrap(), None, cx);
        shell.areas.workspace.code.file_key().unwrap()
    }

    #[gpui::test]
    fn old_preview_click_cannot_use_rebound_same_node_or_another_session(
        cx: &mut gpui::TestAppContext,
    ) {
        use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
        use vessel_app_view::workspace::file_scope::FileScope;
        crate::shell::language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update(cx, |shell, cx| {
            let key = preview_shell(shell, cx);
            let root = key.root;
            let mut epochs = ConnectionEpochs::default();
            let first = epochs
                .admit(MachineKey::from_book("node"), key.node)
                .unwrap();
            let old = FileScope::new(first.clone(), root);
            shell
                .areas
                .workspace
                .code
                .target
                .as_mut()
                .unwrap()
                .admission
                .as_mut()
                .unwrap()
                .context = first.clone();
            shell.areas.workspace.code.viewer_for(&key).unwrap().scope = old.clone();
            let mut machine = vessel_app_view::Machine::unreachable("node".into(), "fixture");
            machine.node = Some(key.node);
            machine.connection_epoch = Some(first.epoch());
            shell.fleet.machines.push(machine);
            let old_open = shell.file_open_context().unwrap();
            assert_eq!(old_open.scope(), &old);
            assert!(shell.markdown_link_is_current(&key, &old_open));
            let second = epochs
                .admit(MachineKey::from_book("node"), key.node)
                .unwrap();
            let new = FileScope::new(second.clone(), root);
            shell.fleet.machines.last_mut().unwrap().connection_epoch = Some(second.epoch());
            shell
                .areas
                .workspace
                .code
                .target
                .as_mut()
                .unwrap()
                .admission
                .as_mut()
                .unwrap()
                .context = second;
            shell.areas.workspace.code.viewer_for(&key).unwrap().scope = new.clone();
            assert_eq!(shell.areas.workspace.code.file_key().as_ref(), Some(&key));
            assert!(!shell.markdown_link_is_current(&key, &old_open));
            shell.markdown_link(&key, &old_open, "../other.md", cx);
            shell.activate_file_row(
                &old_open,
                &"other.txt".parse().unwrap(),
                vessel_proto::workspace::EntryKind::File,
                cx,
            );
            assert_eq!(shell.areas.workspace.code.file_key().as_ref(), Some(&key));
            let new_open = shell.file_open_context().unwrap();
            assert_eq!(new_open.scope(), &new);
            assert!(shell.markdown_link_is_current(&key, &new_open));
            shell.areas.workspace.code.open = false;
            assert!(!shell.markdown_link_is_current(&key, &new_open));
            assert!(!shell.file_tree_scope_is_current(&new_open));
            shell.areas.workspace.code.open = true;
            let next_receipt = shell.next_file_ticket().unwrap();
            shell
                .areas
                .workspace
                .code
                .target
                .as_mut()
                .unwrap()
                .admission
                .as_mut()
                .unwrap()
                .ticket = next_receipt;
            assert!(!shell.markdown_link_is_current(&key, &new_open));
            let refreshed = shell.file_open_context().unwrap();
            assert!(shell.markdown_link_is_current(&key, &refreshed));
            shell.areas.session.open.as_mut().unwrap().session = vessel_proto::SessionId::random();
            assert!(!shell.markdown_link_is_current(&key, &new_open));
            assert!(shell.active_file_scope().is_none());
        });
    }
}
