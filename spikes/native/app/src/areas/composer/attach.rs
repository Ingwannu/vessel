//! A composer's attached files (track P's row c): files picked, pasted or
//! dropped join the composer's tray (`vessel_app_view::composer::tray`);
//! Send uploads them, two at a time, through the link
//! (`link::attachments`), and the prompt goes when every one is in.
//!
//! A session's composer sends at once to its live session. The new chat
//! first starts its session without a prompt (`Files::Create`); once the
//! session exists its tray becomes the session's, and once the session's
//! agent has said what it takes (its first records) the files go and the
//! prompt follows (the node takes a file only into a live session).

mod upload;
use std::path::PathBuf;
use std::sync::Arc;

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_link::attachments::Files;
use vessel_app_view::Card;
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer::tray::{ItemState, Origin, Target};
use vessel_proto::{AttachmentLimits, Delivery, SessionId};

use crate::shell::Shell;

/// A file arriving in a composer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Incoming {
    /// Its name.
    pub(crate) name: String,
    /// Its size in bytes.
    pub(crate) size: u64,
    /// Where its bytes are.
    pub(crate) origin: Origin,
}

impl Incoming {
    /// Inspect on the background executor; a refusal stays visible to the user.
    pub(super) fn file(path: PathBuf) -> Result<Self, LocalFileProblem> {
        let metadata = std::fs::metadata(&path).map_err(LocalFileProblem::Read)?;
        if !metadata.is_file() {
            return Err(LocalFileProblem::NotFile);
        }
        let name = path
            .file_name()
            .ok_or(LocalFileProblem::NoName)?
            .to_string_lossy()
            .into_owned();
        Ok(Self {
            name,
            size: metadata.len(),
            origin: Origin::File(path),
        })
    }

    /// Restore or acquire a private immutable byte snapshot.
    pub(super) fn stored(file: Arc<vessel_app_view::composer::local::LocalFile>) -> Self {
        Self {
            name: file.record.name.as_str().to_owned(),
            size: file.record.size.value(),
            origin: Origin::Stored(file),
        }
    }

    /// An image pasted as `mime` (named after its type: a clipboard has no
    /// file name).
    pub(crate) fn pasted(mime: &str, bytes: Vec<u8>) -> Self {
        let extension = match mime {
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "png",
        };
        Self {
            name: format!("image.{extension}"),
            size: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            origin: Origin::Pasted(Arc::from(bytes)),
        }
    }
}

/// Inspection failures stay typed and never include file contents.
#[derive(Debug)]
pub(super) enum LocalFileProblem {
    Read(std::io::Error),
    NotFile,
    NoName,
    #[cfg(not(target_arch = "wasm32"))]
    Storage(vessel_app_link::attachments::local::LocalFailure),
}

/// A prompt waiting for its files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Waiting {
    pub(super) connection: vessel_app_view::connection::ConnectionContext,
    pub(super) message: vessel_proto::timeline::ClientMessageId,
    pub(super) machine: vessel_app_view::machine_book::Name,
    pub(super) expected_node: vessel_proto::NodeId,
    pub(super) text: String,
    pub(super) context: Option<vessel_app_view::composer::context::Submission>,
    pub(super) phase: super::transmission::Phase,
    pub(super) local_files: Vec<Arc<vessel_app_view::composer::local::LocalFile>>,
    pub(super) delivery: Delivery,
    pub(super) creation: Option<vessel_app_view::creation::Ticket>,
}

/// A new chat started for its files, until its session exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Starting {
    pub(super) connection: vessel_app_view::connection::ConnectionContext,
    pub(super) message: vessel_proto::timeline::ClientMessageId,
    pub(super) machine: vessel_app_view::machine_book::Name,
    pub(super) expected_node: vessel_proto::NodeId,
    pub(super) text: String,
    pub(super) ticket: Option<vessel_app_view::creation::Ticket>,
    pub(super) files: vessel_app_view::composer::tray::Tray,
    pub(super) context: Option<vessel_app_view::composer::context::Submission>,
}

impl Shell {
    /// Files picked, pasted or dropped into `key`'s composer, each checked
    /// against its session as it comes.
    pub(crate) fn composer_add(
        &mut self,
        key: DraftKey,
        files: Vec<Incoming>,
        cx: &mut Context<'_, Self>,
    ) {
        if let DraftKey::Session { session } = key
            && self.areas.composer.sends.contains_key(&session)
        {
            self.composer_context_refused(
                vessel_app_view::composer::context::ContextProblem::Busy,
                cx,
            );
            return;
        }
        self.prune_empty_attachment_slots();
        let reserved = self.reserved_attachment_slots().0;
        let composer = &self.areas.composer;
        if composer.trays.get(key).is_none()
            && composer.trays.len().saturating_add(reserved)
                >= vessel_app_view::composer::trays::Trays::MAX
        {
            self.composer_context_refused(
                vessel_app_view::composer::context::ContextProblem::Busy,
                cx,
            );
            return;
        }
        let target = self.composer_target(key);
        let tray = self.areas.composer.trays.tray(key);
        tray.target(target);
        for file in files {
            let _key = tray.add(&file.name, file.size, file.origin);
        }
        self.local_files_changed(cx);
        cx.notify();
    }

