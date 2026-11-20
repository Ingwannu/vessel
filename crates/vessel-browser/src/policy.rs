//! Whether a session is offered the browser tools, and whether a call is
//! `Disabled` or `NoEngine`. The node passes [`Presence::Absent`]. `Real`
//! exists so a test can show what an injected engine would be told; this
//! slice has no real engine.

use vessel_proto::browser::BrowserFailure;
use vessel_proto::settings::BrowserTools;

/// Whether this process can drive a real browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// A real engine is in the process. This slice never passes this.
    Real,
    /// No engine. Every call that is not `Disabled` is `NoEngine`.
    Absent,
}

/// What the session was told when it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// The setting was not on. A later `on` does not offer the tools,
    /// and a call stays `Disabled`.
    ToolsOff,
    /// The setting was on. A later `off` still refuses the call.
    ToolsOn,
}

impl Start {
    /// The start a session gets from the setting at that moment.
    #[must_use]
    pub const fn at(setting: BrowserTools) -> Self {
        match setting {
            BrowserTools::On => Self::ToolsOn,
            BrowserTools::Off | BrowserTools::Unknown => Self::ToolsOff,
        }
    }
}

/// Whether the caller holds `operate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operate {
    /// The grant includes `operate`.
    Held,
    /// It does not. The names stay hidden. The scope guard answers
    /// `Forbidden` before the browser runs.
    Missing,
}

/// Whether the twenty-two names are in the tool list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    /// Include them.
    Shown,
    /// Leave them out.
    Hidden,
}

/// The call's answer before any engine runs. Off wins, including a
/// session that started off. Absence is `NoEngine`, not a page.
#[must_use]
pub const fn gate(
    start: Start,
    setting: BrowserTools,
    presence: Presence,
) -> Result<(), BrowserFailure> {
    let off = match start {
        Start::ToolsOff => true,
        Start::ToolsOn => !matches!(setting, BrowserTools::On),
    };
    if off {
        return Err(BrowserFailure::Disabled);
    }
    match presence {
        Presence::Absent => Err(BrowserFailure::NoEngine),
        Presence::Real => Ok(()),
    }
}

/// The names, only when the session started on, the setting is still on,
/// a real engine is present, and the caller holds `operate`.
#[must_use]
pub const fn offer(
    start: Start,
    setting: BrowserTools,
    presence: Presence,
    operate: Operate,
) -> Offer {
    let allowed = match (start, setting) {
        (Start::ToolsOff, _) => false,
        (Start::ToolsOn, BrowserTools::On) => true,
        (Start::ToolsOn, BrowserTools::Off | BrowserTools::Unknown) => false,
    };
    if allowed && matches!(presence, Presence::Real) && matches!(operate, Operate::Held) {
        Offer::Shown
    } else {
        Offer::Hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_wins_and_absence_is_not_a_page() {
        assert_eq!(
            gate(Start::ToolsOn, BrowserTools::On, Presence::Absent),
            Err(BrowserFailure::NoEngine)
        );
        assert_eq!(
            gate(Start::ToolsOn, BrowserTools::Off, Presence::Absent),
            Err(BrowserFailure::Disabled)
        );
        assert_eq!(
            gate(Start::ToolsOff, BrowserTools::On, Presence::Real),
            Err(BrowserFailure::Disabled),
            "started off, then the setting was turned on"
        );
        assert_eq!(
            gate(Start::ToolsOn, BrowserTools::Off, Presence::Real),
            Err(BrowserFailure::Disabled),
            "started on, with an engine injected, then the setting was turned off"
        );
        assert_eq!(
            gate(Start::ToolsOn, BrowserTools::On, Presence::Real),
            Ok(())
        );
    }

    #[test]
    fn names_need_the_setting_a_real_engine_and_operate() {
        let on = (Start::ToolsOn, BrowserTools::On);
        assert_eq!(
            offer(on.0, on.1, Presence::Real, Operate::Held),
            Offer::Shown
        );
        assert_eq!(
            offer(on.0, on.1, Presence::Absent, Operate::Held),
            Offer::Hidden
        );
        assert_eq!(
            offer(on.0, on.1, Presence::Real, Operate::Missing),
            Offer::Hidden
        );
        assert_eq!(
            offer(Start::ToolsOff, BrowserTools::On, Presence::Real, Operate::Held),
            Offer::Hidden
        );
        assert_eq!(
            offer(Start::ToolsOn, BrowserTools::Off, Presence::Real, Operate::Held),
            Offer::Hidden
        );
    }
}
