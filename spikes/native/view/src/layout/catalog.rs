//! Launcher ordering is shared by compact, main and supporting menus.
use super::LayoutMode;
/// Closed built-in catalog; panel owners supply working launch handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launch {
    /// New agent composer.
    Agent,
    /// New standalone shell.
    Terminal,
    /// Explorer changes.
    Changes,
    /// Working diff.
    Diff,
    /// Explorer files.
    Files,
    /// Desktop browser.
    Browser,
    /// Existing workspace PR.
    PullRequest,
}
/// The two Paseo launcher orders intentionally differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchPurpose {
    /// Blank main pane.
    Primary,
    /// Supporting-view menu.
    Supporting,
}
/// Capability outcome distinguishes hidden features from disabled launchers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Working entry can launch.
    Ready,
    /// Present but temporarily unavailable.
    Disabled,
    /// No functioning capability; entry is omitted.
    Hidden,
}
const PRIMARY: [Launch; 7] = [
    Launch::Agent,
    Launch::Terminal,
    Launch::Changes,
    Launch::Diff,
    Launch::Files,
    Launch::Browser,
    Launch::PullRequest,
];
const SUPPORTING: [Launch; 7] = [
    Launch::Changes,
    Launch::Terminal,
    Launch::Diff,
    Launch::Files,
    Launch::Agent,
    Launch::Browser,
    Launch::PullRequest,
];
/// Feature owners decide availability; compact never offers redundant Diff.
#[must_use]
pub fn catalog(
    purpose: LaunchPurpose,
    mode: LayoutMode,
    available: impl Fn(Launch) -> Availability,
) -> Vec<(Launch, Availability)> {
    let order = match purpose {
        LaunchPurpose::Primary => PRIMARY,
        LaunchPurpose::Supporting => SUPPORTING,
    };
    order
        .into_iter()
        .filter_map(|launch| {
            if launch == Launch::Diff && mode == LayoutMode::Compact {
                return None;
            }
            let availability = available(launch);
            (availability != Availability::Hidden).then_some((launch, availability))
        })
        .collect()
}
