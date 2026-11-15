//! Adding a machine by pairing (ADR 0010), in two steps a person cannot
//! mix up (the owner typed the key into the name field of build
//! 0.0.0.0.1, whose screen read as if it asked for it):
//!
//! 1. The link: pasted, decoded, cleared from its field (it is a
//!    single-use secret).
//! 2. The key shown large, with nothing to type, and one sentence: it
//!    must be the same as the key the machine printed with the link. Then
//!    the name, under a label that stays, empty with a placeholder that
//!    says an empty name uses the machine's own. The link carries no name;
//!    the machine says it once paired. The field never shows a value that
//!    will be saved as a different name. A name that looks like the key
//!    or a link is called that, beside the field, as it is typed.
//!
//! Errors are shown beside the control they are about, not in the frame's
//! foot. The app reaches the machine afterwards as the CLI does, directly
//! or through its relay, with the device credential it stored.

#[cfg(all(test, target_os = "linux"))]
pub(crate) mod capture;

mod render;
pub(crate) use render::render;

use vessel_app_kit::TextField;
use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{Command, NameProblem, Offer, default_name, name_problem, read_link};
use vessel_app_view::pairing::{LinkProblem, PairProblem, PairedNotice, PreparedFailure};

use crate::routes::Route;
use crate::shell::Shell;

/// The pairing area's state.
pub(crate) struct Pairing {
    link: TextField,
    name: TextField,
    offer: Option<Offer>,
    link_error: Option<LinkProblem>,
    name_problem: Option<NameProblem>,
    pair_error: Option<PreparedFailure>,
    busy: bool,
    success_notice: Option<PairedNotice>,
    book_notice: Option<PreparedFailure>,
    /// The modal's focus (Escape closes it).
    dialog: DialogState,
}

impl Pairing {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            link: TextField::new(
                "",
                |shell: &mut Shell, text, window, cx| shell.read_offer(&text, window, cx),
                window,
                cx,
            ),
            name: TextField::with_change(
                "",
                |shell: &mut Shell, text, _window, cx| shell.pair(text, cx),
                |shell: &mut Shell, text, cx| {
                    // Empty is allowed: the machine's own name.
                    shell.areas.pair.name_problem = (!text.trim().is_empty())
                        .then(|| name_problem(&text, &shell.book_names()))
                        .flatten();
                    shell.areas.pair.pair_error = None;
                    cx.notify();
                },
                window,
                cx,
            ),
            offer: None,
            link_error: None,
            name_problem: None,
            pair_error: None,
            busy: false,
            success_notice: None,
            book_notice: None,
            dialog: DialogState::new(cx),
        }
    }
}

/// Focus the link's field.
pub(crate) fn arrive(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    // The new chat behind the modal, as arriving there makes it.
    crate::areas::draft::arrive(shell, window, cx);
    refresh_words(shell, window, cx);
    shell.areas.pair.link.focus(window, cx);
}

/// Translate cached field labels without touching the person's text or focus.
pub(crate) fn refresh_words(shell: &Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let words = shell.words.pairing();
    shell
        .areas
        .pair
        .link
        .set_placeholder(words.link_placeholder(), window, cx);
    shell
        .areas
        .pair
        .name
        .set_placeholder(words.name_placeholder(), window, cx);
}

/// Reword retained outcomes only at the language-change event, never on arrival
/// (an event may defer arrival to the next frame).
pub(crate) fn translate_outcomes(shell: &mut Shell) {
    if let Some(failure) = shell.areas.pair.pair_error.as_mut() {
        failure.translate(shell.words);
    }
    if let Some(notice) = shell.areas.pair.book_notice.as_mut() {
        let previous = notice.words().to_owned();
        notice.translate(shell.words);
        if shell.notice.as_deref() == Some(previous.as_str()) {
            shell.notice = Some(notice.words().to_owned());
        }
    }
    if let Some(notice) = shell.areas.pair.success_notice.as_mut() {
        let previous = notice.words().to_owned();
        match notice.translate(shell.words) {
            Ok(()) if shell.notice.as_deref() == Some(previous.as_str()) => {
                shell.notice = Some(notice.words().to_owned());
                if let Some((text, _address)) = shell.notice_detail.as_mut()
                    && *text == previous
                {
                    notice.words().clone_into(text);
                }
            }
            Ok(()) => {}
            Err(error) => tracing::error!(%error, "the paired status could not change language"),
        }
    }
}

