//! Fixtures the page's renderers are held to (ADR 0111): JSON a node sends
//! and the text the Rust renderer makes of it, kept in `web/src/wire/
//! fixtures/` so the page's tests read the same files.
//!
//! A text fixture is the Rust renderer's output: a test compares it, and
//! rewrites it when `VESSEL_WRITE_TS` is set (the gate's `ts` step runs the
//! tests named `typescript…` that way), so a change of the text shows in
//! review as a change of the file.

use std::path::PathBuf;

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../web/src/wire/fixtures")
        .join(name)
}

/// The fixture `name`'s text.
pub(crate) fn read(name: &str) -> String {
    std::fs::read_to_string(path(name)).unwrap_or_default()
}

/// Whether the fixture `name` holds `generated`; rewritten instead when
/// `VESSEL_WRITE_TS` is set.
pub(crate) fn current(name: &str, generated: &str) -> bool {
    if std::env::var_os("VESSEL_WRITE_TS").is_some() {
        let path = path(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, generated).unwrap();
        return true;
    }
    read(name) == generated
}
