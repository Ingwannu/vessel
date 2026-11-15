//! Conservative pre-mutation admission: no accepted rows or selections are evicted.
use super::{Entry, Problem};
const MAX_RETAINED_ENTRIES: usize = 10_000;
const MAX_RETAINED_BYTES: usize = 16 * 1024 * 1024;
/// A single user import action cannot keep an unbounded sequential task alive.
pub const MAX_IMPORT_ENTRIES: usize = 100;
pub(super) fn admit(existing: &[Entry], incoming: &[Entry]) -> Result<(), Problem> {
    if incoming.len() > vessel_proto::history::AgentHistoryPage::MAX_ENTRIES {
        return Err(Problem::InvalidReply);
    }
    if existing.len().saturating_add(incoming.len()) > MAX_RETAINED_ENTRIES {
        return Err(Problem::Capacity);
    }
    let bytes = existing
        .iter()
        .chain(incoming)
        .fold(0_usize, |bytes, entry| {
            bytes
                .saturating_add(std::mem::size_of::<Entry>())
                .saturating_add(entry.id.as_str().len())
                .saturating_add(entry.cwd.as_os_str().len())
                .saturating_add(entry.title.as_ref().map_or(0, |title| title.as_str().len()))
        });
    if bytes > MAX_RETAINED_BYTES {
        Err(Problem::Capacity)
    } else {
        Ok(())
    }
}
