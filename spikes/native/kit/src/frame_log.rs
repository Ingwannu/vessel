//! A debug record of the frames drawn while something moves (the
//! coordinator, 2026-11-14: proof that motion advances once per frame,
//! without the owner's GPU). With `VESSEL_APP_FRAME_LOG=<file>` set, each
//! frame a moving control draws appends one line to the file: its tag and
//! the microseconds since the first line. Without it nothing is written.
//!
//! [Decision Log]
//! - Purpose: count the frames and their intervals during a scripted drag
//!   and a Fast run, against the display's refresh interval.
//! - Choice: a line per frame from the control's paint (the frame GPUI
//!   draws and then presents), to a file the run names; the variable is
//!   read once.
//! - Alternatives: GPUI's own frame timing (not exposed by the pinned
//!   crate); a log at `tracing` level (the app's log is rotated and
//!   shared with everything else).
//! - Not a setting: a debugging switch, as `VESSEL_APP_REDUCE_MOTION`.

use std::fs::File;
use std::io::Write as _;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// The open record, when the variable names a file.
static LOG: OnceLock<Option<Mutex<(Instant, File)>>> = OnceLock::new();

/// Note a frame drawn by the control `tag`.
pub fn note(tag: &str) {
    let log = LOG.get_or_init(|| {
        let path = std::env::var_os("VESSEL_APP_FRAME_LOG")?;
        let file = File::create(path).ok()?;
        Some(Mutex::new((Instant::now(), file)))
    });
    let Some(log) = log else {
        return;
    };
    if let Ok(mut log) = log.lock() {
        let micros = log.0.elapsed().as_micros();
        let _written = writeln!(log.1, "{tag} {micros}");
    }
}
