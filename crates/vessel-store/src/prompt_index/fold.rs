//! Fold only user-message metadata and rewinds; retained text stays in the log.

use vessel_proto::prompt_index::{
    INDEX_ENTRIES, ListSessionPrompts, PreviewBuilder, PromptIndexEpoch, PromptIndexPage,
    PromptLocation, SCAN_RECORDS,
};
use vessel_proto::{AgentEvent, ContentBlock, EventRecord, EventSeq, SessionEvent};

use crate::StoreError;

struct Chunked {
    first: EventSeq,
    timestamp: chrono::DateTime<chrono::Utc>,
    preview: PreviewBuilder,
}

#[derive(Debug, Clone, Copy)]
enum PromptSource {
    Submitted,
    History,
}

#[derive(Default)]
pub(super) struct Index {
    entries: Vec<PromptLocation>,
    chunked: Option<Chunked>,
    rewound: Option<EventSeq>,
    scanned: usize,
}

impl Index {
    pub(super) fn take(&mut self, record: &EventRecord) -> Result<(), StoreError> {
        // Refusing before growth bounds both decode work and retained metadata.
        self.scanned = self.scanned.saturating_add(1);
        if self.scanned > SCAN_RECORDS {
            return Err(StoreError::PromptIndexRecords);
        }
        match &record.event {
            SessionEvent::Agent { event } => self.agent(event, record),
            SessionEvent::Rewound { after, .. } => {
                self.flush()?;
                self.entries.retain(|entry| entry.seq <= *after);
                self.rewound = Some(record.seq);
                Ok(())
            }
            SessionEvent::Status { .. } | SessionEvent::Terminal(_) => self.flush(),
            SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. } => Ok(()),
        }
    }

    fn agent(&mut self, event: &AgentEvent, record: &EventRecord) -> Result<(), StoreError> {
        match event {
            AgentEvent::TurnStarted { content } | AgentEvent::Steered { content } => {
                self.flush()?;
                let mut preview = PreviewBuilder::default();
                for block in content {
                    push(&mut preview, block, PromptSource::Submitted);
                }
                self.chunked = Some(Chunked {
                    first: record.seq,
                    timestamp: record.at,
                    preview,
                });
                Ok(())
            }
            AgentEvent::UserMessageChunk { content } => {
                let chunked = self.chunked.get_or_insert_with(|| Chunked {
                    first: record.seq,
                    timestamp: record.at,
                    preview: PreviewBuilder::default(),
                });
                push(&mut chunked.preview, content, PromptSource::History);
                Ok(())
            }
            AgentEvent::HarnessLog { .. } => Ok(()),
            AgentEvent::Ready { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. }
            | AgentEvent::PermissionRequested(_)
            | AgentEvent::PermissionResolved { .. }
            | AgentEvent::TurnEnded { .. }
            | AgentEvent::TurnFailed { .. }
            | AgentEvent::QuestionAsked(_)
            | AgentEvent::QuestionResolved { .. } => self.flush(),
        }
    }

    fn add(&mut self, entry: PromptLocation) -> Result<(), StoreError> {
        if self.entries.len() >= INDEX_ENTRIES {
            return Err(StoreError::PromptIndexPrompts);
        }
        self.entries.push(entry);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), StoreError> {
        if let Some(chunked) = self.chunked.take() {
            self.add(PromptLocation {
                seq: chunked.first,
                timestamp: chunked.timestamp,
                preview: chunked.preview.finish(),
            })?;
        }
        Ok(())
    }

    pub(super) fn page(
        mut self,
        first: EventSeq,
        request: &ListSessionPrompts,
    ) -> Result<PromptIndexPage, StoreError> {
        self.flush()?;
        let epoch = PromptIndexEpoch::new(first, self.rewound)?;
        if request.epoch.is_some_and(|expected| expected != epoch) {
            return Err(StoreError::PromptIndexChanged);
        }
        let mut remaining = self
            .entries
            .into_iter()
            .filter(|entry| request.after.is_none_or(|after| entry.seq > after));
        let entries: Vec<_> = remaining.by_ref().take(request.limit.count()).collect();
        let next = remaining
            .next()
            .and_then(|_| entries.last().map(|entry| entry.seq));
        PromptIndexPage::new(request.session, epoch, entries, next).map_err(StoreError::from)
    }
}

fn push(preview: &mut PreviewBuilder, block: &ContentBlock, source: PromptSource) {
    match block {
        ContentBlock::Text { text } => preview.push(text),
        ContentBlock::ResourceLink { name, uri } => {
            // Match the existing timeline text without allocating a whole URI
            // or body before the bounded preview builder truncates it.
            preview.push("[");
            preview.push(name);
            preview.push("](");
            preview.push(uri);
            preview.push(")");
        }
        ContentBlock::Attachment { attachment } => match source {
            // Submitted files have their own chips, outside the prompt text.
            PromptSource::Submitted => {}
            PromptSource::History => {
                preview.push("[");
                preview.push(attachment.name.as_str());
                preview.push("]");
            }
        },
        ContentBlock::Opaque { .. } => {}
    }
}

#[cfg(test)]
mod tests;
