//! The three side-question calls: serialized discovery/creation/reset,
//! inherited configuration, and the sole context-refreshing prompt path.

use std::sync::Arc;

use tokio::sync::OwnedMutexGuard;
use vessel_proto::side_questions::{OpenSideQuestion, PromptSideQuestion, ResetSideQuestion};
use vessel_proto::{Delivered, SessionId, SessionInfo, SessionMode, WireError};

use super::{Connection, NodeService, wire};
use crate::principal::Principal;
use crate::side_questions::{self, SideError};

impl NodeService {
    pub(crate) async fn open_side_question(
        &self,
        connection: &Connection,
        request: OpenSideQuestion,
    ) -> Result<SessionInfo, WireError> {
        self.open_side_as(connection.principal(), request.session)
            .await
    }

    pub(super) async fn open_side_as(
        &self,
        principal: &Principal,
        parent: SessionId,
    ) -> Result<SessionInfo, WireError> {
        side_questions::person(principal).map_err(|error| error.wire())?;
        // One finite creation gate rather than an unbounded lock-per-id
        // map. Only discovery/create/reset hold it; questions never do.
        let lease = Arc::clone(&self.side_opening).lock_owned().await;
        self.side_locked(principal, parent, lease).await
    }

    async fn side_parent(&self, parent: SessionId) -> Result<SessionInfo, WireError> {
        let info = self
            .sessions
            .get(parent)
            .await
            .map_err(|error| wire(&error))?;
        if info.spec.side_question_of.is_some() {
            return Err(SideError::Nested.wire());
        }
        if matches!(info.spec.mode, SessionMode::Terminal { .. }) {
            return Err(SideError::Terminal.wire());
        }
        Ok(self.with_workspace(info))
    }

    async fn newest_side(&self, parent: SessionId) -> Result<Option<SessionInfo>, WireError> {
        Ok(self
            .sessions
            .list()
            .await
            .map_err(|error| wire(&error))?
            .into_iter()
            .filter(|info| {
                info.spec.side_question_of == Some(parent) && info.marks.archived_at.is_none()
            })
            .max_by_key(|info| (info.created_at, info.id)))
    }

    async fn side_locked(
        &self,
        principal: &Principal,
        parent: SessionId,
        lease: OwnedMutexGuard<()>,
    ) -> Result<SessionInfo, WireError> {
        let info = self.side_parent(parent).await?;
        if let Some(side) = self.newest_side(parent).await? {
            return Ok(self.with_workspace(side));
        }
        self.create_side(principal, info, lease).await
    }

    async fn create_side(
        &self,
        principal: &Principal,
        info: SessionInfo,
        lease: OwnedMutexGuard<()>,
    ) -> Result<SessionInfo, WireError> {
        let parent = info.id;
        let mut spec = info.spec.clone();
        let SessionMode::Structured { model, .. } = spec.mode else {
            return Err(SideError::Terminal.wire());
        };
        let options = self
            .sessions
            .choices(parent)
            .await
            .map_err(|error| wire(&error))?;
        spec.mode = SessionMode::Structured {
            model,
            resume: None,
            options,
        };
        spec.profile = info.profile;
        spec.title = vessel_proto::SessionTitle::from_prompt(&format!(
            "Side chat: {}",
            info.title
                .as_ref()
                .map_or("Untitled", vessel_proto::SessionTitle::as_str)
        ));
        spec.side_question_of = Some(parent);
        // Stored placed specs already contain their resolved cwd. The
        // normal creation boundary deliberately rejects place + cwd;
        // resolve the inherited place again instead of bypassing it.
        if spec.place.is_some() {
            spec.cwd = None;
        }
        // The creation task owns the lease until the side is visible even
        // if this request is cancelled while the task starts its process.
        self.create_core_as(principal, spec, None, None, Some(lease))
            .await
    }

    pub(crate) async fn reset_side_question(
        &self,
        connection: &Connection,
        request: ResetSideQuestion,
    ) -> Result<SessionInfo, WireError> {
        let principal = connection.principal();
        side_questions::person(principal).map_err(|error| error.wire())?;
        let lease = Arc::clone(&self.side_opening).lock_owned().await;
        let _parent = self.side_parent(request.session).await?;
        if self
            .newest_side(request.session)
            .await?
            .as_ref()
            .map(|side| side.id)
            != Some(request.side)
        {
            return Err(SideError::Changed.wire());
        }
        let _stopped = self
            .put_away(request.side)
            .await
            .map_err(|error| wire(&error))?;
        self.sessions
            .wait_stored(request.side)
            .await
            .map_err(|error| wire(&error))?;
        // Deletion can refuse while the last records commit. No new side
        // starts after that failure. Opening again is the recovery path.
        let _deleted = self
            .tidying
            .delete(request.side)
            .await
            .map_err(|error| wire(&error))?;
        // An older side may have been restored from the archive. Start
        // over must create fresh state, never fall back to that old side.
        let info = self.side_parent(request.session).await?;
        self.create_side(principal, info, lease).await
    }

    pub(crate) async fn prompt_side_question(
        &self,
        connection: &Connection,
        request: PromptSideQuestion,
    ) -> Result<Delivered, WireError> {
        side_questions::person(connection.principal()).map_err(|error| error.wire())?;
        if request
            .question
            .as_str()
            .trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .starts_with('/')
        {
            return Err(SideError::Slash.wire());
        }
        let side = self
            .sessions
            .get(request.session)
            .await
            .map_err(|error| wire(&error))?;
        let parent = side
            .spec
            .side_question_of
            .ok_or_else(|| SideError::NotSide.wire())?;
        if let Some(handle) = self.sessions.live(side.id) {
            let _ready = crate::sessions::again::ready(&handle)
                .await
                .map_err(|error| wire(&error))?;
        }
        let info = self.side_parent(parent).await?;
        let conversation = self
            .sessions
            .conversation(parent)
            .await
            .map_err(|error| wire(&error))?;
        let snapshot = self
            .sessions
            .side_context(parent)
            .await
            .map_err(|error| wire(&error))?;
        let prefix = side_questions::prompt(&info, &conversation, &snapshot, &request.question)
            .map_err(|error| error.wire())?;
        self.sessions
            .side_prompt(request.session, request.question, prefix)
            .await
            .map_err(|error| wire(&error))
    }
}
