//! Public synthetic JSON inputs shared by real-node and disposable fleet tests.
//! They contain no account credential or app preference record. The fake ACP
//! reports these actual option ids; they are not a vendor mapping policy.

/// Whole ordered-list CAS body for an initially empty fixture store.
pub const REPLACE: &str = include_str!("../fixtures/launch-profiles-replace.json");
/// Selected host-wide probe body, with no prompt or persistent session.
pub const OPTIONS: &str = include_str!("../fixtures/launch-profiles-options.json");

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::launch_profiles::{
        GetLaunchProfileOptions, LaunchPayload, ReplaceLaunchProfiles,
    };
    #[test]
    fn fixture_files_decode_as_real_bounded_requests() {
        assert!(REPLACE.len() < LaunchPayload::MAX_BYTES);
        assert!(OPTIONS.len() < LaunchPayload::MAX_BYTES);
        let replace: ReplaceLaunchProfiles = serde_json::from_str(REPLACE).unwrap();
        let options: GetLaunchProfileOptions = serde_json::from_str(OPTIONS).unwrap();
        assert_eq!(replace.profiles.as_slice().len(), 1);
        assert_eq!(
            replace.profiles.as_slice()[0].launch().harness(),
            options.selection.harness()
        );
    }
}
