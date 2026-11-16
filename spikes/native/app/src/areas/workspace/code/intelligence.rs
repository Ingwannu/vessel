//! Saved-source language queries belong to the file resource, so late
//! replies cannot populate another tab or pane. Unsaved content is never
//! mislabeled as a saved-language-server result.

use std::num::NonZeroU32;

use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::workspace::{FileAsk, IntelligenceAsk, IntelligenceResult};
use vessel_app_view::workspace::intelligence::{IntelligenceAction, language_for};
use vessel_proto::lsp::{AtPosition, FilePath, Offset, PageRequest, Position, Workspace};

use super::buffers::FileKey;
use crate::shell::Shell;

pub(super) enum State {
    Idle,
    Refused(Refusal),
    Loading(IntelligenceAsk),
    Ready {
        ask: IntelligenceAsk,
        result: IntelligenceResult,
    },
}

pub(super) enum Refusal {
    Unsaved,
    Workspace,
    Path,
    Language,
}

impl Shell {
    pub(super) fn language_query_for(
        &mut self,
        key: &FileKey,
        action: IntelligenceAction,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.mirror_editor_for(key, cx);
        let request = self.language_request(key, cx);
        match request {
            Ok(at) => self.send_language_query(key, at, action),
            Err(refusal) => {
                if let Some(viewer) = self.areas.workspace.code.viewer_for(key) {
                    viewer.intelligence = State::Refused(refusal);
                }
            }
        }
        cx.notify();
    }

    fn language_request(
        &mut self,
        key: &FileKey,
        cx: &Context<'_, Self>,
    ) -> Result<AtPosition, Refusal> {
        let viewer = self
            .areas
            .workspace
            .code
            .viewer_for(key)
            .ok_or(Refusal::Workspace)?;
        let language = language_for(key.path.as_str()).ok_or(Refusal::Language)?;
        let root = viewer.workspace_dir.clone().ok_or(Refusal::Workspace)?;
        let file = FilePath::try_from(std::path::PathBuf::from(key.path.as_str()))
            .map_err(|_invalid| Refusal::Path)?;
        let (line, column) = if let Some(editing) = &viewer.edit {
            if editing.stale || editing.model.modified() || editing.model.writing().is_some() {
                return Err(Refusal::Unsaved);
            }
            editing
                .editor
                .as_ref()
                .map_or((1, 1), |editor| editor.cursor(cx))
        } else {
            (
                viewer
                    .chosen
                    .and_then(|(line, _)| u32::try_from(line).ok())
                    .unwrap_or(0)
                    .saturating_add(1),
                1,
            )
        };
        let position = Position {
            line: NonZeroU32::new(line).ok_or(Refusal::Path)?,
            column: NonZeroU32::new(column).ok_or(Refusal::Path)?,
        };
        // All query kinds name the same saved document; diagnostics use
        // the position only as part of the retained navigation context.

        Ok(AtPosition {
            workspace: Workspace { root, language },
            file,
            position,
            page: PageRequest::default(),
        })
    }

    fn send_language_query(&mut self, key: &FileKey, at: AtPosition, action: IntelligenceAction) {
        let Some(ticket) = self.next_file_ticket() else {
            return;
        };
        let ask = IntelligenceAsk {
            ticket,
            root: key.root,
            path: key.path.clone(),
            at,
            action,
        };
        if let Some(viewer) = self.areas.workspace.code.viewer_for(key) {
            viewer.intelligence = State::Loading(ask.clone());
        }
        self.send_file_for(key, FileAsk::Intelligence(ask));
    }

    pub(super) fn language_answered(
        &mut self,
        key: &FileKey,
        ask: IntelligenceAsk,
        result: IntelligenceResult,
    ) {
        let Some(viewer) = self.areas.workspace.code.viewer_for(key) else {
            return;
        };
        if matches!(&viewer.intelligence, State::Loading(pending) if pending.ticket == ask.ticket) {
            viewer.intelligence = State::Ready { ask, result };
        }
    }

    pub(super) fn language_next_for(
        &mut self,
        key: &FileKey,
        offset: Offset,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(viewer) = self.areas.workspace.code.viewer_for(key) else {
            return;
        };
        let State::Ready { ask, .. } = &viewer.intelligence else {
            return;
        };
        let mut at = ask.at.clone();
        at.page.offset = offset;
        let action = ask.action;
        self.send_language_query(key, at, action);
        cx.notify();
    }

    pub(super) fn language_location_for(
        &mut self,
        key: &FileKey,
        location: &vessel_proto::lsp::Location,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.workspace.code.file_key().as_ref() != Some(key) {
            return;
        }
        let path = location
            .file
            .as_path()
            .to_str()
            .and_then(|text| text.parse().ok());
        if let (Some(path), Ok(location)) = (
            path,
            vessel_app_view::workspace::file_location::FileLocation::try_from(location.range),
        ) {
            self.open_file_location(path, Some(location), cx);
        } else if let Some(viewer) = self.areas.workspace.code.files.viewer.as_mut() {
            viewer.intelligence = State::Refused(Refusal::Path);
            cx.notify();
        }
    }
}
