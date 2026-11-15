//! Creation may finish after its pane closes; the captured payload never follows focus.
use super::{PrepareProblem, PreparedSubmission, Starting};
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_view::{Card, composer::drafts::DraftKey, creation::Ticket};
use vessel_proto::timeline::ClientMessageId;

/// One creation attempt and its explicitly addressed source, before any effect.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "W3 keyed creation consumer is staged with this bridge"
    )
)]
pub(crate) struct WorkspaceInput<'a> {
    pub(crate) target: DraftKey,
    pub(crate) ticket: Ticket,
    pub(crate) message: ClientMessageId,
    pub(crate) connection: vessel_app_view::connection::ConnectionContext,
    pub(crate) text: &'a str,
}
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "W3 keyed creation consumer is staged with this bridge"
    )
)]
impl Shell {
    /// Same pipeline as New, with no alias or selected-session fallback.
    pub(crate) fn prepare_workspace_draft_input(
        &mut self,
        input: WorkspaceInput<'_>,
    ) -> Result<Option<PreparedSubmission>, PrepareProblem> {
        let WorkspaceInput {
            target,
            ticket,
            message,
            connection,
            text,
        } = input;
        let node = connection.node();
        let machine = connection.machine().as_str();
        let DraftKey::Workspace { node: expected, .. } = target else {
            return Err(PrepareProblem::Context(
                vessel_app_view::composer::context::ContextProblem::Scope,
            ));
        };
        if expected != node || self.composer_field(target).is_none() {
            return Err(PrepareProblem::Context(
                vessel_app_view::composer::context::ContextProblem::Scope,
            ));
        }
        if self.composer_attachments_blocked(target)
            || self
                .areas
                .composer
                .workspace_starting
                .keys()
                .any(|(key, _)| *key == target)
        {
            return Err(PrepareProblem::Busy);
        }
        if !self.composer_has_files(target) && !self.composer_has_context(target) {
            return Ok(None);
        }
        if self
            .areas
            .composer
            .sends
            .len()
            .saturating_add(self.areas.composer.workspace_starting.len())
            .saturating_add(usize::from(self.areas.composer.starting.borrow().is_some()))
            >= vessel_app_view::composer::trays::Trays::MAX
        {
            return Err(PrepareProblem::Capacity);
        }
        if !self.creation_has_attachment_room(target) {
            return Err(PrepareProblem::Capacity);
        }
        let machine =
            vessel_app_view::machine_book::Name::parse(machine).map_err(PrepareProblem::Machine)?;
        let context = self
            .areas
            .composer
            .contexts
            .get_mut(target)
            .map(vessel_app_view::composer::context::ContextTray::prepare)
            .transpose()?
            .flatten();
        let held = Starting {
            connection,
            message,
            context,
            machine,
            expected_node: node,
            text: text.to_owned(),
            ticket: Some(ticket),
            files: self
                .areas
                .composer
                .trays
                .get(target)
                .cloned()
                .unwrap_or_default(),
        };
        drop(
            self.areas
                .composer
                .workspace_starting
                .insert((target, ticket), held),
        );
        Ok(Some(PreparedSubmission { ticket }))
    }
    /// Only the source+ticket pair can transfer its immutable snapshot.
    pub(crate) fn started_workspace_draft_files(
        &mut self,
        target: DraftKey,
        ticket: Ticket,
        card: &Card,
        cx: &mut Context<'_, Self>,
    ) {
        let key = (target, ticket);
        let Some(held) = self.areas.composer.workspace_starting.get(&key) else {
            return;
        };
        if held.machine.as_str() != card.machine
            || self.areas.composer.sends.contains_key(&card.session)
        {
            return;
        }
        let machine = held.machine.clone();
        let node = held.expected_node;
        let connection = held.connection.clone();
        let Some(held) = self.areas.composer.workspace_starting.remove(&key) else {
            return;
        };
        self.composer_started_from(target, held, card, cx);
        if let Err(problem) =
            self.read_creation_capabilities(ticket, machine, node, connection, card.session)
        {
            self.fail_creation_preparation(card.session, &problem, cx);
        }
    }
    /// A rejected sibling cannot unsuppress or release another draft's source.
    pub(crate) fn rejected_workspace_draft_files(
        &mut self,
        target: DraftKey,
        ticket: Ticket,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(held) = self
            .areas
            .composer
            .workspace_starting
            .remove(&(target, ticket))
        {
            if let Some(context) = &held.context
                && let Some(tray) = self.areas.composer.contexts.get_mut(target)
            {
                let _settled = tray.settle(
                    context,
                    vessel_app_view::composer::context::SubmissionOutcome::Failed,
                );
            }
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{areas::composer::attach::Incoming, shell::language_tests};
    use gpui::TestAppContext;
    use vessel_app_view::{
        Machine, Reach,
        composer::context::ContextText,
        creation::{DraftDestination, DraftSeed, Launch, SeedPlace},
        layout::{DraftId, LayoutKey},
        machine_book::Name,
    };
    use vessel_proto::NodeId;

    fn admit(
        shell: &mut Shell,
        node: NodeId,
        workspace: vessel_proto::WorkspaceId,
        draft: DraftId,
        window: &mut vessel_app_kit::ui::Window,
        cx: &mut Context<'_, Shell>,
    ) -> DraftKey {
        shell
            .seed_draft(
                draft,
                DraftSeed {
                    destination: DraftDestination {
                        machine: Name::parse("fixture").unwrap(),
                        node,
                    },
                    place: SeedPlace::ExistingWorkspace(workspace),
                    launch: Launch::BlankTerminal,
                    text: ContextText::parse("").unwrap(),
                    history: None,
                },
                window,
                cx,
            )
            .unwrap();
        DraftKey::Workspace {
            node,
            workspace,
            draft,
        }
    }
    #[gpui::test]
    fn equal_local_tickets_in_different_drafts_cannot_release_each_other(cx: &mut TestAppContext) {
        language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, window, cx| {
            let node = NodeId::random();
            shell.fleet.machines.push(Machine {
                name: "fixture".to_owned(),
                node: Some(node),
                reach: Reach::Online,
                sessions: Vec::new(),
                agents: Vec::new(),
                protocol: None,
                version: None,
            });
            let workspace = vessel_proto::WorkspaceId::random();
            let a = admit(shell, node, workspace, DraftId::random(), window, cx);
            let b = admit(shell, node, workspace, DraftId::random(), window, cx);
            let tokens = vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark);
            assert!(
                crate::areas::composer::workspace_card(shell, a, true, &tokens, window, cx)
                    .is_some()
            );
            let ticket = Ticket::new(1);
            for target in [a, b] {
                shell.composer_add(
                    target,
                    vec![Incoming::pasted("image/png", vec![1, 2, 3])],
                    cx,
                );
                assert!(
                    shell
                        .prepare_workspace_draft_input(crate::areas::composer::WorkspaceInput {
                            target,
                            ticket,
                            message: ClientMessageId::random(),
                            connection: super::super::tests::connection(node),
                            text: "payload"
                        })
                        .unwrap()
                        .is_some()
                );
            }
            shell.rejected_workspace_draft_files(a, Ticket::new(2), cx);
            assert_eq!(shell.areas.composer.workspace_starting.len(), 2);
            shell.rejected_workspace_draft_files(a, ticket, cx);
            assert_eq!(shell.areas.composer.workspace_starting.len(), 1);
            assert!(
                shell
                    .areas
                    .composer
                    .workspace_starting
                    .contains_key(&(b, ticket))
            );
            let card = super::super::tests::card("fixture");
            shell.started_workspace_draft_files(a, ticket, &card, cx);
            assert!(!shell.areas.composer.sends.contains_key(&card.session));
            shell.started_workspace_draft_files(b, ticket, &card, cx);
            assert!(shell.areas.composer.sends.contains_key(&card.session));
            assert!(shell.composer_has_files(a));
            assert!(!shell.composer_has_files(b));
            assert!(!shell.composer_has_files(DraftKey::New));
            assert!(
                shell
                    .workspace_draft_field(
                        &LayoutKey::Workspace { node, workspace },
                        match b {
                            DraftKey::Workspace { draft, .. } => draft,
                            _ => unreachable!(),
                        }
                    )
                    .is_some()
            );
        });
    }
    #[gpui::test]
    fn history_capacity_refuses_before_evicting_a_sibling(cx: &mut TestAppContext) {
        language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, window, cx| {
            let node = NodeId::random();
            shell.fleet.machines.push(Machine {
                name: "fixture".to_owned(),
                node: Some(node),
                reach: Reach::Online,
                sessions: Vec::new(),
                agents: Vec::new(),
                protocol: None,
                version: None,
            });
            let workspace = vessel_proto::WorkspaceId::random();
            let mut first = None;
            for index in 0..=vessel_app_view::composer::trays::Trays::MAX {
                let target = admit(shell, node, workspace, DraftId::random(), window, cx);
                let result = shell.composer_seed_history(
                    target,
                    super::super::tests::history("retained"),
                    window,
                    cx,
                );
                if index == vessel_app_view::composer::trays::Trays::MAX {
                    assert_eq!(
                        result,
                        Err(vessel_app_view::composer::context::ContextProblem::Capacity)
                    );
                    assert!(!shell.composer_has_context(target));
                } else {
                    assert_eq!(result, Ok(()));
                }
                let _first = first.get_or_insert(target);
            }
            assert!(shell.composer_has_context(first.unwrap()));
            assert_eq!(
                shell.areas.composer.contexts.len(),
                vessel_app_view::composer::trays::Trays::MAX
            );
        });
    }
}
