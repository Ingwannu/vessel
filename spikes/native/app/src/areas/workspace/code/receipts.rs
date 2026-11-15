//! Canonical file identity and original tickets fence every queued receipt.
use super::buffers::FileKey;
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{FileAnswer, FileAsk, scoped::Request};
use vessel_app_view::workspace::{file_scope::FileScope, problem::Problem};

impl Shell {
    pub(super) fn send_file_scope(&mut self, scope: FileScope, ask: FileAsk) {
        match Request::file(scope, ask) {
            Ok(request) => self.command(vessel_app_link::Command::FileOperation(request)),
            Err(problem) => {
                self.areas.workspace.code.files.actions.refused = Some(problem.to_string())
            }
        }
    }
    pub(super) fn send_file_for(&mut self, key: &FileKey, ask: FileAsk) {
        if let Some(scope) = self
            .areas
            .workspace
            .code
            .viewer_for(key)
            .map(|viewer| viewer.scope.clone())
        {
            self.send_file_scope(scope, ask);
        }
    }
    pub(super) fn file_scoped_answer(
        &mut self,
        scope: &FileScope,
        answer: FileAnswer,
        live: bool,
        dispatched: bool,
        cx: &mut Context<'_, Self>,
    ) {
        let shown = live && self.areas.workspace.code.scope().as_ref() == Some(scope);
        match answer {
            FileAnswer::Inspected {
                ask,
                ticket,
                result,
            } => {
                if shown {
                    self.directory_inspected(scope, &ask, ticket, result);
                }
            }
            FileAnswer::Observed {
                ask,
                ticket,
                result,
            } => self.file_observed(scope, &ask, ticket, result, live, cx),
            FileAnswer::Read {
                ask,
                ticket,
                result,
            } => {
                let key = FileKey::new(scope, ask.path.clone());
                let current = live
                    && self
                        .areas
                        .workspace
                        .code
                        .viewer_for(&key)
                        .is_some_and(|viewer| viewer.scope == *scope);
                if current {
                    self.file_read_answered_for(&key, &ask, ticket, result, cx);
                } else if let Some(viewer) = self.areas.workspace.code.viewer_for(&key) {
                    let _settled = viewer.reading.take_if(|pending| *pending == ticket);
                }
            }
            FileAnswer::EditorRead {
                ask,
                ticket,
                result,
            } => {
                let key = FileKey::new(scope, ask.path.clone());
                let current = live
                    && self
                        .areas
                        .workspace
                        .code
                        .viewer_for(&key)
                        .is_some_and(|viewer| viewer.scope == *scope);
                if current {
                    self.editor_read_answered_for(&key, ticket, &ask, result, cx);
                } else if let Some(editing) = self.editing_for(&key) {
                    if editing
                        .reading
                        .take_if(|(pending, _)| *pending == ticket)
                        .is_some()
                    {
                        editing.model.retire_observations();
                    }
                }
            }
            FileAnswer::Saved {
                path,
                ticket,
                saved,
                ..
            } => {
                let key = FileKey::new(scope, path);
                let current = live
                    && self
                        .areas
                        .workspace
                        .code
                        .viewer_for(&key)
                        .is_some_and(|viewer| viewer.scope == *scope);
                self.editor_saved_for(&key, scope, ticket, current, saved, cx);
            }
            FileAnswer::Intelligence { ask, result } => {
                let key = FileKey::new(scope, ask.path.clone());
                let current = live
                    && self
                        .areas
                        .workspace
                        .code
                        .viewer_for(&key)
                        .is_some_and(|viewer| viewer.scope == *scope);
                if current {
                    self.language_answered(&key, ask, result);
                } else if let Some(viewer) = self.areas.workspace.code.viewer_for(&key) {
                    if matches!(&viewer.intelligence, super::intelligence::State::Loading(pending) if pending.ticket == ask.ticket)
                    {
                        viewer.intelligence = super::intelligence::State::Idle;
                    }
                }
            }
            FileAnswer::TrackedFound {
                ask,
                ticket,
                result,
            } => {
                let result = if live {
                    result
                } else {
                    Err(Problem::Gone("the original connection retired".into()))
                };
                self.file_search_answered(scope, &ask, ticket, result, cx);
            }
            FileAnswer::PathChanged {
                change,
                ticket,
                result,
            } => self.file_mutation_answered_for(scope, &change, ticket, result, dispatched, cx),
            FileAnswer::Listed {
                ask,
                ticket,
                result,
            } => {
                if self.file_listing_search_pending(scope, ticket) {
                    let result = if live {
                        result
                    } else {
                        Err(Problem::Gone("the original connection retired".into()))
                    };
                    self.file_listing_search_answered(scope, &ask, ticket, result, cx);
                } else if shown {
                    self.folder_read(scope.connection().machine().as_str(), &ask, ticket, result);
                }
            }
            FileAnswer::Found { ask, result } => {
                if shown {
                    self.files_found(scope.connection().machine().as_str(), &ask, result, cx);
                }
            }
            FileAnswer::Grepped { ask, result } => {
                if shown {
                    self.files_grepped(scope.connection().machine().as_str(), &ask, result);
                }
            }
        }
    }
}
