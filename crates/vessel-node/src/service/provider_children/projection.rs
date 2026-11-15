//! Pure bounded reduction of real observations; never guesses provider support.

use std::collections::VecDeque;
use vessel_proto::subagents::{
    ChildCursor, ChildDescriptor, ChildDescriptors, ChildGap, ChildHistory, ProviderChildId,
    ProviderChildPage, ProviderChildren, ProviderObservation, ReadProviderChild,
};
use vessel_proto::{AgentEvent, EventRecord, EventSeq, SessionEvent};

pub(super) struct Projection {
    descriptors: Vec<ChildDescriptor>,
    activity: VecDeque<EventRecord>,
    request: Option<ReadProviderChild>,
    supported: bool,
    gap: Option<ChildGap>,
    first: Option<EventSeq>,
    next: EventSeq,
    end: EventSeq,
    at_end: bool,
}
impl Projection {
    pub(super) fn new(request: Option<ReadProviderChild>, start: EventSeq, end: EventSeq) -> Self {
        Self {
            descriptors: Vec::new(),
            activity: VecDeque::new(),
            request,
            supported: false,
            gap: (start != EventSeq::ZERO).then_some(ChildGap::ScanBudget),
            first: None,
            next: start,
            end,
            at_end: false,
        }
    }
    pub(super) fn observe(&mut self, record: EventRecord) -> bool {
        if self.first.is_none() {
            if record.seq != self.next {
                self.gap = Some(ChildGap::Retention);
            }
            self.first = Some(record.seq);
        }
        self.next = record.seq.next().unwrap_or(record.seq);
        let SessionEvent::Agent {
            event: AgentEvent::ProviderChild(observation),
        } = record.event
        else {
            return true;
        };
        self.supported = true;
        match observation {
            ProviderObservation::Supported => {}
            ProviderObservation::Descriptor { descriptor } => self.descriptor(descriptor),
            ProviderObservation::Gap { reason } => self.gap = Some(reason),
            ProviderObservation::Activity { child, activity } => {
                if !self.descriptors.iter().any(|d| d.id == child) {
                    self.descriptor(identity(child.clone()));
                }
                if let Some(request) = &self.request
                    && request.child == child
                    && !matches!(request.cursor, Some(ChildCursor::After(from)) if record.seq < from)
                {
                    let event = SessionEvent::Agent {
                        event: (*activity).into(),
                    };
                    self.activity.push_back(EventRecord {
                        seq: record.seq,
                        at: record.at,
                        event,
                    });
                    let limit = usize::try_from(u32::from(request.limit)).unwrap_or(usize::MAX);
                    if matches!(request.cursor, Some(ChildCursor::After(_)))
                        && self.activity.len() >= limit
                    {
                        return false;
                    }
                    while self.activity.len() > limit {
                        let _oldest = self.activity.pop_front();
                    }
                }
            }
        }
        true
    }
    fn descriptor(&mut self, descriptor: ChildDescriptor) {
        let mut proposed = self.descriptors.clone();
        if let Some(row) = proposed.iter_mut().find(|row| row.id == descriptor.id) {
            row.parent = descriptor.parent.or_else(|| row.parent.take());
            row.title = descriptor.title.or_else(|| row.title.take());
            row.description = descriptor.description.or_else(|| row.description.take());
            row.subtitle = descriptor.subtitle.or_else(|| row.subtitle.take());
            row.status = descriptor.status.or(row.status);
            row.created_at = descriptor.created_at.or(row.created_at);
            row.updated_at = descriptor.updated_at.or(row.updated_at);
        } else {
            proposed.push(descriptor);
        }
        match ChildDescriptors::try_from(proposed) {
            Ok(rows) => self.descriptors = rows.as_slice().to_vec(),
            Err(vessel_proto::subagents::InvalidChildCollection::Identity) => {
                self.gap = Some(ChildGap::Nesting);
            }
            Err(
                vessel_proto::subagents::InvalidChildCollection::Count
                | vessel_proto::subagents::InvalidChildCollection::Bytes
                | vessel_proto::subagents::InvalidChildCollection::Timeline,
            ) => self.gap = Some(ChildGap::IngestionBudget),
        }
    }
    pub(super) fn scanned(&self) -> bool {
        self.first.is_some()
    }
    pub(super) fn next(&self) -> EventSeq {
        self.next
    }
    pub(super) fn finish(&mut self, at_end: bool) {
        self.at_end = at_end;
    }
    pub(super) fn truncated(&mut self) {
        self.gap = Some(ChildGap::ScanBudget);
    }
    /// A decoded record proves a real boundary even when its bytes exceed the budget.
    pub(super) fn refused_record(&mut self, seq: EventSeq) {
        let _first = self.first.get_or_insert(seq);
        self.truncated();
    }
    fn history(&self, missing: bool) -> ChildHistory {
        if let (Some(reason), Some(from)) = (self.gap, self.first) {
            return ChildHistory::Partial { reason, from };
        }
        if !self.supported || missing {
            return ChildHistory::Missing;
        }
        ChildHistory::Complete
    }
    pub(super) fn list(
        self,
    ) -> Result<ProviderChildren, vessel_proto::subagents::InvalidChildCollection> {
        let history = self.history(false);
        Ok(ProviderChildren {
            children: self.descriptors.try_into()?,
            history,
        })
    }
    pub(super) fn page(
        self,
    ) -> Result<ProviderChildPage, vessel_proto::subagents::InvalidChildCollection> {
        let missing = self
            .request
            .as_ref()
            .is_none_or(|request| !self.descriptors.iter().any(|d| d.id == request.child));
        let history = self.history(missing);
        // Prefix metadata scanning must not move an existing continuation backward.
        let next = match self.request.as_ref().and_then(|request| request.cursor) {
            Some(ChildCursor::After(from)) => self.next.max(from),
            Some(ChildCursor::Before(_)) | None => self.next,
        };
        Ok(ProviderChildPage {
            records: self.activity.into_iter().collect::<Vec<_>>().try_into()?,
            next,
            at_end: self.at_end && self.next >= self.end,
            history,
        })
    }
}
fn identity(id: ProviderChildId) -> ChildDescriptor {
    ChildDescriptor {
        id,
        parent: None,
        title: None,
        description: None,
        subtitle: None,
        status: None,
        created_at: None,
        updated_at: None,
    }
}

#[cfg(test)]
mod tests;
