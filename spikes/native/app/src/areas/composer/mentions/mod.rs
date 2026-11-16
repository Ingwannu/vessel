//! File suggestions share the kit list and keep the text field's focus.
//! Captured draft/node/root/token identities guard debounce and late replies.
mod draw;
use crate::shell::Shell;
pub(super) use draw::menu;
use std::{cell::RefCell, time::Duration};
use vessel_app_kit::field::composer_hooks::{ComposerKey, KeyAnswer};
use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{TextField, autocomplete::AutocompleteState};
use vessel_app_link::{
    Command,
    mentions::{Answer, RootAnswer, RootAsk},
};
use vessel_app_view::composer::{
    context::ContextScope,
    drafts::DraftKey,
    mentions::{self, Menu, Scope},
    slash::Step,
};
use vessel_app_view::machine_book::Name;
use vessel_proto::workspace::WorkspaceRoot;

#[derive(Debug, Default)]
pub(super) struct FileMenu {
    model: Menu,
    list: AutocompleteState,
    drawn: std::rc::Rc<
        std::cell::Cell<Option<vessel_app_kit::ui::Bounds<vessel_app_kit::ui::Pixels>>>,
    >,
    reveal: RefCell<vessel_app_view::composer::reveal::Reveal>,
    last: RefCell<Option<draw::Drawn>>,
}

