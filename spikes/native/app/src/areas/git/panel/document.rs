//! Immutable unified buffers over the checked readonly document control.
//! The source editor's lexer, selection, clipboard and outer reveal are reused.
//! Split remains unmounted until its full-width header/copy plan is approved.
use super::bodies::Reveal;
use crate::shell::Shell;
use std::{fmt, sync::Arc};
use vessel_app_kit::code_editor::{
    DiffDocumentEditor, DiffLine, DiffText, DiffTextError, DiffTone, DiffWrapping,
};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, App, Context, IntoElement as _, ParentElement as _, Pixels, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::localization::Translator;
use vessel_app_view::text::workspace::WorkspaceWords;
use vessel_app_view::workspace::changes::FileNote;
use vessel_app_view::workspace::diff_rows::{DiffBody, add_page};
use vessel_app_view::workspace::git::panel::{
    BufferColumns, BufferError, BufferKind, DiffDocument, FilePages, FileTicket, Layout,
};

#[derive(Debug)]
enum DocumentProblem {
    Source(BufferError),
    Input(DiffTextError),
    UnexpectedSplit,
}
impl fmt::Display for DocumentProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(why) => fmt::Display::fmt(why, f),
            Self::Input(why) => fmt::Display::fmt(why, f),
            Self::UnexpectedSplit => f.write_str("the unified document returned split buffers"),
        }
    }
}
impl std::error::Error for DocumentProblem {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(why) => Some(why),
            Self::Input(why) => Some(why),
            Self::UnexpectedSplit => None,
        }
    }
}

enum Content {
    Text {
        editor: DiffDocumentEditor,
        truncated: bool,
    },
    Notice(FileNote),
    Failed(DocumentProblem),
}
pub(super) struct Document {
    receipt: FileTicket,
    content: Content,
}
pub(super) struct Words<'a> {
    pub(super) catalog: &'a Translator,
    pub(super) legacy: &'static dyn WorkspaceWords,
}
impl Document {
    pub(super) fn new(
        pages: &FilePages,
        reveal: Reveal,
        window: &mut Window,
        cx: &mut Context<'_, Shell>,
    ) -> Self {
        let content = match add_page(None, pages.file()) {
            DiffBody::Rows { rows, .. } => match input(pages.ticket(), rows) {
                Ok((text, truncated)) => Content::Text {
                    editor: DiffDocumentEditor::new(
                        text,
                        crate::areas::workspace::code::syntax_for(pages.ticket().path()),
                        DiffWrapping::Scroll,
                        move |shell: &mut Shell, bounds, window, cx| {
                            shell.git_panel_reveal(&reveal, bounds, window, cx)
                        },
                        window,
                        cx,
                    ),
                    truncated,
                },
                Err(why) => Content::Failed(why),
            },
            DiffBody::Binary => Content::Notice(FileNote::Binary),
            DiffBody::Submodule { from, to } => Content::Notice(FileNote::Submodule { from, to }),
            DiffBody::TooLarge { bytes } => Content::Notice(FileNote::TooLarge { bytes }),
            DiffBody::Unchanged => Content::Notice(FileNote::Unchanged),
            DiffBody::Unknown => Content::Notice(FileNote::Unknown),
        };
        Self {
            receipt: pages.ticket().clone(),
            content,
        }
    }

    pub(super) fn matches(&self, pages: &FilePages) -> bool {
        self.receipt == *pages.ticket()
    }

    pub(super) fn selection_focused(&self, window: &Window, cx: &App) -> bool {
        match &self.content {
            Content::Text { editor, .. } => editor.selection_focused(window, cx),
            Content::Notice(_) | Content::Failed(_) => false,
        }
    }

    pub(super) fn element(
        &self,
        words: Words<'_>,
        tokens: &Tokens,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<'_, Shell>,
    ) -> AnyElement {
        match &self.content {
            Content::Text { editor, truncated } => div()
                .w_full()
                .flex()
                .flex_col()
                .child(editor.element(tokens, width, window, cx))
                .children(truncated.then(|| caption(words.catalog.git_panel().truncated(), tokens)))
                .into_any_element(),
            Content::Notice(note) => caption(&words.legacy.file_note(note), tokens),
            Content::Failed(why) => super::draw::failure_detail(
                words.catalog,
                words.catalog.git_panel().invalid_document(),
                vessel_app_view::workspace::git::panel::ReadIssue::Page.variant(),
                Some(why.to_string()),
                tokens,
                window,
                cx,
            ),
        }
    }
}

fn input(
    ticket: &FileTicket,
    rows: Arc<[vessel_app_view::workspace::diff_rows::DiffRow]>,
) -> Result<(DiffText, bool), DocumentProblem> {
    let document = DiffDocument::new(ticket, rows, Layout::Unified);
    let BufferColumns::Unified(column) = document.buffers().map_err(DocumentProblem::Source)?
    else {
        return Err(DocumentProblem::UnexpectedSplit);
    };
    let truncated = column.lines.iter().any(|line| line.cut);
    let lines = column
        .lines
        .iter()
        .map(|line| DiffLine {
            number: line
                .number
                .and_then(|number| std::num::NonZeroU32::new(number.get())),
            tone: match line.kind {
                BufferKind::Context => DiffTone::Context,
                BufferKind::Added => DiffTone::Added,
                BufferKind::Removed => DiffTone::Removed,
                BufferKind::Empty => DiffTone::Empty,
                BufferKind::Header => DiffTone::Header,
                BufferKind::Notice => DiffTone::Notice,
            },
        })
        .collect::<Vec<_>>();
    Ok((
        DiffText::new(column.text, lines.into()).map_err(DocumentProblem::Input)?,
        truncated,
    ))
}

pub(super) fn caption(text: &str, tokens: &Tokens) -> AnyElement {
    div()
        .w_full()
        .px(px(spacing::S3))
        .py(px(spacing::S2))
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.to_owned())
        .into_any_element()
}
