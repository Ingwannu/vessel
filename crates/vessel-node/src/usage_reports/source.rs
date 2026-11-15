//! One shared typed registration supplies both node identities and old-peer native pins.
pub(crate) use vessel_proto::usage_reports::SavedUsageSource as SavedSource;
pub(crate) const fn source(harness: &vessel_proto::HarnessRef) -> Option<SavedSource> {
    SavedSource::for_harness(harness)
}
