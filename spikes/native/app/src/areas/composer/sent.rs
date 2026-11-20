//! Sent files on screen (track P's row c, its third step): in a message, an
//! image as its picture and any other file as its label, as Paseo's user
//! message shows them (`components/message.tsx`: `UserMessageImagePill`,
//! `AttachmentLabel`), drawn by the kit's `attachment_frame`.
//!
//! An image's bytes come from the view's store (`composer::sent`, whose
//! decision log follows the page's `data/images.ts`): asked for when its
//! message is drawn and read back from its node two at a time, or kept from
//! the composer when it was sent from here. A message is drawn from the
//! shell read only, so the store sits in a cell and the reads are sent from
//! the drawing (a command is a message to the link, not a change of the
//! shell); their answers change the shell as any answer does.

use std::cell::RefCell;
use std::sync::Arc;

use vessel_app_kit::Tokens;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::attachment::{Body, attachment_frame, attachment_tray};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, FluentBuilder as _, ParentElement as _, SharedString, Styled as _, div, px,
};
use vessel_app_link::Command;
use vessel_app_link::attachments::{Answer, Files};
use vessel_app_view::composer::sent::{Picture, Pictures, Sent};
use vessel_app_view::composer::tray::{ItemState, Origin, Tray};
use vessel_proto::{AttachmentKind, AttachmentRef, SessionId};

use crate::areas::conversation::ConversationArea;
use crate::shell::Shell;

/// What a message's files add under them so the text sits Paseo's 8
/// (`spacing[2]`, `imagePreviewSpacing`) below: the bubble
/// (`conversation::bubbles`) sets its parts 6 apart.
const UNDER_FILES: f32 = 2.0;

/// The files in one conversation's messages, as they are drawn.
#[derive(Clone, Copy)]
pub(crate) struct SentFiles<'a> {
    shell: &'a Shell,
    /// The conversation's machine and session, when one is shown.
    shown: Option<(&'a str, SessionId)>,
}

impl std::fmt::Debug for SentFiles<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SentFiles")
            .field("session", &self.shown.map(|(_, session)| session))
            .finish_non_exhaustive()
    }
}

impl<'a> SentFiles<'a> {
    /// The files of the conversation of `machine`'s `session`, when one is
    /// shown.
    pub(crate) fn new(shell: &'a Shell, shown: Option<(&'a str, SessionId)>) -> Self {
        Self { shell, shown }
    }

    /// A message's files as Paseo's user message shows them: its images in
    /// a row, then its other files in a row, each row 8 apart that wraps,
    /// and 8 above the text when text follows; `None` when it has none.
    pub(crate) fn block(
        &self,
        files: &[AttachmentRef],
        text_follows: bool,
        tokens: &Tokens,
    ) -> Option<Div> {
        if files.is_empty() {
            return None;
        }
        let (images, others): (Vec<&AttachmentRef>, Vec<&AttachmentRef>) = files
            .iter()
            .partition(|file| file.kind == AttachmentKind::Image);
        let row = |files: Vec<&AttachmentRef>| {
            (!files.is_empty()).then(|| {
                attachment_tray().children(
                    files
                        .into_iter()
                        .map(|file| attachment_frame(&self.body(file), tokens)),
                )
            })
        };
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S2))
                .children(row(images))
                .children(row(others))
                .when(text_follows, |block| block.mb(px(UNDER_FILES))),
        )
    }

    /// `file`'s body in a message: an image as its picture once its bytes
    /// are in (asked for now; until then the frame is empty, Paseo's
    /// placeholder), an image that could not be read and any other file
    /// as its name over its kind.
    pub(crate) fn body(&self, file: &AttachmentRef) -> Body {
        let shell = self.shell;
        let Some((machine, session)) = self.shown.filter(|_| Pictures::shows(file)) else {
            return label(shell, file);
        };
        let sent = Sent {
            machine: machine.to_owned(),
            session,
            attachment: file.id,
        };
        let picture = shell.areas.composer.sent.borrow_mut().ask(&sent);
        match picture {
            Picture::Ready(bytes) => Body::Image(Some(
                shell.areas.composer.pictures.of(&bytes, file.mime.as_str()),
            )),
            Picture::Wanted => {
                shell.sent_reads();
                Body::Image(None)
            }
            Picture::Reading => Body::Image(None),
            Picture::Failed => label(shell, file),
        }
    }
}

impl ConversationArea {
    /// The files in this conversation's messages, as they are drawn (a
    /// seam in track C's rows: their `Paint` carries it).
    pub(crate) fn sent_files<'a>(&'a self, shell: &'a Shell) -> SentFiles<'a> {
        let shown = self.shown.as_ref();
        SentFiles::new(
            shell,
            shown.map(|shown| (shown.machine.as_str(), shown.session)),
        )
    }
}

impl Shell {
    /// Begin the reads whose turn it is.
    fn sent_reads(&self) {
        let starts = self.areas.composer.sent.borrow_mut().starts();
        for sent in starts {
            self.command(Command::Files(Files::Read(sent)));
        }
    }

    /// The link's answer about files: a composer's upload goes on
    /// (`composer_upload`), or a sent image's bytes are in and the next
    /// reads begin.
    pub(crate) fn composer_answered(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        match answer {
            Answer::Upload { session, key, step } => self.composer_upload(session, key, step, cx),
            Answer::Read { sent, bytes } => {
                self.areas.composer.sent.borrow_mut().read(&sent, bytes);
                self.sent_reads();
                cx.notify();
            }
        }
    }
}

/// The images of a prompt about to go from `machine`'s `session`'s `tray`,
/// kept in `store` from the bytes the composer holds (a pasted image), so
/// the message shows them without reading them back. A file picked on
/// this device is read back as any other (the file may have changed).
pub(super) fn seed(store: &RefCell<Pictures>, machine: &str, session: SessionId, tray: &Tray) {
    let mut pictures = store.borrow_mut();
    for item in tray.items() {
        let (ItemState::Uploaded(attachment), Origin::Pasted(bytes)) = (&item.state, &item.origin)
        else {
            continue;
        };
        if attachment.kind == AttachmentKind::Image {
            let sent = Sent {
                machine: machine.to_owned(),
                session,
                attachment: attachment.id,
            };
            pictures.seed(sent, Arc::clone(bytes));
        }
    }
}

/// `file` as its name over its kind (its extension, or the word for a
/// file).
fn label(shell: &Shell, file: &AttachmentRef) -> Body {
    let name = file.name.as_str();
    Body::Label {
        icon: Glyph::FileText,
        title: SharedString::from(name.to_owned()),
        subtitle: SharedString::from(super::files::kind_line(shell, name)),
    }
}
