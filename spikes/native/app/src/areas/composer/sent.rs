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
    Context, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_link::Command;
use vessel_app_link::attachments::{Answer, Files};
use vessel_app_view::composer::sent::{Picture, Pictures, Sent};
use vessel_app_view::composer::tray::{ItemState, Origin, Tray};
use vessel_proto::{AttachmentKind, AttachmentRef, SessionId};

use crate::areas::conversation::ConversationArea;
use crate::shell::Shell;
use vessel_app_view::composer::drafts::DraftKey;

/// What a message's files add under them so the text sits Paseo's 8
/// (`spacing[2]`, `imagePreviewSpacing`) below: the bubble
/// (`conversation::bubbles`) sets its parts 6 apart.
const UNDER_FILES: f32 = 2.0;

/// The files in one conversation's messages, as they are drawn.
#[derive(Clone)]
pub(crate) struct SentFiles<'a> {
    shell: &'a Shell,
    /// The conversation's machine and session, when one is shown.
    shown: Option<(vessel_app_view::connection::ConnectionContext, SessionId)>,
}

impl std::fmt::Debug for SentFiles<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SentFiles")
            .field("session", &self.shown.as_ref().map(|(_, session)| session))
            .finish_non_exhaustive()
    }
}

impl<'a> SentFiles<'a> {
    /// The files of the conversation of `machine`'s `session`, when one is
    /// shown.
    pub(crate) fn new(
        shell: &'a Shell,
        shown: Option<(vessel_app_view::machine_book::Name, SessionId)>,
    ) -> Self {
        let shown = shown.and_then(|(machine, session)| {
            shell
                .attachment_context(&machine)
                .map(|connection| (connection, session))
        });
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
        cx: &mut Context<'_, Shell>,
    ) -> Option<Div> {
        if files.is_empty() {
            return None;
        }
        let (images, others): (Vec<&AttachmentRef>, Vec<&AttachmentRef>) = files
            .iter()
            .partition(|file| file.kind == AttachmentKind::Image);
        let mut row = |files: Vec<&AttachmentRef>| {
            (!files.is_empty()).then(|| {
                attachment_tray()
                    .children(files.into_iter().map(|file| self.frame(file, tokens, cx)))
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

    fn frame(
        &self,
        file: &AttachmentRef,
        tokens: &Tokens,
        cx: &mut Context<'_, Shell>,
    ) -> vessel_app_kit::ui::AnyElement {
        let frame = attachment_frame(&self.body(file), tokens);
        let sent = self.shown.as_ref().map(|(node, session)| Sent {
            connection: node.clone(),
            session: *session,
            attachment: file.id,
        });
        let file = file.clone();
        frame
            .id(SharedString::from(format!("sent-image-{}", file.id)))
            .when(file.kind == AttachmentKind::Image, |frame| {
                frame
                    .cursor_pointer()
                    .on_click(cx.listener(move |shell, _, window, cx| {
                        if let Some(sent) = &sent {
                            shell.open_sent_image(sent.clone(), file.clone(), window, cx);
                        }
                    }))
            })
            .into_any_element()
    }

    /// `file`'s body in a message: an image as its picture once its bytes
    /// are in (asked for now; until then the frame is empty, Paseo's
    /// placeholder), an image that could not be read and any other file
    /// as its name over its kind.
    pub(crate) fn body(&self, file: &AttachmentRef) -> Body {
        let shell = self.shell;
        let Some((node, session)) = self.shown.as_ref().filter(|_| Pictures::shows(file)) else {
            return label(shell, file);
        };
        let sent = Sent {
            connection: node.clone(),
            session: *session,
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
            shown.and_then(|shown| {
                if let Ok(machine) = vessel_app_view::machine_book::Name::parse(&shown.machine) {
                    Some((machine, shown.session))
                } else {
                    tracing::warn!("a conversation has an invalid machine key");
                    None
                }
            }),
        )
    }
}

impl Shell {
    /// Begin the reads whose turn it is.
    pub(super) fn sent_reads(&self) {
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
            Answer::Submitted { prompt, result } => self.composer_submitted(prompt, result, cx),
            Answer::Root(answer) => self.composer_mention_root(answer, cx),
            Answer::Suggestions(answer) => self.composer_mention_answer(answer, cx),
            Answer::Capabilities { ask, result } => self.composer_capabilities(&ask, result, cx),
            Answer::Woke {
                message,
                session,
                outcome,
            } => {
                if self
                    .areas
                    .composer
                    .sends
                    .get(&session)
                    .is_none_or(|waiting| waiting.message != message)
                {
                    return;
                }

                match outcome {
                    vessel_app_link::attachments::WakeOutcome::Unknown => {
                        if let Some(waiting) = self.areas.composer.sends.get_mut(&session) {
                            waiting.phase = super::transmission::Phase::ResumeUncertain;
                        }
                        cx.notify();
                    }
                    vessel_app_link::attachments::WakeOutcome::Ready => self.resumed(session, cx),
                    vessel_app_link::attachments::WakeOutcome::Refused => {
                        if let Some(waiting) = self.areas.composer.sends.remove(&session)
                            && let Some(context) = &waiting.context
                            && let Some(tray) = self
                                .areas
                                .composer
                                .contexts
                                .get_mut(DraftKey::Session { session })
                        {
                            let _settled = tray.settle(
                                context,
                                vessel_app_view::composer::context::SubmissionOutcome::Failed,
                            );
                        }
                        self.resume_failed(
                            session,
                            self.words.composer().file_wake_failed().to_owned(),
                        );
                        cx.notify();
                    }
                }
            }
            Answer::Upload {
                message,
                session,
                key,
                step,
            } => self.composer_upload(session, &message, key, step, cx),
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
pub(super) fn seed(
    store: &RefCell<Pictures>,
    connection: &vessel_app_view::connection::ConnectionContext,
    session: SessionId,
    tray: &Tray,
) {
    let mut pictures = store.borrow_mut();
    for item in tray.items() {
        let (ItemState::Uploaded(attachment), Origin::Pasted(bytes)) = (&item.state, &item.origin)
        else {
            continue;
        };
        if attachment.kind == AttachmentKind::Image {
            let sent = Sent {
                connection: connection.clone(),
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
