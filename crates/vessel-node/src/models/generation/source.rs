//! Freeze the first provisional title's material; lost history is never repaired with later input.

use super::text::Material;
use vessel_proto::{AgentEvent, ContentBlock, EventSeq, SessionEvent, SessionTitle, TitleOrigin};
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
    Stream(#[from] tokio::sync::broadcast::error::RecvError),
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

/// Cached and live records are subscribed atomically, covering input racing creation.
pub(super) async fn first(handle: &SessionHandle) -> Result<Seed, SourceError> {
    let follow = handle.follow(EventSeq::ZERO)?;
    let mut cached = follow.cached.into_iter();
    let mut live = follow.live;
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
        let record = if let Some(record) = cached.next() {
            record
        } else {
            tokio::select! {
                record = live.recv() => record?,
                changed = info.changed() => {
                    changed.map_err(|_ended| SourceError::Kept)?;
                    continue;
                }
            }
        };
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
    let mut info = handle.watch_info();
    loop {
        if !handle.accepts_generated_title(expected) || info.changed().await.is_err() {
            return;
        }
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
