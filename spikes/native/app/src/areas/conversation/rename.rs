//! The one rename dialog, as Paseo's `AdaptiveRenameModal`
//! (`components/rename-modal.tsx`, opened by `use-workspace-tab-rename.tsx`):
//! "Rename agent", the title in a field, prefilled and selected, then
//! Cancel and Rename side by side. A card on a wide window, a sheet from
//! the bottom on a phone (the kit dialog).
//!
//! [Decision Log]
//! - Purpose: TRACKER 800. Vessel had two rename dialogs: this one (789,
//!   Paseo's frame; it sent the rename and closed) and the workspace
//!   layout's (it waited for the node's answer, in a frame of its own). The
//!   coordinator ruled one (2026-11-18): Paseo's frame with the layout's
//!   send-and-wait and original-node authority, from every surface (the
//!   bar's title, the tab menu's Rename on both widths).
//! - Reference: Paseo's modal waits on its submit. While it waits, the
//!   field, Cancel and the dismiss are held and Rename reads "Saving...";
//!   success closes it; a failure shows its message under the field (or
//!   "Unable to save") with the typed title kept. An empty title says "Name
//!   is required"; Rename is disabled for the same title or one it would
//!   refuse; the field stops at `maxLength` (200 in Paseo).
//! - Choice: one state (`Rename`) with a route:
//!   - a workspace layout's (`layout::LayoutRoute`): the original node's
//!     alias and id, answered by ticket (`layout_renamed`);
//!   - otherwise the session's own act (`Act::Rename`), answered by the act
//!     for this machine and session (`rename_acted`).
//! - Field: the kit's rename profile, limited to the longest title the
//!   session's node decodes (`SessionTitle::max_chars_for` its protocol:
//!   Paseo's 200 from protocol 30, 120 before or while not connected).
//! - Answers come without a window: success closes the dialog and the next
//!   frame acknowledges it (`DialogState::dismissed`), so its opener has the
//!   keys again and the next opening remembers its own opener; a failure
//!   gives the field the keys at the next frame (app review of 800, R1).
//! - The session's act goes to the session shown when it is sent, so the
//!   dialog sends only while that is still the session it was opened for,
//!   and says it could not save otherwise (R2). A send waits at most 30 s
//!   (`RENAME_DEADLINE`): the act has no deadline of its own, and a held
//!   Cancel must not outlive a stalled connection.
//! - Cost: a session's own refusal is said in the dialog only, not again
//!   under the bar's title. An answer after the deadline still names the
//!   open session (its tab) and the cards.

use vessel_app_kit::theme::spacing::{S2, S3};
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, ControlSizing, DialogContent, DialogSize, Editability, TextField,
    Tokens, dialog,
};
use vessel_app_link::{Act, ActAnswer, ActKind, Acted};
use vessel_app_view::session::check_title;
use vessel_proto::{InvalidTitle, SessionId, SessionTitle};

use super::area::ConversationArea;
use crate::areas::layout::LayoutRoute;
use crate::shell::Shell;

/// How long a rename waits for the node before the dialog lets go.
const RENAME_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

/// A title being edited.
pub(crate) struct Rename {
    /// The field, holding the title as it is typed.
    pub(crate) field: TextField,
    /// What the title was when the edit began.
    was: String,
    /// What shows under the field: why the text cannot be a title, or why
    /// the rename failed.
    error: Option<String>,
    /// Where the rename goes.
    route: Route,
    /// A rename was sent and its answer has not come.
    pending: bool,
    /// A failure handed the keys back: the field takes them at the next
    /// frame (an answer comes without a window).
    refocus: std::cell::Cell<bool>,
    /// Which send it waits for (its deadline's).
    attempt: u64,
}

/// Where a rename goes, and how its answer is known.
pub(crate) enum Route {
    /// The session's own act, answered for this machine and session.
    Session {
        /// The session's machine.
        machine: String,
        /// The session.
        session: SessionId,
    },
    /// A workspace layout's: the original node's authority, by ticket.
    Layout(LayoutRoute),
}