    /// Whether `key`'s composer holds files.
    pub(crate) fn composer_has_files(&self, key: DraftKey) -> bool {
        self.areas
            .composer
            .trays
            .get(key)
            .is_some_and(|tray| !tray.is_empty())
    }

    /// Whether the open session's composer holds files.
    pub(crate) fn session_has_files(&self) -> bool {
        self.session_tray_key()
            .is_some_and(|key| self.composer_has_files(key) || self.composer_has_context(key))
    }

    /// Whether the new chat's composer holds files.
    pub(crate) fn draft_has_files(&self) -> bool {
        self.composer_has_files(DraftKey::New) || self.composer_has_context(DraftKey::New)
    }

    /// Whether Send goes in the open session's composer with `text`: there
    /// is something to send, no file in it is refused (the prompt would
    /// wait on a file that can never go; the pill says why), and no earlier
    /// prompt still waits for its files (the files shown are that prompt's,
    /// and a second send would take its place). The text stays in the
    /// composer until it may go.
    pub(crate) fn session_may_send(&self, text: &str) -> bool {
        let Some(session) = self.areas.session.open.as_ref().map(|open| open.session) else {
            return false;
        };
        if self.composer_attachments_blocked(DraftKey::Session { session }) {
            return false;
        }
        let composer = &self.areas.composer;
        let tray = composer.trays.get(DraftKey::Session { session });
        let files = tray.is_some_and(|tray| !tray.is_empty());
        let refused = tray.is_some_and(|tray| {
            tray.items()
                .iter()
                .any(|item| matches!(item.state, ItemState::Refused(_)))
        });
        let waiting = composer.sends.contains_key(&session)
            || (files || self.composer_has_context(DraftKey::Session { session }))
                && composer
                    .sends
                    .len()
                    .saturating_add(usize::from(composer.starting.borrow().is_some()))
                    .saturating_add(composer.workspace_starting.len())
                    >= vessel_app_view::composer::trays::Trays::MAX;
        (files
            || self.composer_has_context(DraftKey::Session { session })
            || !text.trim().is_empty())
            && !refused
            && !waiting
    }

    /// The session composer's tray key, when a session is open.
    pub(crate) fn session_tray_key(&self) -> Option<DraftKey> {
        let session = self.areas.session.open.as_ref()?.session;
        Some(DraftKey::Session { session })
    }

    /// What `key`'s files are checked against: the open session's agent
    /// and the node's limits, while the session runs; nothing for the new
    /// chat (its files wait for its session). The node's limits are this
    /// build's: the app does not read a conversation's own, and the node
    /// checks again at `attachment.begin`.
    fn composer_target(&self, key: DraftKey) -> Option<Target> {
        let DraftKey::Session { session } = key else {
            return None;
        };
        if self.areas.composer.sends.contains_key(&session)
            && let Some(target) = self.areas.composer.capabilities.target(session)
        {
            return Some(target);
        }
        let open = self.areas.session.open.as_ref()?;
        (open.session == session && open.live && open.structured).then(|| Target {
            abilities: self.areas.conversation.timeline.facts().abilities,
            limits: AttachmentLimits::DEFAULT,
        })
    }

    /// The open session's records arrived: its tray is checked against
    /// what its agent now says it takes, and a prompt waiting for its
    /// files goes on.
    pub(crate) fn composer_files_seen(&mut self, cx: &mut Context<'_, Self>) {
        let Some(key) = self.session_tray_key() else {
            return;
        };
        let target = self.composer_target(key);
        let Some(tray) = self.areas.composer.trays.get_mut(key) else {
            return;
        };
        tray.target(target);
        if let DraftKey::Session { session } = key {
            self.composer_pump(session, cx);
        }
    }

    /// Send in the session's composer with files attached: the text waits
    /// for the files (`true`: the send is taken; the composer empties).
    pub(crate) fn session_sends_files(
        &mut self,
        message: vessel_proto::timeline::ClientMessageId,
        text: &str,
        delivery: Delivery,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let Some(DraftKey::Session { session }) = self.session_tray_key() else {
            return false;
        };
        let Some(open) = self.areas.session.open.as_ref() else {
            return false;
        };
        if !self.session_has_files() {
            return false;
        }
        let Ok(machine) = vessel_app_view::machine_book::Name::parse(&open.machine) else {
            return true;
        };
        let Some(connection) = self.attachment_context(&machine) else {
            self.conversation_send_failed();
            self.toast(
                self.words.composer().file_capabilities_failed(),
                vessel_app_view::toasts::ToastTone::Error,
                cx,
            );
            return true;
        };
        let expected_node = connection.node();
        let Ok(context) = self.prepare_session_context(session, cx) else {
            return true;
        };
        let waiting = Waiting {
            connection,
            message,
            local_files: self.retained_local_files(DraftKey::Session { session }),
            phase: super::transmission::Phase::Preparing,
            context,
            machine,
            expected_node,
            text: text.to_owned(),
            delivery,
            creation: None,
        };
        drop(self.areas.composer.sends.insert(session, waiting));
        self.local_files_changed(cx);
        self.session_composer_sent(window, cx);
        self.composer_pump(session, cx);
        true
    }

