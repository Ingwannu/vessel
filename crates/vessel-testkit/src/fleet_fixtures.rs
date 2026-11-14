//! Fleet source and name shapes used through the real node controls.
//! No simulated fleet state: the node still pairs, applies and reports.

/// Long but within pairing's 64-character label bound; also used by `fleet.sh`.
pub const MEMBER_NAME: &str = "Connected build server with a long name for applied-state rows";

/// Before and after have equal length. With mtime restored, only manual
/// rebundling notices this edit; a cheap stamp is intentionally unchanged.
pub const INSTRUCTIONS: [&str; 2] = ["version a\n", "version b\n"];