impl Route {
    /// The machine the rename goes to.
    fn machine(&self) -> &str {
        match self {
            Self::Session { machine, .. } => machine,
            Self::Layout(route) => route.machine().as_str(),
        }
    }
}

#[cfg(test)]
impl Rename {
    /// What shows under the field.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether it waits for an answer.
    pub(crate) const fn pending(&self) -> bool {
        self.pending
    }
}

impl std::fmt::Debug for Rename {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rename")
            .field("was", &self.was)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl ConversationArea {
    /// The open form's layout route, when it has one.
    pub(crate) fn rename_layout_route(&mut self) -> Option<&mut LayoutRoute> {
        match self.rename.as_mut().map(|rename| &mut rename.route) {
            Some(Route::Layout(route)) => Some(route),
            Some(Route::Session { .. }) | None => None,
        }
    }

    /// The layout an open form was opened from, which it holds (the staged
    /// layout preferences read it).
    #[cfg(test)]
    pub(crate) fn rename_workspace(&self) -> Option<&vessel_app_view::layout::LayoutKey> {
        match self.rename.as_ref().map(|rename| &rename.route) {
            Some(Route::Layout(route)) => Some(route.workspace()),
            Some(Route::Session { .. }) | None => None,
        }
    }
}

impl Shell {
    /// Rename the open session, from any surface: its title as shown,
    /// selected, in the dialog. Nothing while the node does not describe it
    /// as renamable.
    pub(crate) fn open_rename(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let renamable = self
            .areas
            .conversation
            .info
            .as_deref()
            .is_some_and(vessel_app_view::session::can_rename);
        let Some(route) = self.rename_route().filter(|_| renamable) else {
            return;
        };
        let shown = super::heading::shown_title(self);
        self.start_rename(&shown, route, window, cx);
    }

    /// Open the dialog on `shown`, its text selected, renaming by `route`.
    pub(crate) fn start_rename(
        &mut self,
        shown: &str,
        route: Route,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        // No placeholder, as Paseo's tab rename passes none.
        let field = TextField::with_change(
            "",
            Self::rename_entered,
            |shell: &mut Self, _text, cx| {
                // Typing clears the last reason and updates Rename.
                if let Some(rename) = shell.areas.conversation.rename.as_mut() {
                    rename.error = None;
                }
                cx.notify();
            },
            window,
            cx,
        );
        // The longest title the session's node decodes (TRACKER 801): 200 from
        // protocol 30, 120 before or while it is not connected.
        let most = self.fleet.protocol_of(route.machine());
        let most = most.map_or(SessionTitle::LEGACY_MAX_CHARS, SessionTitle::max_chars_for);
        field.set_max_chars(most, cx);
        field.set_text(shown, window, cx);
        self.areas.conversation.rename_dialog.focus(window, cx);
        field.focus(window, cx);
        field.select_all(window, cx);
        // A new edit starts clean: the last refusal was about another.
        self.areas.conversation.refused = None;
        self.areas.conversation.rename = Some(Rename {
            field,
            was: shown.to_owned(),
            error: None,
            route,
            pending: false,
            refocus: std::cell::Cell::new(false),
            attempt: 0,
        });
        cx.notify();
    }

    /// The route for the open session: its workspace layout's, else its
    /// own act's. (Today the layout key is the session's, so the act's; the
    /// layout's route waits for workspace layouts.)
    fn rename_route(&self) -> Option<Route> {
        if self.on_workspace_layout() {
            return self.layout_rename_route().map(Route::Layout);
        }
        let shown = self.areas.conversation.shown.as_ref()?;
        Some(Route::Session {
            machine: shown.machine.clone(),
            session: shown.session,
        })
    }

    /// Cancel, Escape or the scrim: the title stays. Held while the rename
    /// waits for its answer, as Paseo's.
    pub(super) fn cancel_rename(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.conversation;
        if area.rename.as_ref().is_some_and(|rename| !rename.pending) {
            area.rename = None;
            area.rename_dialog.dismissed(window, cx);
            cx.notify();
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the field's submit callback hands the text over by value"
    )]
    fn rename_entered(&mut self, text: String, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.save_rename(&text, window, cx);
    }

