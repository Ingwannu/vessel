//! Per-device presentation of a saved machine, separate from its authority.
//!
//! [Decision Log]
//! - Purpose: Paseo's `hosts/appearance.ts` distinguishes an explicit badge
//!   choice from the automatic local-hidden/remote-name default.
//! - Constraint: whether a machine is local may not be known at first paint;
//!   treating that interval as remote briefly paints the wrong badge.
//! - Choice: typed locality and resolution states. Explicit choices resolve
//!   even while locality is pending; only the automatic choice waits.
//! - Boundary: no credentials, transport, persistence or UI tokens here.
//!   This policy does not change the machine's book name or node identity.

mod label;
pub use label::{EmptyLabel, Label};

/// Paseo's host colors, in `HOST_COLORS` order (none then identity colors).
/// This is a choice, not a paint value; the kit owns its actual colors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Color {
    /// No identity color.
    #[default]
    None,
    /// Violet.
    Violet,
    /// Sky.
    Sky,
    /// Emerald.
    Emerald,
    /// Orange.
    Orange,
    /// Pink.
    Pink,
    /// Indigo.
    Indigo,
    /// Teal.
    Teal,
    /// Red.
    Red,
    /// Amber.
    Amber,
    /// Blue.
    Blue,
}

impl Color {
    /// Choices in the source's stable order.
    pub const ALL: [Self; 11] = [
        Self::None,
        Self::Violet,
        Self::Sky,
        Self::Emerald,
        Self::Orange,
        Self::Pink,
        Self::Indigo,
        Self::Teal,
        Self::Red,
        Self::Amber,
        Self::Blue,
    ];
}

/// A visible badge's contents; Hidden is a separate resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BadgeContent {
    /// Machine label.
    Name,
    /// Machine icon without a label.
    Icon,
}

/// The person's choice, kept separately from the automatic default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum BadgePreference {
    /// Local machines hide, remote machines show their name.
    #[default]
    Automatic,
    /// Show the machine's name.
    Name,
    /// Show its icon only.
    Icon,
    /// Hide it regardless of where the machine runs.
    Hidden,
}

/// Whether the managed local machine has been identified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locality {
    /// Local-machine lookup has not answered.
    Pending,
    /// This is the local machine.
    Local,
    /// This is another machine (or there is no managed local machine).
    Remote,
}

/// A badge's resolved state; pending is not a remote machine or a choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// Wait for the local-machine lookup before choosing the default.
    Pending,
    /// No badge should be painted.
    Hidden,
    /// Paint these contents in the chosen identity color.
    Visible(BadgeContent),
}

/// The machine's per-device appearance; it carries no connection identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Appearance {
    /// An identity color, or none.
    pub color: Color,
    /// The explicit choice or automatic default.
    pub badge: BadgePreference,
}

impl Appearance {
    /// Resolve the source's local/remote defaults without an interim flash.
    #[must_use]
    pub const fn badge(self, locality: Locality) -> Badge {
        match self.badge {
            BadgePreference::Name => Badge::Visible(BadgeContent::Name),
            BadgePreference::Icon => Badge::Visible(BadgeContent::Icon),
            BadgePreference::Hidden => Badge::Hidden,
            BadgePreference::Automatic => match locality {
                Locality::Pending => Badge::Pending,
                Locality::Local => Badge::Hidden,
                Locality::Remote => Badge::Visible(BadgeContent::Name),
            },
        }
    }
}

#[cfg(test)]
mod tests;