impl Shell {
    pub(crate) fn pairing_opened(&mut self, cx: &mut Context<'_, Self>) {
        self.areas.pair.pair_error = None;
        cx.notify();
    }

    fn book_names(&self) -> Vec<String> {
        self.fleet
            .machines
            .iter()
            .map(|machine| machine.name.clone())
            .collect()
    }

    fn read_offer(&mut self, text: &str, window: &mut Window, cx: &mut Context<'_, Self>) {
        match read_link(text) {
            Ok(offer) => {
                let pair = &mut self.areas.pair;
                pair.offer = Some(offer);
                pair.link_error = None;
                pair.pair_error = None;
                pair.name_problem = None;
                // The link is held only as the offer, not left on screen.
                pair.link.clear(window, cx);
                self.areas.pair.name.clear(window, cx);
                self.areas.pair.name.focus(window, cx);
            }
            Err(error) => self.areas.pair.link_error = Some(LinkProblem::from(&error)),
        }
        cx.notify();
    }

    fn pair(&mut self, name: String, cx: &mut Context<'_, Self>) {
        let pair = &mut self.areas.pair;
        if pair.busy {
            return;
        }
        let Some(offer) = pair.offer.clone() else {
            pair.link_error = Some(LinkProblem::Missing);
            cx.notify();
            return;
        };
        // Empty uses the machine's own valid, free book name when it answers;
        // otherwise the app supplies a free fallback. Typed names stay typed.
        let prefer_node_name = name.trim().is_empty();
        let name = if prefer_node_name {
            default_name(&self.book_names())
        } else {
            if let Some(problem) = name_problem(&name, &self.book_names()) {
                self.areas.pair.name_problem = Some(problem);
                cx.notify();
                return;
            }
            name
        };
        let pair = &mut self.areas.pair;
        pair.busy = true;
        pair.pair_error = None;
        self.command(Command::Pair {
            offer,
            name,
            prefer_node_name,
        });
        cx.notify();
    }

    /// Cancel, the close button or Escape: the offer is dropped, the
    /// fields emptied, and the new chat stays.
    fn close_pairing(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let pair = &mut self.areas.pair;
        if pair.busy {
            return;
        }
        pair.offer = None;
        pair.link_error = None;
        pair.pair_error = None;
        pair.name_problem = None;
        pair.link.clear(window, cx);
        self.go(Route::Draft, window, cx);
    }

    fn pair_from_field(&mut self, cx: &mut Context<'_, Self>) {
        let name = self.areas.pair.name.text(cx);
        self.pair(name, cx);
    }

    /// Pairing worked: the draft next, to start working there.
    pub(crate) fn paired(
        &mut self,
        node: &str,
        via: vessel_app_view::Via,
        address: String,
        cx: &mut Context<'_, Self>,
    ) {
        let pair = &mut self.areas.pair;
        pair.busy = false;
        pair.offer = None;
        match PairedNotice::new(node, via, self.words) {
            Ok(notice) => {
                let text = notice.words().to_owned();
                pair.success_notice = Some(notice);
                self.notice_detail = Some((text.clone(), address));
                self.notice = Some(text);
            }
            Err(error) => tracing::error!(%error, "the paired status could not be formatted"),
        }
        self.route_to(Route::Draft);
        self.settle_draft();
        cx.notify();
    }

    /// The machine list does not read: the app's words for where and what
    /// (never the file's text: it may hold a token) in the frame's foot,
    /// and beside the pairing form when a pairing was under way.
    pub(crate) fn book_unreadable(&mut self, fault: &vessel_app_view::files::FileFault) {
        let failure = PreparedFailure::new(PairProblem::Book(fault.clone()), self.words);
        let words = failure.words().to_owned();
        self.areas.pair.book_notice = Some(failure);
        if self.areas.pair.busy {
            self.pair_failed(PairProblem::Book(fault.clone()));
        }
        self.notice = Some(words);
    }

    pub(crate) fn pair_failed(&mut self, why: PairProblem) {
        let pair = &mut self.areas.pair;
        pair.busy = false;
        pair.pair_error = Some(PreparedFailure::new(why, self.words));
    }
}