    fn rename_pressed(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(text) = self
            .areas
            .conversation
            .rename
            .as_ref()
            .map(|rename| rename.field.text(cx))
        else {
            return;
        };
        self.save_rename(&text, window, cx);
    }

    /// Send `text` as the title and wait: the reason stays under the field
    /// when it cannot be one, and nothing is asked when it is the title
    /// already or a rename waits.
    fn save_rename(&mut self, text: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        // The catalog is the app's for its life: borrowed apart from `self`.
        let translator = self.words;
        let words = translator.layout();
        let Some(rename) = self.areas.conversation.rename.as_mut() else {
            return;
        };
        if rename.pending {
            return;
        }
        let title = match check_title(text) {
            Err(InvalidTitle::Empty) => {
                rename.error = Some(words.name_required().to_owned());
                cx.notify();
                return;
            }
            Err(why) => {
                rename.error = Some(self.t.lang.conversation().title_refused(why));
                cx.notify();
                return;
            }
            Ok(title) if title.as_str() == rename.was => return,
            Ok(title) => title,
        };
        rename.error = None;
        rename.pending = true;
        // Read-only and without its caret while it waits (Paseo's
        // `editable={!isPending}`): the dialog holds the keys, so Escape
        // still reaches it (and is held).
        rename.field.set_editability(Editability::ReadOnly, cx);
        let target = match &rename.route {
            Route::Layout(_) => None,
            Route::Session { machine, session } => Some((machine.clone(), *session)),
        };
        let attempt = self.areas.conversation.rename_sends.wrapping_add(1);
        self.areas.conversation.rename_sends = attempt;
        rename.attempt = attempt;
        self.areas.conversation.rename_dialog.focus(window, cx);
        let sent =
            match target {
                None => self.layout_send_rename(title),
                // Only to the session the dialog was opened for: the act goes
                // to the session shown now (the legacy seat), which may have
                // gone or changed while the dialog was open.
                Some((machine, session)) => {
                    let still =
                        self.areas.conversation.shown.as_ref().is_some_and(|shown| {
                            shown.machine == machine && shown.session == session
                        });
                    if still {
                        self.act(Act::Rename(title));
                    }
                    still
                }
            };
        if sent {
            Self::rename_deadline(attempt, cx);
        } else {
            self.rename_failed(words.unable_to_save().to_owned(), cx);
        }
        cx.notify();
    }

    /// No answer within `RENAME_DEADLINE`: the dialog stops waiting and
    /// says so, so Cancel works again (a stalled connection never answers;
    /// the session's act has no deadline of its own).
    fn rename_deadline(attempt: u64, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(RENAME_DEADLINE).await;
            let _waited = this.update(cx, |shell, cx| {
                let waiting = shell
                    .areas
                    .conversation
                    .rename
                    .as_ref()
                    .is_some_and(|rename| rename.pending && rename.attempt == attempt);
                if waiting {
                    let why = shell.words.layout().rename_timeout().to_owned();
                    shell.rename_failed(why, cx);
                }
            });
        })
        .detach();
    }

    /// The session's act answered: a rename this form sent closes it on
    /// success (naming the open session, so its tab's label follows) or
    /// shows its failure under the field. Whether the form took it.
    pub(crate) fn rename_acted(&mut self, answer: &ActAnswer, cx: &mut Context<'_, Self>) -> bool {
        // The open session takes the node's title (its tab's label), also
        // when the answer comes after the dialog stopped waiting.
        if answer.kind == ActKind::Rename
            && let Ok(Acted::Session(info)) = &answer.result
            && let Some(title) = info.title.as_ref()
            && let Some(open) = self
                .areas
                .session
                .open
                .as_mut()
                .filter(|open| open.session == answer.session && open.machine == answer.machine)
        {
            title.as_str().clone_into(&mut open.title);
        }
        let ours = answer.kind == ActKind::Rename
            && self
                .areas
                .conversation
                .rename
                .as_ref()
                .is_some_and(|rename| {
                    rename.pending
                        && matches!(&rename.route, Route::Session { machine, session }
                        if *machine == answer.machine && *session == answer.session)
                });
        if !ours {
            return false;
        }
        match &answer.result {
            Ok(_) => self.rename_done(cx),
            Err(failed) => {
                let why = if failed.message.trim().is_empty() {
                    self.words.layout().unable_to_save().to_owned()
                } else {
                    failed.message.clone()
                };
                self.rename_failed(why, cx);
            }
        }
        true
    }

    /// The rename was taken: the dialog closes, and the next frame hands
    /// the keys back to its opener (`rename_closed`).
    pub(crate) fn rename_done(&mut self, cx: &mut Context<'_, Self>) {
        self.areas.conversation.rename = None;
        self.areas.conversation.rename_closed.set(true);
        cx.notify();
    }

    /// The rename failed: the typed title stays, editable, with `why`.
    pub(crate) fn rename_failed(&mut self, why: String, cx: &mut Context<'_, Self>) {
        if let Some(rename) = self.areas.conversation.rename.as_mut() {
            rename.pending = false;
            rename.error = Some(why);
            rename.field.set_editability(Editability::Editable, cx);
            rename.refocus.set(true);
        }
        cx.notify();
    }
}

