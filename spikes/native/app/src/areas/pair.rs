//! Adding a machine by pairing (ADR 0010), in two steps a person cannot
//! mix up (the owner typed the key into the name field of build
//! 0.0.0.0.1, whose screen read as if it asked for it):
//!
//! 1. The link: pasted, decoded, cleared from its field (it is a
//!    single-use secret).
//! 2. The key shown large, with nothing to type, and one sentence: it
//!    must be the same as the key the machine printed with the link. Then
//!    the name, under a label that stays, filled in with a name the book
//!    does not have, so Pair works at once. A name that looks like the key
//!    or a link is called that, beside the field, as it is typed.
//!
//! Errors are shown beside the control they are about, not in the frame's
//! foot. The app reaches the machine afterwards as the CLI does, directly
//! or through its relay, with the device credential it stored.

use vessel_app_kit::dialog::{DialogContent, DialogSize, DialogState, dialog};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{TextField, Tokens, Type, TypeExt as _, button, settings};
use vessel_app_link::{Command, NameProblem, Offer, default_name, name_problem, read_link};

use crate::routes::Route;
use crate::shell::Shell;

/// The pairing area's state.
pub(crate) struct Pairing {
    link: TextField,
    name: TextField,
    offer: Option<Offer>,
    link_error: Option<String>,
    name_problem: Option<NameProblem>,
    pair_error: Option<String>,
    busy: bool,
    /// The name the app filled in; kept as it is, the node's own name is
    /// used instead once pairing says it.
    prefilled: Option<String>,
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
                    shell.areas.pair.name_problem = name_problem(&text, &shell.book_names());
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
            prefilled: None,
            dialog: DialogState::new(cx),
        }
    }
}

/// Focus the link's field.
pub(crate) fn arrive(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    // The new chat behind the modal, as arriving there makes it.
    crate::areas::draft::arrive(shell, window, cx);
    let t = shell.t;
    shell
        .areas
        .pair
        .link
        .set_placeholder(t.link_placeholder, window, cx);
    shell
        .areas
        .pair
        .name
        .set_placeholder(t.name_hint, window, cx);
    shell.areas.pair.link.focus(window, cx);
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
                let name = default_name(&self.book_names());
                self.areas.pair.name.set_text(&name, window, cx);
                self.areas.pair.prefilled = Some(name);
                self.areas.pair.name.focus(window, cx);
            }
            Err(error) => self.areas.pair.link_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn pair(&mut self, name: String, cx: &mut Context<'_, Self>) {
        let pair = &mut self.areas.pair;
        if pair.busy {
            return;
        }
        let Some(offer) = pair.offer.clone() else {
            pair.link_error = Some(self.t.paste_first.to_owned());
            cx.notify();
            return;
        };
        if let Some(problem) = name_problem(&name, &self.book_names()) {
            self.areas.pair.name_problem = Some(problem);
            cx.notify();
            return;
        }
        let pair = &mut self.areas.pair;
        pair.busy = true;
        pair.pair_error = None;
        let prefer_node_name = pair.prefilled.as_deref() == Some(name.trim());
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
        let notice = self.t.paired(node, via);
        self.notice_detail = Some((notice.clone(), address));
        self.notice = Some(notice);
        self.route_to(Route::Draft);
        self.settle_draft();
        cx.notify();
    }

    /// The machine list does not read: the app's words for where and what
    /// (never the file's text: it may hold a token) in the frame's foot,
    /// and beside the pairing form when a pairing was under way.
    pub(crate) fn book_unreadable(&mut self, fault: &vessel_app_view::files::FileFault) {
        let words = self.t.lang.machines_unreadable(fault);
        if self.areas.pair.busy {
            self.pair_failed(words.clone());
        }
        self.notice = Some(words);
    }

    pub(crate) fn pair_failed(&mut self, why: String) {
        let pair = &mut self.areas.pair;
        pair.busy = false;
        pair.pair_error = Some(why);
    }
}

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    // Paseo pairs in a modal over the screen it was asked from: here the
    // new chat, where pairing leads.
    let narrow = crate::areas::draft::compact(window);
    let modal = match &shell.areas.pair.offer {
        None => link_step(shell, tokens, window, narrow, cx),
        Some(offer) => confirm_step(shell, offer, tokens, window, narrow, cx),
    };
    crate::areas::draft::render(shell, tokens, window, cx).child(modal)
}

