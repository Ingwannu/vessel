//! Admission retirement fences callbacks before a replacement fleet snapshot.
//! Retained forms remain visible but disabled; drafts survive re-confirmation.
use crate::shell::Shell;
use vessel_app_link::inbox::Target;
use vessel_app_view::connection::ConnectionTag;

impl Shell {
    pub(crate) fn inbox_retired(&mut self, tag: &ConnectionTag) -> bool {
        let area = &mut self.areas.inbox;
        let mut changed = false;
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
