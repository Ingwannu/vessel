//! The context ring beside the agent's control (agent-orchestrator's
//! ring; the page's `data/context-meter.ts`): filled by how much of the
//! context the agent says is used, coloured by how close it is to full.
//! The figures are the agent's own; Vessel neither counts tokens nor
//! guesses a window the agent did not report.

use vessel_proto::ContextUsage;

/// How full the context is, by agent-orchestrator's thresholds (70% and
/// 90%).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fullness {
    /// Under 70%.
    Room,
    /// From 70%: amber.
    Low,
    /// From 90%: red.
    Full,
}

/// The ring for one report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meter {
    /// Tokens used.
    pub used: u64,
    /// The window's size.
    pub size: u64,
    /// Used over size, in whole percent, 0 to 100.
    pub percent: u8,
    /// What the ring draws, in thousandths of a turn: never under 80, so a
    /// nearly empty context still shows a sliver (the reference's 8%).
    pub arc: u16,
    /// Its colour.
    pub fullness: Fullness,
}

/// The ring for `usage`; `None` before the agent reported one, or when
/// it reported no window.
#[must_use]
pub fn meter_of(usage: Option<&ContextUsage>) -> Option<Meter> {
    let usage = usage?;
    let size = usage.size.get();
    if size == 0 {
        return None;
    }
    let used = usage.used.get().min(size);
    // Whole numbers throughout (u128, so `used * 1000` cannot overflow):
    // thousandths, then percent rounded half up as the page's
    // `Math.round` does.
    let ratio = |scale: u128| {
        u128::from(used)
            .saturating_mul(scale)
            .checked_div(u128::from(size))
            .unwrap_or(0)
    };
    let thousandths = ratio(1000);
    let percent = ratio(200).saturating_add(1).checked_div(2).unwrap_or(0);
    let fullness = match thousandths {
        900.. => Fullness::Full,
        700..900 => Fullness::Low,
        _ => Fullness::Room,
    };
    Some(Meter {
        used,
        size,
        percent: u8::try_from(percent.min(100)).unwrap_or(100),
        arc: u16::try_from(thousandths.clamp(80, 1000)).unwrap_or(1000),
        fullness,
    })
}