/// The first step, Paseo's paste-link modal: what to paste, the field,
/// why it is a secret, Cancel and Pair.
fn link_step(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.pairing();
    let pair = &shell.areas.pair;
    let neutral = tokens.neutral;
    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(words.helper),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(field_label(words.label, tokens))
                .child(pair.link.element(tokens, narrow, cx))
                .children(pair.link_error.clone().map(|error| beside(error, tokens))),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(words.security_warning),
        )
        .child(actions(
            words.cancel,
            button("pair-link", words.pair, true, tokens).on_click(cx.listener(
                |this, _event, window, cx| {
                    let text = this.areas.pair.link.text(cx);
                    this.read_offer(&text, window, cx);
                },
            )),
            tokens,
            cx,
        ));
    modal(shell, words.title, body, tokens, window, narrow, cx)
}

/// The second step, Paseo's host confirmation: what connecting allows, the
/// host, its key's fingerprint (the machine printed the same with the
/// link) and its relay in a settings card; then the name the book will
/// call it (Vessel's), Cancel and Connect.
fn confirm_step(
    shell: &Shell,
    offer: &Offer,
    tokens: &Tokens,
    window: &Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let t = shell.t;
    let words = t.lang.pairing();
    let pair = &shell.areas.pair;
    let neutral = tokens.neutral;
    let card = settings::card(tokens)
        // The machine's id and its key's fingerprint in the code face (each
        // character told from its neighbours, as the machine printed them).
        .child(settings::code_row(words.host, offer.node.clone(), true, tokens))
        .child(settings::code_row(
            words.fingerprint,
            offer.key.clone(),
            false,
            tokens,
        ))
        .child(settings::row(
            words.relay,
            Some(offer.routes.join(", ").into()),
            None,
            false,
            tokens,
        ));
    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground_muted)
                .child(words.confirm_description),
        )
        .child(card)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(field_label(t.name_label, tokens))
                .child(pair.name.element(tokens, narrow, cx))
                .child(match pair.name_problem {
                    Some(problem) => beside(problem_words(problem, shell).to_owned(), tokens),
                    None => div()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .child(t.name_hint),
                }),
        )
        .children(pair.pair_error.clone().map(|error| beside(error, tokens)))
        .child(actions(
            words.cancel,
            button(
                "pair-connect",
                if pair.busy {
                    words.pairing
                } else {
                    words.connect
                },
                true,
                tokens,
            )
            .on_click(cx.listener(|this, _event, _window, cx| this.pair_from_field(cx))),
            tokens,
            cx,
        ));
    modal(shell, words.confirm_title, body, tokens, window, narrow, cx)
}

/// The modal (the kit's dialog: Paseo's adaptive modal sheet).
fn modal(
    shell: &Shell,
    title: &'static str,
    body: Div,
    tokens: &Tokens,
    window: &Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    dialog(
        &shell.areas.pair.dialog,
        DialogContent {
            title: title.into(),
            body: body.into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        |shell: &mut Shell, window, cx| shell.close_pairing(window, cx),
        cx,
    )
}

/// A field's label (Paseo's: 14, medium, muted).
fn field_label(text: &'static str, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Label)
        .text_color(tokens.neutral.foreground_muted)
        .child(text)
}

/// Cancel and the step's own button, side by side and as wide (Paseo's
/// `actions`: 12 between, 8 above).
fn actions(
    cancel: &'static str,
    go: vessel_app_kit::ui::Stateful<Div>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    div()
        .flex()
        .flex_row()
        .gap(px(12.0))
        .mt(px(8.0))
        .child(
            button("pair-cancel", cancel, false, tokens)
                .flex_1()
                .on_click(cx.listener(|this, _event, window, cx| this.close_pairing(window, cx))),
        )
        .child(go.flex_1())
}

const fn problem_words(problem: NameProblem, shell: &Shell) -> &'static str {
    let t = shell.t;
    match problem {
        NameProblem::Empty => t.name_empty,
        NameProblem::LooksLikeKey => t.name_is_key,
        NameProblem::LooksLikeLink => t.name_is_link,
        NameProblem::Invalid => t.name_invalid,
        NameProblem::Taken => t.name_taken,
    }
}

/// An error, beside the control it is about (Paseo's: 14, destructive).
fn beside(text: String, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Body)
        .text_color(tokens.neutral.destructive)
        .child(text)
}
