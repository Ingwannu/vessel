//! Markdown paths are relative to the captured document, never selected globals.
//! Lexical normalization grants no disk authority: the node rechecks links/root.

use vessel_app_kit::ui::Context;
use vessel_app_view::composer_shell::links::{Link, Origin, Workspace, classify};
use vessel_app_view::workspace::file_location::FileLocation;
use vessel_proto::workspace::RelPath;

use super::FileKey;
use crate::shell::Shell;

impl Shell {
    pub(super) fn markdown_link(&mut self, key: &FileKey, href: &str, cx: &mut Context<'_, Self>) {
        // A retained callback cannot navigate a newly selected workspace, even
        // when it has a file with the same path. The live lease cutover adds
        // G4's generation admission to this existing captured owner guard.
        if self.areas.workspace.code.file_key().as_ref() != Some(key) {
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
}
