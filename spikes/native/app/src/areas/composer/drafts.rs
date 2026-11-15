//! What was typed in a composer and not sent (track P's row e): kept per
//! session and for the new chat as it is typed, put back when that
//! composer comes back, kept on the device a second after the last edit;
//! and Up and Down in an empty session composer through the messages the
//! person sent (Paseo's `composer/recall.ts`). The rules are
//! `vessel_app_view::composer::{drafts, recall}`'s; the file is the link's
//! (`link::drafts`).

use std::time::Duration;

use chrono::Utc;
use vessel_app_kit::field::composer_hooks::{ComposerKey, KeyAnswer};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::composer::drafts::{DraftKey, Kept};
use vessel_app_view::composer::recall::{Arrow, At, Recall, sent_newest_first};
use vessel_app_view::timeline::EntryKind;
use vessel_proto::SessionId;

use crate::areas::Areas;
use crate::shell::Shell;

/// How long after the last edit the drafts are written (one write for a
/// burst of typing).
const SETTLE: Duration = Duration::from_secs(1);

/// The drafts kept on the device, the new chat's put back, and both
/// composers' edits followed (called once, when the areas are made).
pub(crate) fn keep_drafts(areas: &mut Areas, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let drafts = vessel_app_link::drafts::load();
    let new_chat = drafts.text(DraftKey::New);
    if !new_chat.is_empty() {
        areas.draft.composer.set_text(new_chat, window, cx);
    }
    areas.composer.drafts = drafts;
    areas
        .draft
        .composer
        .on_edit(Shell::draft_composer_edited, window, cx);
    areas
        .session
        .composer
        .on_edit(Shell::session_composer_edited, window, cx);
}

impl Shell {
    /// The new chat's composer changed: its draft follows.
    fn draft_composer_edited(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.new_chat_kept(cx);
    }

    /// The new chat's draft made what its composer holds. Also called when
    /// a session starts: the composer was emptied when it was sent, and a
    /// text the screen writes (`clear`, `set_text`) is not an edit
    /// `on_edit` hears (gpui-base's `set_value` turns its events off).
    /// Kept until the start succeeds, so a failed start's text, put back
    /// in the composer, is still the draft.
    pub(crate) fn new_chat_kept(&mut self, cx: &mut Context<'_, Self>) {
        let text = self.areas.draft.composer.text(cx);
        self.keep_draft(DraftKey::New, &text, cx);
    }

    /// The session composer was sent from: emptied, and the open session's
    /// draft with it (the field's `clear` is not heard as an edit; see
    /// `new_chat_kept`).
    pub(crate) fn session_composer_sent(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.session.composer.clear(window, cx);
        self.session_composer_edited(window, cx);
    }

    /// The session composer changed: the open session's draft follows. A
    /// sent message shown by recall is not a draft until it is edited (the
    /// page's `show`): recall starts only from an empty composer, so the
    /// draft stays empty while the composer holds it as recalled.
    fn session_composer_edited(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(session) = self.areas.session.open.as_ref().map(|open| open.session) else {
            return;
        };
        let text = self.areas.session.composer.text(cx);
        let draft = if self.areas.composer.recall.showing(&text) {
            ""
        } else {
            text.as_str()
        };
        self.keep_draft(DraftKey::Session { session }, draft, cx);
    }

    fn keep_draft(&mut self, key: DraftKey, text: &str, cx: &mut Context<'_, Self>) {
        let before = self.areas.composer.drafts.text(key) == text;
        if before {
            return;
        }
        if self.areas.composer.drafts.keep(key, text, Utc::now()) == Kept::TooLong {
            tracing::info!(bytes = text.len(), "a draft too long to keep on the device");
        }
        self.drafts_changed(cx);
    }

    /// Another session opens in the session composer: what was typed for
    /// the one open stays its draft, and the new one's draft comes back.
    pub(crate) fn composer_switched(
        &mut self,
        to: SessionId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let from = self.areas.session.open.as_ref().map(|open| open.session);
        if from == Some(to) {
            return;
        }
        let field = &self.areas.session.composer;
        let typed = field.text(cx);
        if let Some(from) = from {
            self.keep_draft(DraftKey::Session { session: from }, &typed, cx);
        }
        let next = self
            .areas
            .composer
            .drafts
            .text(DraftKey::Session { session: to })
            .to_owned();
        if next != typed {
            self.areas.session.composer.set_text(&next, window, cx);
        }
        self.areas.composer.recall = Recall::default();
    }

    /// The drafts changed: written a second after the last change.
    fn drafts_changed(&mut self, cx: &mut Context<'_, Self>) {
        if self.areas.composer.saving {
            return;
        }
        self.areas.composer.saving = true;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SETTLE).await;
            drop(this.update(cx, |shell, _cx| {
                shell.areas.composer.saving = false;
                shell.command(Command::SaveDrafts(shell.areas.composer.drafts.clone()));
            }));
        })
        .detach();
    }

    /// Up or Down in the session composer when no menu takes it: the
    /// messages sent in this session, newest first, from an empty composer
    /// (Paseo's `ComposerRecall`).
    pub(crate) fn recall_key(
        &mut self,
        key: ComposerKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> KeyAnswer {
        let arrow = match key {
            ComposerKey::Up => Arrow::Up,
            ComposerKey::Down => Arrow::Down,
            ComposerKey::Enter | ComposerKey::Tab | ComposerKey::Escape => {
                return KeyAnswer::Pass;
            }
        };
        let field = &self.areas.session.composer;
        let text = field.text(cx);
        let selection = field.selection(cx);
        let entries = self.areas.conversation.timeline.entries();
        let shown = self.areas.composer.recall.press(
            arrow,
            At {
                text: &text,
                selection: (selection.start, selection.end),
            },
            || {
                sent_newest_first(entries.iter().filter_map(|entry| {
                    if let EntryKind::Prompt(prompt) = &entry.kind {
                        Some(prompt.text.as_str())
                    } else {
                        None
                    }
                }))
            },
        );
        let Some(shown) = shown else {
            return KeyAnswer::Pass;
        };
        // The caret at the end, as Paseo's composer leaves it (its recall
        // tests drive it so): Up then moves through a recalled message's
        // lines before it recalls an older one.
        self.areas.session.composer.replace_all(&shown, window, cx);
        cx.notify();
        KeyAnswer::Taken
    }
}
