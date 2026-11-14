//! Paseo's branch switcher (`components/branch-switcher.tsx`: a
//! searchable list titled "Switch branch", 280 wide, each name led by a
//! 14-point branch glyph, the current one checked; choosing the current
//! one closes it, `hooks/use-branch-switcher.ts`), and the page's
//! branch-name form for a new branch or a rename (`branch-dialog.tsx`),
//! which Paseo has no form for.

use vessel_app_kit::dialog::{DialogContent, DialogSize};
use vessel_app_kit::picker::{ComboHandlers, ComboHeader, ComboRow, Leading, Side};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, assets::Glyph, breakpoint,
    button_with_kind, combobox,
};
use vessel_app_link::workspace::GitWrite;
use vessel_app_view::workspace::git::{
    BranchChoice, BranchChoices, BranchMode, BranchRequest, BranchSelection, Issue, WriteIntent,
    branch_choices, branch_request, branch_selection, can_branch, stash_for,
};

use super::{Open, menu::issue_alert, name};
use crate::shell::Shell;

/// Paseo's `desktopMinWidth`.
const SWITCHER_WIDTH: f32 = 280.0;

/// The widest the list grows to follow a wide header.
const SWITCHER_WIDEST: f32 = 420.0;

/// A switcher row's parts besides its name (the kit's combobox row): the
/// list's 4-point inset each side, the row's 12-point insets, the 14-point
/// branch glyph, two 8-point gaps and the 16-point check, and two points
/// of margin.
const ROW_CHROME: f32 = 80.0;

/// The rows the switcher shows for `query`, with their branches.
fn choices(shell: &Shell, query: &str) -> Vec<BranchChoice> {
    let git = &shell.areas.git;
    match branch_choices(&git.branches, &git.status, &git.writing, query) {
        BranchChoices::Rows { rows, .. } => rows,
        BranchChoices::Waiting | BranchChoices::Failed(_) => Vec::new(),
    }
}

/// The branch list, when open.
pub(super) fn switcher(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let git = &shell.areas.git;
    if !git.switcher.is_open() {
        return None;
    }
    let lang = shell.t.lang;
    let labels = lang.git_labels();
    let query = git.switcher.query(cx);
    let read = branch_choices(&git.branches, &git.status, &git.writing, &query);
    let (rows, empty) = match read {
        BranchChoices::Waiting => (Vec::new(), labels.reading_branches.to_owned()),
        BranchChoices::Failed(problem) => (Vec::new(), lang.git_call_failure(problem).title),
        BranchChoices::Rows { rows, .. } => (rows, labels.empty_branches.to_owned()),
    };
    // The list is at least as wide as the branch name's box in the header
    // (Paseo's 280 at the least, about 420 at the most), so two names that
    // share both ends still read apart; the window's width as a sheet
    // below `COMPACT`. Each name is one line cut in its middle (`name.rs`),
    // the whole name in the row's tooltip when cut.
    let width = if window.viewport_size().width < px(vessel_app_kit::picker::COMPACT) {
        f32::from(window.viewport_size().width)
    } else {
        git.branch_room.get().map_or(SWITCHER_WIDTH, |bounds| {
            f32::from(bounds.size.width).clamp(SWITCHER_WIDTH, SWITCHER_WIDEST)
        })
    };
    let room = ROW_CHROME.mul_add(-1.0, width);
    let rows: Vec<ComboRow> = rows
        .iter()
        .map(|choice| {
            let whole = choice.branch.name.as_str();
            let shown = name::one_line(window, whole, Type::Body, room);
            ComboRow {
                leading: Some(Leading::SmallGlyph(Glyph::Branch)),
                checked: choice.selected,
                disabled: !choice.enabled,
                tooltip: shown.cut.then(|| whole.to_owned().into()),
                ..ComboRow::new(shown.text)
            }
        })
        .collect();
    let header = ComboHeader {
        title: Some(labels.switch_title.into()),
        back: false,
        search: true,
        empty: empty.into(),
        note: None,
        side: Side::Below,
    };
    let handlers = ComboHandlers {
        state: |shell: &mut Shell| &mut shell.areas.git.switcher,
        chose: std::rc::Rc::new(|shell: &mut Shell, index, window, cx| {
            shell.git_switch_chosen(index, window, cx);
        }),
        back: None,
    };
    combobox(
        "git-switcher",
        &git.switcher,
        &header,
        &rows,
        &handlers,
        width,
        tokens,
        window,
        cx,
    )
}

impl Shell {
    /// The branch name was pressed: open the list (reading it), or close it.
    pub(super) fn git_toggle_switcher(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.git.switcher.is_open() {
            self.areas.git.switcher.close();
        } else {
            self.git_open_switcher(window, cx);
        }
        cx.notify();
    }

    /// Open the branch list and read it.
    pub(super) fn git_open_switcher(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let placeholder = self.t.lang.git_labels().filter;
        let git = &mut self.areas.git;
        git.menu.close();
        git.switcher.open(Some(placeholder), window, cx);
        self.git_read_branches();
    }

    /// Row `index` of the list was chosen.
    fn git_switch_chosen(&mut self, index: usize, window: &mut Window, cx: &mut Context<'_, Self>) {
        let query = self.areas.git.switcher.query(cx);
        let Some(choice) = choices(self, &query).into_iter().nth(index) else {
            return;
        };
        let git = &mut self.areas.git;
        let (Some(target), Some(status)) = (git.target.clone(), git.status.shown()) else {
            return;
        };
        git.switcher.close();
        if !choice.enabled {
            return;
        }
        match branch_selection(target.root, &choice.branch, status.branch.as_ref()) {
            Ok(BranchSelection::Current) => {}
            Ok(BranchSelection::Switch(switch)) => {
                self.git_write(GitWrite::Switch(switch), WriteIntent::SwitchBranch, cx);
            }
            Err(problem) => git.issue = Some(Issue::Switch(problem)),
        }
        window.refresh();
        cx.notify();
    }

