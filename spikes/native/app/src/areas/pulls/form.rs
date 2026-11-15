//! The "Create pull request" form: the title, the description, the base
//! and draft, then Cancel and Create PR (Paseo's labels: "Create PR",
//! "Creating PR..."). Paseo sends no form (its daemon writes the text with
//! a model); the fields are `gh pr create`'s, with its `--fill` defaults
//! (`view::workspace::pull::prefill`).

use vessel_app_kit::alert;
use vessel_app_kit::dialog::{DialogContent, DialogSize};
use vessel_app_kit::picker::{ComboHandlers, ComboRow};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, breakpoint, button_with_kind, combobox,
    select,
};
use vessel_app_view::workspace::git::Issue;
use vessel_app_view::workspace::pull::{BaseChoice, base_choices, refusal_words, start_base};
use vessel_proto::forge::OpenRefusal;
use vessel_proto::workspace::BranchName;

use super::{Form, FormIssue};
use crate::shell::Shell;

/// A forge's words show their first lines and no more, as the git row's
/// failure detail does, so the dialog keeps its size.
const OUTPUT_MOST: f32 = 160.0;

impl Shell {
    /// A base was chosen from the select's list.
    fn pulls_base_chosen(&mut self, choice: BaseChoice, cx: &mut Context<'_, Self>) {
        let pulls = &mut self.areas.pulls;
        pulls.bases.close();
        if let Some(form) = pulls.form.as_mut() {
            form.draft.base = choice.branch;
            form.base_chosen = true;
        }
        cx.notify();
    }

    /// The draft switch.
    fn pulls_toggle_draft(&mut self, cx: &mut Context<'_, Self>) {
        let pulls = &mut self.areas.pulls;
        if let Some(form) = pulls.form.as_mut().filter(|form| form.sending.is_none()) {
            form.draft.draft = !form.draft.draft;
            pulls.draft_turn = pulls.draft_turn.wrapping_add(1);
        }
        cx.notify();
    }
}

/// The base choices for the open form.
fn choices(shell: &Shell, form: &Form) -> Vec<BaseChoice> {
    let head = form.branch.parse().ok();
    base_choices(shell.areas.git.branches.shown(), head.as_ref())
}

/// The base the form shows and sends: the person's choice, else the one
/// the branch was made from as the forge spells it (`start_base`, read
/// again each frame so a branch list that arrives late corrects it).
pub(crate) fn form_base(shell: &Shell, form: &Form) -> Option<BranchName> {
    if form.base_chosen {
        form.draft.base.clone()
    } else {
        start_base(form.proposed.as_ref(), shell.areas.git.branches.shown())
    }
}

/// A choice's words: the branch, or "Default branch".
fn base_label(shell: &Shell, branch: Option<&BranchName>) -> String {
    branch.map_or_else(
        || shell.words.pulls().default_base().to_owned(),
        ToString::to_string,
    )
}

/// A field's label above its control.
fn label(text: &str, tokens: &Tokens) -> AnyElement {
    div()
        .type_(Type::Body)
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(tokens.neutral.foreground)
        .child(text.to_owned())
        .into_any_element()
}

/// The form, when open.
pub(crate) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let pulls = &shell.areas.pulls;
    let form = pulls.form.as_ref()?;
    if form.fill.is_some() {
        cx.defer_in(window, Shell::pulls_fill);
    }
    let words = shell.words.pulls();
    let narrow = breakpoint::phone(window);
    let sending = form.sending.is_some();
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S4))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .child(label(words.field_title(), tokens))
                .child(pulls.title.element(tokens, narrow, cx)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .child(label(words.field_body(), tokens))
                .child(pulls.body.element(tokens, narrow, cx)),
        )
        .child(base_row(shell, form, tokens, narrow, cx))
        .child(draft_row(shell, form, tokens, narrow, cx))
        .children(
            form.issue
                .as_ref()
                .map(|issue| issue_alert(shell, issue, tokens)),
        )
        .into_any_element();
    let confirm = if sending {
        words.creating()
    } else {
        words.create()
    };
    let state = if sending {
        ButtonState::Disabled
    } else {
        ButtonState::Enabled
    };
    let create = button_with_kind("pulls-create", confirm, ButtonKind::Primary, state, tokens);
    let create = if sending {
        create.into_any_element()
    } else {
        create
            .on_click(cx.listener(|shell, _event, window, cx| shell.pulls_submit(window, cx)))
            .into_any_element()
    };
    let footer = vec![
        button_with_kind(
            "pulls-cancel",
            words.cancel(),
            ButtonKind::Secondary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.pulls_close(window, cx)))
        .into_any_element(),
        create,
    ];
    Some(vessel_app_kit::dialog::dialog(
        &pulls.dialog,
        DialogContent {
            title: words.form_title().into(),
            body,
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::pulls_close,
        cx,
    ))
}

