//! A credential's lifetime on screen. Kept outside a text field so its
//! edit history cannot retain a revealed link after it has been hidden.

use super::{LinkShown, Problem, Ticket};
use chrono::{DateTime, Utc};
use vessel_proto::{Invitation, PairingLink, PairingOffer};

/// A node may mint an invitation before it has published any route.
/// Such a link cannot pair another device, so the UI explains setup
/// instead of presenting an unusable credential as a successful action.
///
/// # Errors
/// The offer contains neither a direct address nor a relay.
pub fn invitation_route(offer: &PairingOffer) -> Result<(), Problem> {
    if offer.direct.is_empty() && offer.relays.is_empty() {
        Err(Problem::NoRoute)
    } else {
        Ok(())
    }
}

#[derive(Debug, Default)]
enum Visibility {
    #[default]
    Hidden,
    Revealed {
        since: DateTime<Utc>,
        key: Ticket,
    },
}

/// A timer's safe guard: no credential, only a unique reveal identity and
/// its deadline. Matching the date alone fails when a clock moves back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayLimit {
    deadline: DateTime<Utc>,
    key: Ticket,
}

impl DisplayLimit {
    /// When the real elapsed timer should wake.
    #[must_use]
    pub const fn deadline(self) -> DateTime<Utc> {
        self.deadline
    }
}

/// A single-use invitation, initially masked even when just created.
#[derive(Debug)]
pub struct Pairing {
    invitation: Invitation,
    fingerprint: String,
    shown: Visibility,
}

impl Pairing {
    /// Keep a node's invitation in memory, without revealing it.
    #[must_use]
    pub fn new(invitation: Invitation, fingerprint: String) -> Self {
        Self {
            invitation,
            fingerprint,
            shown: Visibility::Hidden,
        }
    }
    /// The public key the other device must show before pairing.
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    /// When the node will stop accepting this invitation.
    #[must_use]
    pub const fn expires(&self) -> DateTime<Utc> {
        self.invitation.expires
    }
    /// Reveal an unexpired link. `key` comes from the page's non-reused
    /// ticket source, which survives machine changes and new invitations.
    pub fn show(&mut self, now: DateTime<Utc>, key: Ticket) {
        if now < self.expires() {
            self.shown = Visibility::Revealed { since: now, key };
        }
    }
    /// Hide on copy, focus loss, window hiding, navigation or request.
    pub fn hide(&mut self) {
        self.shown = Visibility::Hidden;
    }
    /// The only route to a credential's text, including clipboard writes.
    #[must_use]
    pub fn visible_link(&self, now: DateTime<Utc>) -> Option<&PairingLink> {
        let visible = match self.shown {
            Visibility::Hidden => false,
            Visibility::Revealed { since, .. } => {
                now < self.expires() && LinkShown::Since(since).visible(now)
            }
        };
        visible.then_some(&self.invitation.link)
    }
    /// A real elapsed timer masks only the reveal it was created for,
    /// even if a new invitation or a clock rollback has the same deadline.
    pub fn hide_limit(&mut self, limit: DisplayLimit) {
        if self.display_limit() == Some(limit) {
            self.hide();
        }
    }
    /// Encode only a currently revealed link. Hidden and expired links
    /// produce no matrix; the renderer never receives the source text.
    ///
    /// # Errors
    /// The revealed link cannot fit a QR code.
    pub fn matrix(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<super::qr::Matrix>, super::qr::QrProblem> {
        self.visible_link(now)
            .map(|link| super::qr::Matrix::encode(link.expose()))
            .transpose()
    }
    /// Wake at the earlier of display timeout and invitation expiry.
    #[must_use]
    pub fn display_limit(&self) -> Option<DisplayLimit> {
        match self.shown {
            Visibility::Hidden => None,
            Visibility::Revealed { since, key } => {
                LinkShown::Since(since).hides_at().map(|at| DisplayLimit {
                    deadline: at.min(self.expires()),
                    key,
                })
            }
        }
    }
    /// The display's deadline, without its identity.
    #[must_use]
    pub fn hides_at(&self) -> Option<DateTime<Utc>> {
        self.display_limit().map(DisplayLimit::deadline)
    }
}
