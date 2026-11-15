//! Retained Markdown presentation owns only preview work and scoped interaction.
//! The shared kit owns rendering/selection; the editor keeps the same buffer.

mod draw;
mod links;
pub(super) use draw::{body, toolbar};

use std::sync::Arc;

use gpui::AppContext as _;
use vessel_app_kit::ui::Context;
use vessel_app_view::workspace::file_location::FileLocation;
use vessel_app_view::workspace::file_ticket::FileTicket;
use vessel_app_view::workspace::markdown_preview::{MarkdownDocument, MarkdownSource, parse};
use vessel_proto::workspace::RelPath;

use super::{Shown, Viewer, buffers::FileKey};
use crate::shell::Shell;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Source,
    Preview,
}

enum Stage {
    Idle,
    Parsing(FileTicket),
    Ready {
        ticket: FileTicket,
        document: MarkdownDocument,
    },
    Incomplete,
}

pub(super) struct Preview {
    mode: Mode,
    stage: Stage,
}

impl Preview {
    pub(super) fn for_path(path: &RelPath, location: Option<FileLocation>) -> Option<Self> {
        let (_, extension) = path.file_name()?.rsplit_once('.')?;
        if !["md", "markdown"]
            .iter()
            .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        {
            return None;
        }
        Some(Self {
            mode: if location.is_some() {
                Mode::Source
            } else {
                Mode::Preview
            },
            stage: Stage::Idle,
        })
    }

    pub(super) fn reveal_source(&mut self) {
        self.mode = Mode::Source;
    }

    fn complete(&mut self, ticket: FileTicket, document: MarkdownDocument) -> bool {
        if !matches!(self.stage, Stage::Parsing(at) if at == ticket) {
            return false;
        }
        self.stage = Stage::Ready { ticket, document };
        true
    }

    pub(super) fn invalidate(&mut self) {
        self.stage = Stage::Idle;
    }
}

fn complete_source(viewer: &Viewer) -> Option<MarkdownSource> {
    let source = if let Some(editing) = &viewer.edit {
        let _loaded = editing.model.version()?;
        Arc::<str>::from(editing.model.content())
    } else if let Shown::Text {
        source,
        more: false,
        ..
    } = &viewer.text
    {
        source.shared()
    } else {
        return None;
    };
    MarkdownSource::try_from(source).ok()
}

impl Shell {
    pub(super) fn prepare_markdown(&mut self, key: &FileKey, cx: &mut Context<'_, Self>) {
        let Some(viewer) = self.areas.workspace.code.viewer_for(key) else {
            return;
        };
        let Some(preview) = &viewer.markdown else {
            return;
        };
        if !matches!(preview.stage, Stage::Idle) {
            return;
        }
        let source = complete_source(viewer);
        let Some(source) = source else {
            if let Some(preview) = &mut viewer.markdown {
                preview.stage = Stage::Incomplete;
            }
            cx.notify();
            return;
        };
        let Some(ticket) = self.next_file_ticket() else {
            return;
        };
        if let Some(preview) = self
            .areas
            .workspace
            .code
            .viewer_for(key)
            .and_then(|viewer| viewer.markdown.as_mut())
        {
            preview.stage = Stage::Parsing(ticket);
        }
        let parse = cx.background_spawn(async move { parse(&source) });
        let key = key.clone();
        cx.spawn(async move |this, cx| {
            let document = parse.await;
            let _gone = this.update(cx, |shell, cx| {
                if let Some(preview) = shell
                    .areas
                    .workspace
                    .code
                    .viewer_for(&key)
                    .and_then(|viewer| viewer.markdown.as_mut())
                    && preview.complete(ticket, document)
                {
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(body: &str) -> MarkdownDocument {
        MarkdownDocument {
            front_matter: Vec::new(),
            body: Arc::from(body),
        }
    }

    #[test]
    fn navigation_opens_source_and_refresh_cannot_accept_old_parse() {
        let path = "Readme.MD".parse().unwrap();
        let mut preview = Preview::for_path(&path, None).unwrap();
        assert!(preview.mode == Mode::Preview);
        preview.stage = Stage::Parsing(FileTicket(1));
        preview.invalidate();
        preview.stage = Stage::Parsing(FileTicket(2));
        assert!(!preview.complete(FileTicket(1), document("old")));
        assert!(preview.complete(FileTicket(2), document("new")));
        assert!(
            matches!(&preview.stage, Stage::Ready { document, .. } if &*document.body == "new")
        );
        let preview =
            Preview::for_path(&path, Some(FileLocation::line(std::num::NonZeroU32::MIN))).unwrap();
        assert!(preview.mode == Mode::Source);
        assert!(Preview::for_path(&"source.rs".parse().unwrap(), None).is_none());
    }

    #[gpui::test]
    fn initial_markdown_waits_for_a_real_read_but_accepts_an_empty_loaded_document(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::super::io_tests;
        crate::shell::language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update(cx, |shell, cx| {
            let root = io_tests::root();
            io_tests::target(shell, root);
            shell.open_file_at("readme.md".parse().unwrap(), None, cx);
            let key = shell.areas.workspace.code.file_key().unwrap();
            assert!(complete_source(shell.areas.workspace.code.viewer_at(&key).unwrap()).is_none());
            let ticket = shell.editing_for(&key).unwrap().reading.unwrap().0;
            let ask = super::super::editing::whole_read(root, key.path.clone());
            shell.editor_read_answered_for(&key, ticket, &ask, Ok(io_tests::text("")), cx);
            assert!(complete_source(shell.areas.workspace.code.viewer_at(&key).unwrap()).is_some());
        });
    }
}