/// The base, as the kit's select.
fn base_row(
    shell: &Shell,
    form: &Form,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let pulls = &shell.areas.pulls;
    let trigger = select::trigger(
        "pulls-base",
        base_label(shell, form_base(shell, form).as_ref()),
        &pulls.base_anchor,
        pulls.bases.is_open(),
        form.sending.is_none(),
        narrow,
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| {
        let pulls = &mut shell.areas.pulls;
        if pulls.bases.is_open() {
            pulls.bases.close();
        } else {
            let bounds = pulls.base_anchor.get();
            pulls.bases.set_anchor(bounds);
            pulls.bases.open(None, window, cx);
        }
        cx.notify();
    }));
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(spacing::S2))
        .child(label(shell.words.pulls().field_base(), tokens))
        .child(trigger)
        .into_any_element()
}

/// The base's list, when open (over the form).
pub(crate) fn base_list(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let pulls = &shell.areas.pulls;
    let form = pulls.form.as_ref()?;
    if !pulls.bases.is_open() {
        return None;
    }
    let choices = choices(shell, form);
    let shown = form_base(shell, form);
    let rows: Vec<ComboRow> = choices
        .iter()
        .map(|choice| ComboRow {
            checked: choice.branch == shown,
            ..ComboRow::new(base_label(shell, choice.branch.as_ref()))
        })
        .collect();
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.pulls.bases,
        chose: std::rc::Rc::new(move |shell: &mut Shell, index, _window, cx| {
            if let Some(choice) = choices.get(index) {
                shell.pulls_base_chosen(choice.clone(), cx);
            }
        }),
        back: None,
    };
    combobox(
        "pulls-base-list",
        &pulls.bases,
        &select::header(),
        &rows,
        &handlers,
        select::WIDTH,
        tokens,
        window,
        cx,
    )
}

/// Draft, as Paseo's settings row: the words left, the switch right.
fn draft_row(
    shell: &Shell,
    form: &Form,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let pulls = &shell.areas.pulls;
    let enabled = form.sending.is_none();
    div()
        .id("pulls-draft-row")
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(spacing::S3))
        .cursor_pointer()
        .child(label(shell.words.pulls().field_draft(), tokens))
        .child(vessel_app_kit::settings::switch(
            "pulls-draft",
            form.draft.draft,
            enabled,
            touch,
            pulls.draft_turn,
            tokens,
        ))
        .on_click(cx.listener(|shell, _event, _window, cx| shell.pulls_toggle_draft(cx)))
        .into_any_element()
}

/// Why the form did not send: its own rule, the node's refusal (a push's
/// git refusal and the forge's message beneath), or the call.
fn issue_alert(shell: &Shell, issue: &FormIssue, tokens: &Tokens) -> AnyElement {
    let words = shell.words;
    let lang = shell.t.lang;
    let (title, detail): (String, Option<AnyElement>) = match issue {
        FormIssue::Form(problem) => (problem.words(words).to_owned(), None),
        FormIssue::Refused(OpenRefusal::Push { refusal, .. }) => {
            let text = lang.git_issue(&Issue::Refused(refusal.clone()));
            (
                refusal_words(
                    &OpenRefusal::Push {
                        refusal: refusal.clone(),
                    },
                    words,
                )
                .into_owned(),
                Some(crate::areas::git::failure_body(&text, tokens)),
            )
        }
        FormIssue::Refused(refusal) => {
            let message = match refusal {
                OpenRefusal::Rejected { message, .. } => Some(mono(message, tokens)),
                OpenRefusal::NotPushed
                | OpenRefusal::NoCommits
                | OpenRefusal::Push { .. }
                | OpenRefusal::Access { .. }
                | OpenRefusal::Unknown => None,
            };
            (refusal_words(refusal, words).into_owned(), message)
        }
        FormIssue::Call(problem) => {
            let text = lang.git_issue(&Issue::Call(problem.clone()));
            (
                words.pulls().failed().to_owned(),
                Some(crate::areas::git::failure_body(&text, tokens)),
            )
        }
    };
    alert::alert(
        alert::Content {
            title: Some(title.into()),
            description: detail,
            action: None,
        },
        alert::Variant::Error,
        alert::Size::Small,
        tokens,
    )
    .into_any_element()
}

/// A forge's own words, as the git row shows a hook's output.
fn mono(text: &str, tokens: &Tokens) -> AnyElement {
    div()
        .max_h(px(OUTPUT_MOST))
        .overflow_hidden()
        .type_(Type::Mono)
        .text_color(tokens.neutral.foreground)
        .whitespace_normal()
        .child(text.to_owned())
        .into_any_element()
}
