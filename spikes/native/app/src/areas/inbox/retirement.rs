//! Admission retirement fences callbacks before a replacement fleet snapshot.
//! Retained forms remain visible but disabled; drafts survive re-confirmation.
use crate::shell::Shell;
use vessel_app_link::inbox::Target;
use vessel_app_view::connection::ConnectionTag;

impl Shell {
    pub(crate) fn inbox_retired(&mut self, tag: &ConnectionTag) -> bool {
        let area = &mut self.areas.inbox;
        let mut changed = false;
        if area
            .read_source
            .as_ref()
            .is_some_and(|source| source.context().tag() == *tag)
        {
            area.read_source = None;
            area.read_open = None;
            area.read_last = None;
            let _retired = area.reading.observe(None);
            changed = true;
        }
        for target in &mut area.targets {
            if let Target::Online(source) = target
                && source.context().tag() == *tag
            {
                area.store.disconnect(source.node());
                if area.detail_source.as_ref() == Some(source) {
                    area.detail.retire();
                    area.detail_source = None;
                }
                area.read_last = None;
                area.terminals.queued.retain(|(known, _)| known != source);
                area.terminals
                    .pending
                    .retain(|(known, _, _)| known != source);
                area.terminals.granted.retain(|(known, _)| known != source);
                area.granted.retain(|pending| pending.source != *source);
                *target = Target::Offline(source.name().clone());
                changed = true;
            }
        }
        area.admission
            .retain(|source| source.context().tag() != *tag);
        changed
    }
}
