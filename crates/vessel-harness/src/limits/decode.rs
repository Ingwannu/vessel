//! Sensitive external schemas are owned here; callers receive sanitized facts.
use vessel_proto::limits::{LimitFailure, LimitKey, LimitWindow};

pub(super) fn unique_key(
    candidate: &str,
    previous: &[LimitWindow],
) -> Result<LimitKey, LimitFailure> {
    (1..=128)
        .map(|suffix| {
            if suffix == 1 {
                candidate.to_owned()
            } else {
                format!("{candidate}_{suffix}")
            }
        })
        .find(|key| previous.iter().all(|window| window.id.as_str() != key))
        .ok_or(LimitFailure::ChangedResponse)?
        .parse()
        .map_err(|_invalid| LimitFailure::ChangedResponse)
}