#[derive(Debug, Clone, Default)]
pub(super) enum Root {
    #[default]
    Missing,
    Reading(RootAsk),
    Exhausted,
    Ready(RootAsk, Result<Option<WorkspaceRoot>, mentions::Failure>),
}
impl Shell {
    pub(super) fn composer_field(&self, target: DraftKey) -> Option<&TextField> {
        match target {
            DraftKey::New => Some(&self.areas.draft.composer),
            DraftKey::Workspace {
                node,
                workspace,
                draft,
            } => self.workspace_draft_field(
                &vessel_app_view::layout::LayoutKey::Workspace { node, workspace },
                draft,
            ),
            DraftKey::Session { session } => self
                .areas
                .session
                .open
                .as_ref()
                .filter(|open| open.session == session)
                .map(|_| &self.areas.session.composer),
        }
    }
    pub(super) fn file_menu(&self, target: DraftKey) -> std::rc::Rc<RefCell<FileMenu>> {
        match target {
            DraftKey::New => std::rc::Rc::clone(&self.areas.composer.draft_files),
            DraftKey::Session { .. } => std::rc::Rc::clone(&self.areas.composer.session_files),
            DraftKey::Workspace { .. } => std::rc::Rc::clone(
                self.areas
                    .composer
                    .workspace_files
                    .borrow_mut()
                    .tray(target),
            ),
        }
    }
    pub(super) fn composer_context_scope(&self, target: DraftKey) -> Option<ContextScope> {
        match target {
            DraftKey::Workspace {
                node,
                workspace,
                draft,
            } => self.workspace_draft_context_scope(
                &vessel_app_view::layout::LayoutKey::Workspace { node, workspace },
                draft,
            ),
            DraftKey::New => {
                let machine = Name::parse(self.areas.draft.machine.as_deref()?).ok()?;
                let project = self.chosen_place()?.project;
                Some(ContextScope {
                    machine,
                    root: WorkspaceRoot::Project { project },
                })
            }
            DraftKey::Session { session } => {
                let open = self
                    .areas
                    .session
                    .open
                    .as_ref()
                    .filter(|open| open.session == session)?;
                let machine = Name::parse(&open.machine).ok()?;
                let connection = self.attachment_context(&machine)?;
                let root = self
                    .code_workspace_scope(&machine, session)
                    .or_else(|| match &*self.areas.composer.mention_root.borrow() {
                        Root::Ready(ask, Ok(Some(root)))
                            if ask.machine == machine
                                && ask.session == session
                                && ask.connection == connection =>
                        {
                            Some(*root)
                        }
                        Root::Missing | Root::Exhausted | Root::Reading(_) | Root::Ready(_, _) => {
                            None
                        }
                    })?;
                Some(ContextScope { machine, root })
            }
        }
    }
    fn resolve_mention_root(&self, target: DraftKey, token: &mentions::Mention) {
        let DraftKey::Session { session } = target else {
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
        let Ok(machine) = Name::parse(&open.machine) else {
            return;
        };
        let Some(connection) = self.attachment_context(&machine) else {
            return;
        };
        let mut state = self.areas.composer.mention_root.borrow_mut();
        let unchanged = match &*state {
            Root::Reading(old) => {
                old.machine == machine && old.session == session && old.connection == connection
            }
            Root::Ready(old, result) => {
                old.machine == machine
                    && old.session == session
                    && old.connection == connection
                    && (result.is_ok()
                        || self.areas.composer.mention_root_token.borrow().as_ref() == Some(token))
            }
            Root::Missing => false,
            Root::Exhausted => return,
        };
        if unchanged {
            return;
        }
        let Some(ticket) = self.areas.composer.mention_root_clock.get().checked_next() else {
            *state = Root::Exhausted;
            return;
        };
        self.areas.composer.mention_root_clock.set(ticket);
        let ask = RootAsk {
            connection,
            machine,
            session,
            ticket,
        };
        *self.areas.composer.mention_root_token.borrow_mut() = Some(token.clone());
        *state = Root::Reading(ask.clone());
        drop(state);
        self.command(Command::Files(
            vessel_app_link::attachments::Files::Suggest(vessel_app_link::mentions::Ask::Root(ask)),
        ));
    }
    pub(super) fn active_file_token(
        &self,
        target: DraftKey,
        cx: &vessel_app_kit::ui::App,
    ) -> Option<mentions::Mention> {
        let field = self.composer_field(target)?;
        let token = mentions::active(&field.text(cx), field.caret(cx))
            .ok()
            .flatten()?;
        if self.areas.composer.mention_dismissed.borrow().as_ref() == Some(&(target, token.clone()))
        {
            None
        } else {
            Some(token)
        }
    }
    /// Rendering observes both edits and caret-only movement without polling.
    pub(super) fn observe_file_menu(&self, target: DraftKey, cx: &mut Context<'_, Self>) {
        let Some(field) = self.composer_field(target) else {
            return;
        };
        let text = field.text(cx);
        let caret = field.caret(cx);
        let token = mentions::active(&text, caret).ok().flatten();
        {
            let mut dismissed = self.areas.composer.mention_dismissed.borrow_mut();
            if dismissed.as_ref().is_some_and(|(draft, previous)| {
                *draft == target && token.as_ref() != Some(previous)
            }) {
                *dismissed = None;
            }
        }
        if self.active_file_token(target, cx).is_none() && token.is_some() {
            let _observed = self.file_menu(target).borrow_mut().model.observe(
                None,
                &text,
                caret,
                Duration::from_secs_f32(self.areas.composer.now()),
            );
            return;
        }
        if let Some(token) = token {
            self.resolve_mention_root(target, &token);
        }
        let scope = self.composer_context_scope(target).and_then(|scope| {
            self.attachment_context(&scope.machine)
                .map(|connection| Scope {
                    connection,
                    draft: target,
                    machine: scope.machine,
                    root: scope.root,
                })
        });
        let now = Duration::from_secs_f32(self.areas.composer.now());
        let owner = self.file_menu(target);
        let mut menu = owner.borrow_mut();
        menu.model
            .inherit_ticket_clock(self.areas.composer.mention_query_clock.get());
        let request = menu.model.observe(scope, &text, caret, now);
        self.areas
            .composer
            .mention_query_clock
            .set(menu.model.ticket_clock());
        drop(menu);
        let Ok(Some(request)) = request else {
            return;
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(mentions::DEBOUNCE).await;
            let _closed = this.update(cx, |shell, cx| {
                // Re-observe before dispatch: the caret may have moved without
                // an edit while the timer was waiting.
                if shell.composer_field(target).is_none() {
                    return;
                }
                shell.observe_file_menu(target, cx);
                let ready = shell
                    .file_menu(target)
                    .borrow_mut()
                    .model
                    .begin(request.ticket);
                if let Some(ready) = ready {
                    shell.command(Command::Files(
                        vessel_app_link::attachments::Files::Suggest(
                            vessel_app_link::mentions::Ask::Suggestions(ready),
                        ),
                    ));
                    cx.notify();
                }
            });
        })
        .detach();
    }
    pub(crate) fn composer_mention_root(&mut self, answer: RootAnswer, cx: &mut Context<'_, Self>) {
        let matches = matches!(&*self.areas.composer.mention_root.borrow(), Root::Reading(ask) if ask == &answer.ask);
        if matches {
            *self.areas.composer.mention_root.borrow_mut() = Root::Ready(answer.ask, answer.result);
            cx.notify();
        }
    }
    pub(crate) fn composer_mention_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        if self.composer_field(answer.request.scope.draft).is_none()
            || self
                .attachment_context(&answer.request.scope.machine)
                .as_ref()
                != Some(&answer.request.scope.connection)
        {
            return;
        }
        let now = Duration::from_secs_f32(self.areas.composer.now());
        let changed = self
            .file_menu(answer.request.scope.draft)
            .borrow_mut()
            .model
            .arrived(answer.request.ticket, answer.result, now);
        if changed {
            cx.notify();
        }
    }
    /// Global shortcut routing reads admission without changing selection/input.
    pub(crate) fn composer_file_menu_owns_key(
        &self,
        target: DraftKey,
        key: ComposerKey,
        cx: &vessel_app_kit::ui::App,
    ) -> bool {
        if self.composer_destination() != Some(target) {
            return false;
        }
        let owner = self.file_menu(target);
        let state = owner.borrow();
        let token = self.active_file_token(target, cx);
        let current = token.as_ref().is_some_and(|token| {
            state.model.request().is_some_and(|request| {
                request.mention == *token
                    && self.attachment_context(&request.scope.machine).as_ref()
                        == Some(&request.scope.connection)
                    && self.composer_context_scope(target).is_some_and(|scope| {
                        request.scope.draft == target
                            && request.scope.machine == scope.machine
                            && request.scope.root == scope.root
                    })
            })
        });
        if key != ComposerKey::Escape {
            return current && !state.model.rows().is_empty();
        }
        let root_popup = token.is_some()
            && match &*self.areas.composer.mention_root.borrow() {
                Root::Reading(ask) | Root::Ready(ask, Err(_)) => {
                    self.areas.session.open.as_ref().is_some_and(|open| {
                        target
                            == (DraftKey::Session {
                                session: open.session,
                            })
                            && open.session == ask.session
                            && open.machine == ask.machine.as_str()
                            && self.attachment_context(&ask.machine).as_ref()
                                == Some(&ask.connection)
                    })
                }
                Root::Exhausted => true,
                Root::Missing | Root::Ready(_, Ok(_)) => false,
            };
        let invalid = match state.model.phase() {
            mentions::Phase::Invalid(problem) if self.composer_context_scope(target).is_some() => {
                self.composer_field(target).is_some_and(|field| {
                    mentions::active(&field.text(cx), field.caret(cx))
                        .is_err_and(|current| &current == problem)
                })
            }
            mentions::Phase::Hidden
            | mentions::Phase::Debouncing { .. }
            | mentions::Phase::Loading { .. }
            | mentions::Phase::Rows { .. }
            | mentions::Phase::Empty(_)
            | mentions::Phase::Failed { .. }
            | mentions::Phase::Invalid(_) => false,
        };
        current || root_popup || invalid
    }

    /// The composer shell delegates before slash handling so an active @ token wins.
    pub(crate) fn composer_file_key(
        &mut self,
        target: DraftKey,
        key: ComposerKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> KeyAnswer {
        self.observe_file_menu(target, cx);
        if !self.composer_file_menu_owns_key(target, key, cx) {
            return KeyAnswer::Pass;
        }
        if key == ComposerKey::Escape {
            *self.areas.composer.mention_dismissed.borrow_mut() = self
                .active_file_token(target, cx)
                .map(|token| (target, token));
            self.file_menu(target).borrow_mut().model.dismiss();
            cx.notify();
            return KeyAnswer::Taken;
        }
        if self.file_menu(target).borrow().model.rows().is_empty() {
            return KeyAnswer::Pass;
        }
        match key {
            ComposerKey::Up => {
                let _chosen = self.file_menu(target).borrow_mut().model.step(Step::Up);
            }
            ComposerKey::Down => {
                let _chosen = self.file_menu(target).borrow_mut().model.step(Step::Down);
            }
            ComposerKey::Enter | ComposerKey::Tab => {
                let selected = self.file_menu(target).borrow().model.selected();
                if let Some(index) = selected {
                    self.choose_mention(target, index, window, cx);
                }
            }
            ComposerKey::Escape => {}
        }
        cx.notify();
        KeyAnswer::Taken
    }
    fn choose_mention(
        &mut self,
        target: DraftKey,
        index: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.composer_destination() != Some(target) {
            return;
        }
        let choice = {
            let owner = self.file_menu(target);
            let state = owner.borrow();
            state
                .model
                .request()
                .zip(state.model.row(index))
                .map(|(request, row)| (request.clone(), row.path.clone()))
        };
        let Some((request, path)) = choice else {
            return;
        };
        self.commit_mention(target, &request, &path, window, cx);
    }
    fn commit_mention(
        &mut self,
        target: DraftKey,
        request: &mentions::Request,
        path: &vessel_proto::workspace::RelPath,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.composer_destination() != Some(target)
            || self.attachment_context(&request.scope.machine).as_ref()
                != Some(&request.scope.connection)
        {
            return;
        }
        if self.composer_context_scope(target).is_none_or(|scope| {
            scope.machine != request.scope.machine || scope.root != request.scope.root
        }) {
            return;
        }
        let Some(field) = self.composer_field(target) else {
            return;
        };
        // The exact token still has to be under the caret at commit time.
        let text = field.text(cx);
        if mentions::active(&text, field.caret(cx))
            .ok()
            .flatten()
            .as_ref()
            != Some(&request.mention)
        {
            return;
        }
        field.replace_range(request.mention.range(), &mentions::quoted(path), window, cx);
        field.focus(window, cx);
        let current = field.text(cx);
        if matches!(target, DraftKey::Workspace { .. }) {
            self.composer_keep_workspace_text(target, &current, cx);
        }
        self.file_menu(target).borrow_mut().model.dismiss();
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
