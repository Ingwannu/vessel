//! Modal entity rename keeps its text on refusal and correlates cancelled forms.
use super::with_layout;
use crate::shell::Shell;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, Window, div, px,
};
use vessel_app_kit::{DialogContent, DialogSize, Editability, TextField, Tokens, button, dialog};
use vessel_app_view::layout::{
    HostAlias, LayoutError, RenamePhase, RenameProblem, RenameSubmission, RenameTarget, TabId,
    Target,
};
use vessel_proto::InvalidTitle;

pub(super) struct Rename {
    workspace: vessel_app_view::layout::LayoutKey,
    machine: HostAlias,
    node: vessel_proto::NodeId,
    target: RenameTarget,
    field: TextField,
    phase: RenamePhase,
    invalid: Option<InvalidTitle>,
}
impl Rename {
    #[cfg(test)]
    pub(super) const fn workspace(&self) -> &vessel_app_view::layout::LayoutKey {
        &self.workspace
    }
}
impl Shell {
    pub(super) fn layout_start_rename(
        &mut self,
        tab: TabId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        // The legacy route cannot supply original-node authority until its
        // lifecycle binding adapter lands. Never bless it from the latest fleet.
        let Ok(workspace @ vessel_app_view::layout::LayoutKey::Workspace { node, .. }) =
            super::key(self)
        else {
            return;
        };
        let target = with_layout(self, |layout| {
            layout
                .tab(tab)
                .map(|tab| tab.target().clone())
                .ok_or(LayoutError::MissingTab)
        });
        let Ok(Target::Agent(session)) = target else {
            return;
        };
        let Some(open) = self
            .areas
            .session
            .open
            .as_ref()
            .filter(|open| open.session == session)
        else {
            return;
        };
        let (Ok(machine), Ok(ticket)) = (
            HostAlias::parse(&open.machine),
            self.areas.layout.rename_ticket.next(),
        ) else {
            self.layout_result(Err(LayoutError::Exhausted), cx);
            return;
        };
        let field = TextField::new(
            self.words.layout().rename_field(),
            Self::layout_rename_entered,
            window,
            cx,
        );
        field.set_text(&open.title, window, cx);
        self.areas.layout.rename_dialog.focus(window, cx);
        field.focus(window, cx);
        self.areas.layout.rename_ticket = ticket;
        self.areas.layout.rename = Some(Rename {
            workspace,
            machine,
            node,
            target: RenameTarget::Agent(session),
            field,
            phase: RenamePhase::Editing,
            invalid: None,
        });
        cx.notify();
    }
    fn layout_rename_entered(
        &mut self,
        _text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.layout_submit_rename(window, cx);
    }
    fn layout_submit_rename(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        let Ok(ticket) = self.areas.layout.rename_ticket.next() else {
            self.layout_result(Err(LayoutError::Exhausted), cx);
            return;
        };
        let Some(form) = self.areas.layout.rename.as_mut() else {
            return;
        };
        if matches!(form.phase, RenamePhase::Sending(_)) {
            return;
        }
        let title = match vessel_app_view::session::check_title(&form.field.text(cx)) {
            Ok(title) => title,
            Err(error) => {
                form.invalid = Some(error);
                cx.notify();
                return;
            }
        };
        let submission = RenameSubmission {
            ticket,
            target: form.target,
            title,
        };
        form.invalid = None;
        form.phase = RenamePhase::Sending(submission.clone());
        form.field.set_editability(Editability::ReadOnly, cx);
        let ask = vessel_app_link::workspace::layout::Ask {
            node: form.node,
            machine: form.machine.clone(),
            submission,
        };
        self.areas.layout.rename_ticket = ticket;
        self.command(vessel_app_link::Command::Workspace {
            machine: ask.machine.as_str().to_owned(),
            ask: vessel_app_link::workspace::Ask::Layout(ask),
        });
        cx.notify();
    }
    pub(crate) fn layout_renamed(
        &mut self,
        answer: vessel_app_link::workspace::layout::Answer,
        cx: &mut Context<'_, Self>,
    ) {
        let active_workspace = super::key(self).ok();
        let active_node = match &active_workspace {
            Some(vessel_app_view::layout::LayoutKey::Workspace { node, .. }) => Some(*node),
            Some(vessel_app_view::layout::LayoutKey::Session { .. }) | None => None,
        };
        let Some(form) = self.areas.layout.rename.as_mut() else {
            return;
        };
        if form.machine != answer.ask.machine
            || form.node != answer.ask.node
            || !form.phase.answered(
                answer.ask.submission.ticket,
                answer.ask.submission.target,
                answer
                    .result
                    .as_ref()
                    .map(|_| ())
                    .map_err(|problem| *problem),
            )
        {
            return;
        }
        form.field.set_editability(Editability::Editable, cx);
        if let Ok(title) = answer.result {
            if let RenameTarget::Agent(session) = form.target {
                for machine in &mut self.fleet.machines {
                    if machine.name == form.machine.as_str() && machine.node == Some(form.node) {
                        for card in &mut machine.sessions {
                            if card.session == session {
                                title.as_str().clone_into(&mut card.title);
                            }
                        }
                    }
                }
                if let Some(open) = self.areas.session.open.as_mut().filter(|open| {
                    open.session == session
                        && open.machine == form.machine.as_str()
                        && active_node == Some(form.node)
                        && active_workspace.as_ref() == Some(&form.workspace)
                }) {
                    title.as_str().clone_into(&mut open.title);
                }
            }
            self.areas.layout.rename = None;
        }
        cx.notify();
    }
    fn layout_cancel_rename(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.layout.rename = None;
        cx.notify();
    }
}
pub(super) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let form = shell.areas.layout.rename.as_ref()?;
    let words = shell.words.layout();
    let busy = matches!(form.phase, RenamePhase::Sending(_));
    let problem = match &form.phase {
        RenamePhase::Refused(RenameProblem::NodeChanged) => Some(words.rename_node_changed()),
        RenamePhase::Refused(RenameProblem::Offline) => Some(words.rename_offline()),
        RenamePhase::Refused(RenameProblem::TimedOut) => Some(words.rename_timeout()),
        RenamePhase::Refused(RenameProblem::Busy) => Some(words.rename_queue()),
        RenamePhase::Refused(RenameProblem::Node(_)) => Some(words.rename_failed()),
        RenamePhase::Refused(RenameProblem::MalformedResponse) => Some(words.rename_malformed()),
        RenamePhase::Editing | RenamePhase::Sending(_) | RenamePhase::Complete => None,
    };
    let invalid = form
        .invalid
        .as_ref()
        .map(|error| shell.t.lang.conversation().title_refused(*error));
    let body = div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(
            form.field
                .element(tokens, vessel_app_kit::breakpoint::phone(window), cx),
        )
        .children(problem.map(|text| {
            div()
                .text_color(tokens.neutral.destructive)
                .child(text.to_owned())
        }))
        .children(invalid.map(|text| div().text_color(tokens.neutral.destructive).child(text)));
    let mut footer = vec![
        button(
            "layout-rename-cancel",
            words.cancel().to_owned(),
            false,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.layout_cancel_rename(window, cx)))
        .into_any_element(),
    ];
    if busy {
        footer.push(
            div()
                .child(words.rename_busy().to_owned())
                .into_any_element(),
        );
    } else {
        footer.push(
            button(
                "layout-rename-save",
                words.rename_save().to_owned(),
                true,
                tokens,
            )
            .on_click(
                cx.listener(|shell, _event, window, cx| shell.layout_submit_rename(window, cx)),
            )
            .into_any_element(),
        );
    }
    Some(dialog(
        &shell.areas.layout.rename_dialog,
        DialogContent {
            title: words.rename().to_owned().into(),
            body: body.into_any_element(),
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        vessel_app_kit::breakpoint::phone(window),
        Shell::layout_cancel_rename,
        cx,
    ))
}
