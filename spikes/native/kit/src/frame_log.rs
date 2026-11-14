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
//! - The draw probe ([`drawn`], 2026-11-14, the caret's repaint scope):
//!   one line per window frame, `draw <time> <took>`, the microseconds
//!   from the root's render to the end of its paint. That is the CPU the
//!   app spends per frame before the renderer rasterises, the part a GPU
//!   does not take away; the CPU a process measure gives on a software
//!   renderer is mostly rasterising (it grows with the pixels).

use std::fs::File;
use std::io::Write as _;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// The open record, when the variable names a file.
static LOG: OnceLock<Option<Mutex<(Instant, File)>>> = OnceLock::new();

/// The open record, if the variable names a file.
fn log() -> Option<&'static Mutex<(Instant, File)>> {
    LOG.get_or_init(|| {
        let path = std::env::var_os("VESSEL_APP_FRAME_LOG")?;
        let file = File::create(path).ok()?;
        Some(Mutex::new((Instant::now(), file)))
    })
    .as_ref()
}

/// Note a frame drawn by the control `tag`.
pub fn note(tag: &str) {
    if let Some(Ok(mut log)) = log().map(Mutex::lock) {
        let micros = log.0.elapsed().as_micros();
        let _written = writeln!(log.1, "{tag} {micros}");
    }
}

/// When the root's render began, for [`drawn`]; `None` without the
/// variable (no clock is read then: a browser build has none to read).
#[must_use]
pub fn started() -> Option<Instant> {
    log().map(|_| Instant::now())
}

/// The draw probe: placed as the root's last child with [`started`], it
/// notes `draw <time> <took>` when it is painted. Without the variable it
/// is an empty element that notes nothing.
#[must_use]
pub fn drawn(started: Option<Instant>) -> impl gpui::IntoElement {
    gpui::canvas(
        |_, _, _| {},
        move |_, (), _, _| {
            if let (Some(started), Some(Ok(mut log))) = (started, log().map(Mutex::lock)) {
                let micros = log.0.elapsed().as_micros();
                let took = started.elapsed().as_micros();
                let _written = writeln!(log.1, "draw {micros} {took}");
            }
        },
    )
}
