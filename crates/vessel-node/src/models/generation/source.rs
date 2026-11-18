//! Freeze the first provisional title's material; lost history is never repaired with later input.

use super::text::Material;
use std::sync::Arc;
use tokio::sync::{broadcast, watch};
use vessel_proto::{
    AgentEvent, ContentBlock, EventSeq, SessionEvent, SessionInfo, SessionTitle, TitleOrigin,
};
use vessel_session::SessionHandle;

pub(super) struct Seed {
    pub(super) expected: SessionTitle,
    pub(super) material: Material,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum SourceError {
    #[error("the first title source is no longer cached")]
    History(#[from] vessel_session::FollowError),
    #[error("the first title source stream was lost")]
    Stream(#[from] broadcast::error::RecvError),
    #[error("the title source was empty within its bound")]
    Empty,
    #[error("the session no longer owns its provisional title")]
    Kept,
}

/// The runtime chooses the first titleable text block too; attachments cannot seed a title.
fn capture(content: &[ContentBlock]) -> Result<Option<Seed>, SourceError> {
    for block in content {
        if let ContentBlock::Text { text } = block
            && let Some(expected) = SessionTitle::from_prompt(text)
        {
            let material = Material::capture(text).ok_or(SourceError::Empty)?;
            return Ok(Some(Seed { expected, material }));
        }
    }
    Ok(None)
}

/// One bounded cursor; cancelling a pending broadcast receive loses no record.
struct Cursor {
    cached: std::vec::IntoIter<Arc<vessel_proto::EventRecord>>,
    live: broadcast::Receiver<Arc<vessel_proto::EventRecord>>,
}
impl Cursor {
    fn new(handle: &SessionHandle) -> Result<Self, SourceError> {
        let follow = handle.follow(EventSeq::ZERO)?;
        Ok(Self {
            cached: follow.cached.into_iter(),
            live: follow.live,
        })
    }
    async fn next(
        &mut self,
        info: &mut watch::Receiver<SessionInfo>,
    ) -> Result<Option<Arc<vessel_proto::EventRecord>>, SourceError> {
        if let Some(record) = self.cached.next() {
            return Ok(Some(record));
        }
        tokio::select! {
            record = self.live.recv() => Ok(Some(record?)),
            changed = info.changed() => {
                changed.map_err(|_ended| SourceError::Kept)?;
                Ok(None)
            }
        }
    }
}

/// Cached and live records are subscribed atomically, covering input racing creation.
pub(super) async fn first(handle: &SessionHandle) -> Result<Seed, SourceError> {
    let mut cursor = Cursor::new(handle)?;
    let mut info = handle.watch_info();
    loop {
        {
            let current = info.borrow();
            if !current.status.is_live()
                || current.parked.is_some()
                || matches!(
                    current.title_origin,
                    TitleOrigin::Typed | TitleOrigin::Generated
                )
            {
                return Err(SourceError::Kept);
            }
        }
        let Some(record) = cursor.next(&mut info).await? else {
            continue;
        };
        if matches!(record.event, SessionEvent::Parked { .. }) {
            return Err(SourceError::Kept);
        }
        if let SessionEvent::Agent {
            event: AgentEvent::TurnStarted { content },
        } = &record.event
            && let Some(seed) = capture(content)?
        {
            return given(handle, seed).await;
        }
    }
}

async fn given(handle: &SessionHandle, seed: Seed) -> Result<Seed, SourceError> {
    let mut info = handle.watch_info();
    loop {
        if handle.accepts_generated_title(&seed.expected) {
            return Ok(seed);
        }
        {
            let current = info.borrow();
            if !current.status.is_live() || current.parked.is_some() || current.title.is_some() {
                return Err(SourceError::Kept);
            }
        }
        info.changed().await.map_err(|_ended| SourceError::Kept)?;
    }
}

/// Used around preparation and transport; the actor checks again when applying.
pub(super) async fn lost(handle: &SessionHandle, expected: &SessionTitle) {
    let Ok(mut cursor) = Cursor::new(handle) else {
        return;
    };
    let mut info = handle.watch_info();
    loop {
        if !handle.accepts_generated_title(expected) {
            return;
        }
        match cursor.next(&mut info).await {
            Ok(Some(record)) if cancels(&record.event, expected) => return,
            Ok(Some(_) | None) => {}
            Err(_lost) => return,
        }
    }
}

// A watch may coalesce Parked followed by Woken. Event history makes that
// cancellation permanent for this task rather than spending after a wake.
fn cancels(event: &SessionEvent, expected: &SessionTitle) -> bool {
    match event {
        SessionEvent::Parked { .. }
        | SessionEvent::Forked { .. }
        | SessionEvent::Rewound { .. } => true,
        SessionEvent::Renamed { title, origin } => {
            *origin != TitleOrigin::Given || title != expected
        }
        SessionEvent::Status { status } => !status.is_live(),
        SessionEvent::Terminal(_)
        | SessionEvent::Agent { .. }
        | SessionEvent::ProfileChanged { .. }
        | SessionEvent::ModelCall(_)
        | SessionEvent::OptionsChanged { .. }
        | SessionEvent::QueueChanged { .. }
        | SessionEvent::CommandsChanged { .. }
        | SessionEvent::UsageChanged { .. }
        | SessionEvent::Woken
        | SessionEvent::Reloaded { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_titleable_block_is_frozen_and_a_blank_prefix_never_uses_a_later_one() {
        let content = [
            ContentBlock::Text {
                text: "  \n ".into(),
            },
            ContentBlock::Text {
                text: "First source\nMore detail".into(),
            },
            ContentBlock::Text {
                text: "Later source".into(),
            },
        ];
        assert_eq!(
            capture(&content).unwrap().unwrap().expected.as_str(),
            "First source"
        );
        let over = format!(
            "{}meaningful tail",
            " ".repeat(super::super::text::MAX_SOURCE_CHARS)
        );
        let content = [
            ContentBlock::Text { text: over },
            ContentBlock::Text {
                text: "Later".into(),
            },
        ];
        assert!(matches!(capture(&content), Err(SourceError::Empty)));
    }
}