    /// Resume an ended conversation before uploading, preserving the pending
    /// text and tray if resuming fails. No plain-text prompt escapes first.
    pub(crate) fn resume_attachment_prompt(
        &mut self,
        message: vessel_proto::timeline::ClientMessageId,
        text: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(open) = self.areas.session.open.as_ref() else {
            return;
        };
        let session = open.session;
        let machine = open.machine.clone();
        let Ok(key) = vessel_app_view::machine_book::Name::parse(&machine) else {
            self.toast(
                self.words.composer().file_wake_failed(),
                vessel_app_view::toasts::ToastTone::Error,
                cx,
            );
            return;
        };
        let Some(connection) = self.attachment_context(&key) else {
            self.conversation_send_failed();
            self.toast(
                self.words.composer().file_capabilities_failed(),
                vessel_app_view::toasts::ToastTone::Error,
                cx,
            );
            return;
        };
        let expected_node = connection.node();
        let Ok(context) = self.prepare_session_context(session, cx) else {
            return;
        };
        let local_files = self.retained_local_files(DraftKey::Session { session });
        drop(self.areas.composer.sends.insert(
            session,
            Waiting {
                connection: connection.clone(),
                message: message.clone(),
                local_files,
                phase: super::transmission::Phase::Preparing,
                context,
                machine: key.clone(),
                expected_node,
                text: text.to_owned(),
                delivery: Delivery::Queue,
                creation: None,
            },
        ));
        self.local_files_changed(cx);
        self.areas.session.resuming = Some(text.to_owned());
        self.areas.session.resume_error = None;
        self.command(Command::Files(Files::Wake(
            vessel_app_link::attachments::Wake {
                connection,
                message,
                machine: key,
                expected_node,
                session,
            },
        )));
        self.session_composer_sent(window, cx);
        cx.notify();
    }

    /// A session started from the app: opened as before (`started`), the
    /// new chat's draft made what its composer now holds (`new_chat_kept`),
    /// and when it was started for the new chat's files, the files become
    /// its tray and the prompt waits for them.
    pub(crate) fn started_with_files(&mut self, card: &Card, cx: &mut Context<'_, Self>) {
        self.started(card, cx);
        self.new_chat_kept(cx);
        let legacy = self
            .areas
            .composer
            .starting
            .borrow()
            .as_ref()
            .is_some_and(|held| held.ticket.is_none());
        if legacy {
            self.composer_started(card, cx);
        }
    }

    pub(super) fn composer_started(&mut self, card: &Card, cx: &mut Context<'_, Self>) {
        let matches = self
            .areas
            .composer
            .starting
            .borrow()
            .as_ref()
            .is_some_and(|starting| starting.machine.as_str() == card.machine);
        if !matches {
            return;
        }
        if self.areas.composer.sends.contains_key(&card.session) {
            return;
        }
        let Some(starting) = self.areas.composer.starting.borrow_mut().take() else {
            return;
        };
        self.composer_started_from(DraftKey::New, starting, card, cx);
    }

    pub(super) fn composer_started_from(
        &mut self,
        source: DraftKey,
        starting: Starting,
        card: &Card,
        cx: &mut Context<'_, Self>,
    ) {
        let session = card.session;
        let local_files = starting
            .files
            .items()
            .iter()
            .filter_map(|item| match &item.origin {
                Origin::Stored(file) => Some(Arc::clone(file)),
                Origin::File(_) | Origin::Pasted(_) => None,
            })
            .collect();
        let composer = &mut self.areas.composer;
        if let Some(draft) = composer.trays.get_mut(source) {
            for captured in starting.files.items() {
                let _removed = draft.remove(captured.key);
            }
        }
        if !starting.files.is_empty() {
            *composer.trays.tray(DraftKey::Session { session }) = starting.files;
        }
        if let Some(context) = &starting.context {
            if let Some(draft) = composer.contexts.get_mut(source) {
                let _settled = draft.settle(
                    context,
                    vessel_app_view::composer::context::SubmissionOutcome::Sent,
                );
            }
            *composer.contexts.tray(DraftKey::Session { session }) =
                vessel_app_view::composer::context::ContextTray::captured(context);
        }
        let waiting = Waiting {
            connection: starting.connection,
            message: starting.message,
            local_files,
            phase: super::transmission::Phase::Preparing,
            context: starting.context,
            machine: starting.machine,
            expected_node: starting.expected_node,
            text: starting.text,
            delivery: Delivery::Start,
            creation: starting.ticket,
        };
        drop(composer.sends.insert(session, waiting));
        self.local_files_changed(cx);
        if starting.ticket.is_none() {
            self.composer_pump(session, cx);
        }
    }
}