/// Whether Rename may be pressed: nothing waits, and the text is a title,
/// and another one.
fn savable(rename: &Rename, text: &str) -> bool {
    !rename.pending && check_title(text).is_ok_and(|title| title.as_str() != rename.was)
}

/// The dialog, while a rename is open.
pub(crate) fn rename_dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let area = &shell.areas.conversation;
    // Closed without a window: acknowledged now, so its opener has the
    // keys again (only while the dialog still holds them) and its next
    // opening remembers a new opener.
    if area.rename_closed.take() {
        area.rename_dialog.dismissed(window, cx);
    }
    let rename = area.rename.as_ref()?;
    if rename.refocus.take() {
        rename.field.focus(window, cx);
    }
    let words = shell.words.layout();
    let narrow = vessel_app_kit::breakpoint::phone(window);
    let sizing = ControlSizing::from_compact(narrow);
    let text = rename.field.text(cx);
    let held = if rename.pending {
        ButtonState::Disabled
    } else {
        ButtonState::Enabled
    };
    let cancel = vessel_app_kit::button_with_kind_at(
        "session-rename-cancel",
        words.cancel().to_owned(),
        ButtonKind::Secondary,
        held,
        sizing,
        tokens,
    )
    .flex_1();
    let cancel = if rename.pending {
        cancel
    } else {
        cancel.on_click(cx.listener(|shell, _event, window, cx| shell.cancel_rename(window, cx)))
    };
    let save_state = if savable(rename, &text) {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    };
    // Paseo's pending label in place of Rename.
    let save_label = if rename.pending {
        words.rename_saving()
    } else {
        words.rename_save()
    };
    let save = vessel_app_kit::button_with_kind_at(
        "session-rename-save",
        save_label.to_owned(),
        ButtonKind::Primary,
        save_state,
        sizing,
        tokens,
    )
    .flex_1();
    let save = if save_state == ButtonState::Enabled {
        save.on_click(cx.listener(|shell, _event, window, cx| shell.rename_pressed(window, cx)))
    } else {
        save
    };
    let body = div()
        .flex()
        .flex_col()
        .gap(px(S3))
        .pb(px(S2))
        .child(rename.field.rename_element(tokens, cx))
        .children(
            rename
                .error
                .clone()
                .map(|why| div().text_color(tokens.neutral.destructive).child(why)),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(S2))
                .child(cancel)
                .child(save),
        );
    Some(dialog(
        &shell.areas.conversation.rename_dialog,
        DialogContent {
            title: words.rename_agent().to_owned().into(),
            body: body.into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::cancel_rename,
        cx,
    ))
}

#[cfg(test)]
mod tests;
