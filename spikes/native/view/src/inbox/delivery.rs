//! Desktop delivery policy stays pure and shares the persisted four-mode alert
//! preference. Operating-system capability and consent remain platform-owned.
use super::Key;
use crate::prefs::Alerts;
use vessel_proto::{NodeId, SessionId};

/// One independently configurable delivery channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// This channel is selected.
    Enabled,
    /// This channel is not selected.
    Disabled,
}

/// Separate channels let sound-only preferences remain useful without sending
/// a visual notification, and visual-only mode never requests a bell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivery {
    /// Operating system notification center.
    pub desktop: Channel,
    /// System alert sound.
    pub sound: Channel,
}
impl Delivery {
    /// Decode the existing persisted preference without a second settings file.
    #[must_use]
    pub const fn of(alerts: Alerts) -> Self {
        use Channel::{Disabled, Enabled};
        match alerts {
            Alerts::Off => Self {
                desktop: Disabled,
                sound: Disabled,
            },
            Alerts::System => Self {
                desktop: Enabled,
                sound: Disabled,
            },
            Alerts::Sound => Self {
                desktop: Disabled,
                sound: Enabled,
            },
            Alerts::SystemAndSound => Self {
                desktop: Enabled,
                sound: Enabled,
            },
        }
    }
    /// Toggle one channel while preserving the other channel's choice.
    #[must_use]
    pub const fn toggled(self, channel: Toggle) -> Alerts {
        use Channel::{Disabled, Enabled};
        let (desktop, sound) = match channel {
            Toggle::Desktop => (self.desktop.toggled(), self.sound),
            Toggle::Sound => (self.desktop, self.sound.toggled()),
        };
        match (desktop, sound) {
            (Disabled, Disabled) => Alerts::Off,
            (Enabled, Disabled) => Alerts::System,
            (Disabled, Enabled) => Alerts::Sound,
            (Enabled, Enabled) => Alerts::SystemAndSound,
        }
    }
}
impl Channel {
    const fn toggled(self) -> Self {
        match self {
            Self::Enabled => Self::Disabled,
            Self::Disabled => Self::Enabled,
        }
    }
}

/// Which delivery switch the person changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    /// Visual operating system notifications.
    Desktop,
    /// The system alert bell.
    Sound,
}

/// The actively viewed source is suppressed, while another window/route or a
/// backgrounded application can still receive attention from that session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// No actively viewed source.
    Away,
    /// The person is already looking at this source's conversation.
    Viewing {
        /// Authenticated reporting identity.
        node: NodeId,
        /// Conversation currently visible.
        session: SessionId,
    },
}
impl Visibility {
    /// Suppression is scoped to both node and session, never a name alone.
    #[must_use]
    pub fn permits(self, key: Key) -> bool {
        match self {
            Self::Away => true,
            Self::Viewing { node, session } => node != key.node || session != key.session,
        }
    }
}
