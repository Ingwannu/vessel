//! One set of caps for provider ingestion, retained state and projection (ADR 0090).

/// Descriptors retained for one managed parent; saturation is explicitly reported.
pub const MAX_CHILDREN: usize = 128;
/// Serialized descriptors retained/projected for one managed parent.
pub const MAX_DESCRIPTOR_BYTES: usize = 2 * 1024 * 1024;
/// Distinct message/tool ids retained by all child translators of one parent.
pub const MAX_TRACKED_IDS: usize = 2048;
/// Aggregate UTF-8 bytes of those retained ids; no eviction invents a new replay.
pub const MAX_TRACKED_ID_BYTES: usize = 1024 * 1024;
/// Provider nesting before the projection explicitly reports truncation.
pub const MAX_CHILD_DEPTH: usize = 8;
/// Parent records examined by a single child-list/timeline projection.
pub const MAX_SCAN_RECORDS: usize = 4096;
/// Serialized parent record bytes examined by a single projection.
pub const MAX_SCAN_BYTES: usize = 8 * 1024 * 1024;
/// A child page never means an unbounded `limit=0` read.
pub const MAX_TIMELINE_ITEMS: u32 = 200;
/// Bound for a projection including its durable read, excluding queue time.
pub const READ_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