    /// Open the branch name form.
    pub(super) fn git_open_branch(
        &mut self,
        mode: BranchMode,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let labels = self.t.lang.git_labels();
        let placeholder = match mode {
            BranchMode::Create => labels.branch_name,
            BranchMode::Rename => labels.new_name,
        };
        let current = self
            .areas
            .git
            .status
            .shown()
            .and_then(|status| status.branch.as_ref())
            .map(|branch| branch.as_str().to_owned());
        let git = &mut self.areas.git;
        git.menu.close();
        git.issue = None;
        git.open = Some(Open::Branch(mode));
        git.name.set_placeholder(placeholder, window, cx);
        // A rename starts from the current name, as the page's form does.
        let start = match mode {
            BranchMode::Create => String::new(),
            BranchMode::Rename => current.unwrap_or_default(),
        };
        git.name.set_text(&start, window, cx);
        git.dialog.focus(window, cx);
        git.name.focus(window, cx);
    }

    /// The name form's Enter or its button.
    pub(super) fn git_branch_submitted(
        &mut self,
        name: &str,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let git = &mut self.areas.git;
        let (Some(Open::Branch(mode)), Some(target)) = (git.open.clone(), git.target.clone())
        else {
            return;
        };
        if !can_branch(&git.status, &git.writing, mode, name, target.root) {
            if let Err(error) = branch_request(target.root, mode, name) {
                git.issue = Some(Issue::Name(error));
            }
            cx.notify();
            return;
        }
        match branch_request(target.root, mode, name) {
            Ok(BranchRequest::Create(switch)) => {
                self.git_write(GitWrite::Switch(switch), WriteIntent::CreateBranch, cx);
            }
            Ok(BranchRequest::Rename(rename)) => {
                self.git_write(
                    GitWrite::RenameBranch(rename),
                    WriteIntent::RenameBranch,
                    cx,
                );
            }
            Err(error) => self.areas.git.issue = Some(Issue::Name(error)),
        }
        cx.notify();
    }
}

/// The branch name form, when open.
pub(super) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let git = &shell.areas.git;
    let Some(Open::Branch(mode)) = git.open else {
        return None;
    };
    let lang = shell.t.lang;
    let labels = lang.git_labels();
    let narrow = breakpoint::phone(window);
    let (title, field, confirm) = match mode {
        BranchMode::Create => (labels.create_title, labels.branch_name, labels.create),
        BranchMode::Rename => (labels.rename_title, labels.new_name, labels.rename),
    };
    let root = git.target.as_ref().map(|target| target.root)?;
    let name = git.name.text(cx);
    let ready = can_branch(&git.status, &git.writing, mode, &name, root);
    let busy = git.writing.busy();
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(field),
        )
        .child(git.name.element(tokens, narrow, cx))
        .children(
            git.issue
                .as_ref()
                .map(|issue| issue_alert(lang, issue, tokens)),
        )
        .into_any_element();
    let footer = vec![
        button_with_kind(
            "git-branch-cancel",
            labels.cancel,
            ButtonKind::Secondary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.git_close(window, cx)))
        .into_any_element(),
        {
            let label = match git.writing.intent() {
                Some(intent) if busy => lang.git_pending(intent),
                Some(_) | None => confirm,
            };
            let state = if ready {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            };
            let button = button_with_kind(
                "git-branch-confirm",
                label,
                ButtonKind::Primary,
                state,
                tokens,
            );
            if ready {
                button
                    .on_click(cx.listener(|shell, _event, window, cx| {
                        let name = shell.areas.git.name.text(cx);
                        shell.git_branch_submitted(&name, window, cx);
                    }))
                    .into_any_element()
            } else {
                button.into_any_element()
            }
        },
    ];
    Some(vessel_app_kit::dialog::dialog(
        &git.dialog,
        DialogContent {
            title: title.into(),
            body,
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::git_close,
        cx,
    ))
}

impl Shell {
    /// "Stash & Switch" was confirmed: save, then switch once it is saved.
    fn git_stash_and_switch(&mut self, cx: &mut Context<'_, Self>) {
        let git = &mut self.areas.git;
        let Some(switch) = git.confirm_switch.take() else {
            return;
        };
        let (stash, intent) = stash_for(&switch);
        git.then_switch = Some(switch);
        self.git_write(GitWrite::Stash(stash), intent, cx);
    }
}

/// Paseo's confirm before stashing to switch (`use-branch-switcher.ts`).
pub(super) fn switch_confirm(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let git = &shell.areas.git;
    let _waiting = git.confirm_switch.as_ref()?;
    let labels = shell.t.lang.git_labels();
    let narrow = breakpoint::phone(window);
    let body = div()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground_muted)
        .child(labels.switch_dirty_message)
        .into_any_element();
    let footer = vec![
        button_with_kind(
            "git-switch-cancel",
            labels.cancel,
            ButtonKind::Secondary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.git_close(window, cx)))
        .into_any_element(),
        button_with_kind(
            "git-switch-stash",
            labels.stash_and_switch,
            ButtonKind::Primary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, _window, cx| shell.git_stash_and_switch(cx)))
        .into_any_element(),
    ];
    Some(vessel_app_kit::dialog::dialog(
        &git.dialog,
        DialogContent {
            title: labels.switch_dirty_title.into(),
            body,
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::git_close,
        cx,
    ))
}
