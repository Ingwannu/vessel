//! Reserve the eventual session slot before create, not after its side effect.
//! A later intake may edit the source, so its old slot cannot be presumed free.
use crate::shell::Shell;
use vessel_app_view::composer::{drafts::DraftKey, trays::Trays};

impl Shell {
    pub(super) fn reserved_attachment_slots(&self) -> (usize, usize) {
        let held = self.areas.composer.starting.borrow();
        held.iter()
            .chain(self.areas.composer.workspace_starting.values())
            .fold((0_usize, 0_usize), |(files, contexts), starting| {
                (
                    files.saturating_add(usize::from(!starting.files.is_empty())),
                    contexts.saturating_add(usize::from(starting.context.is_some())),
                )
            })
    }
    /// Empty presentation remnants carry no bytes, context, or pending revision.
    pub(super) fn prune_empty_attachment_slots(&mut self) {
        let files: Vec<_> = self
            .areas
            .composer
            .trays
            .entries()
            .filter_map(|(key, tray)| tray.is_empty().then_some(key))
            .collect();
        for key in files {
            drop(self.areas.composer.trays.remove(key));
        }
        let contexts: Vec<_> = self
            .areas
            .composer
            .contexts
            .entries()
            .filter_map(|(key, tray)| (!tray.has_retained()).then_some(key))
            .collect();
        for key in contexts {
            drop(self.areas.composer.contexts.remove(key));
        }
    }
    pub(super) fn creation_has_attachment_room(&mut self, source: DraftKey) -> bool {
        self.prune_empty_attachment_slots();
        let (files, contexts) = self.reserved_attachment_slots();
        let composer = &self.areas.composer;
        (!composer
            .trays
            .get(source)
            .is_some_and(|tray| !tray.is_empty())
            || composer.trays.len().saturating_add(files) < Trays::MAX)
            && (!composer
                .contexts
                .get(source)
                .is_some_and(|tray| !tray.is_empty())
                || composer.contexts.len().saturating_add(contexts) < Trays::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{areas::composer::attach::Incoming, shell::language_tests};
    use gpui::TestAppContext;
    use vessel_app_view::creation::Ticket;
    use vessel_proto::{NodeId, SessionId, timeline::ClientMessageId};
    #[gpui::test]
    fn a_pending_creation_reserves_its_destination_without_evicting_later_source_input(
        cx: &mut TestAppContext,
    ) {
        language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, _window, cx| {
            shell.composer_add(
                DraftKey::New,
                vec![Incoming::pasted("image/png", vec![1])],
                cx,
            );
            let ticket = Ticket::new(1);
            assert!(
                shell
                    .prepare_draft_input(
                        ticket,
                        ClientMessageId::random(),
                        NodeId::random(),
                        "fixture",
                        ""
                    )
                    .unwrap()
                    .is_some()
            );
            let mut siblings = Vec::new();
            for _ in 0..Trays::MAX.saturating_sub(2) {
                let key = DraftKey::Session {
                    session: SessionId::random(),
                };
                shell.composer_add(key, vec![Incoming::pasted("image/png", vec![2])], cx);
                siblings.push(key);
            }
            let refused = DraftKey::Session {
                session: SessionId::random(),
            };
            shell.composer_add(refused, vec![Incoming::pasted("image/png", vec![3])], cx);
            assert!(!shell.composer_has_files(refused));
            shell.composer_add(
                DraftKey::New,
                vec![Incoming::pasted("image/png", vec![4])],
                cx,
            );
            let card = super::super::prepared::tests::card("fixture");
            shell.started_draft_files(ticket, &card, cx);
            assert!(shell.composer_has_files(DraftKey::New));
            assert!(
                siblings
                    .into_iter()
                    .all(|key| shell.composer_has_files(key))
            );
            assert!(shell.composer_has_files(DraftKey::Session {
                session: card.session
            }));
            assert_eq!(shell.areas.composer.trays.len(), Trays::MAX);
        });
    }
}
