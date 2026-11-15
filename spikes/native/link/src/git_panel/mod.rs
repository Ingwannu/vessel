//! Correlated readonly panel calls on G4's one selected admission.
//!
//! [Decision Log]
//! - Existing assets: typed workspace calls, G4 selected_connection and pure
//!   panel tickets. No alias resolver, node-id map, parser or transport is added.
//! - Choice: echo original owner/context/ticket on every result and local
//!   rejection; a bounded hub owns calls through event delivery.
//! - Effect: closed/full/admission/timeout failures settle the exact request.
//!   Commit/branch identities remain in the immutable comparison ticket.
//! - Compatibility: exact Uncommitted/Commit scopes retain their legacy calls.
//!   All/Committed use reviewed canonical values; the client refuses their staged
//!   None admission before a frame. Neither is approximated by legacy Base.
#[cfg(not(target_arch = "wasm32"))]
mod calls;

use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::workspace::git::panel::{
    ComparisonTicket, FileTicket, LocationTicket, PanelAddress, ReadFailure, StatusTicket,
};
use vessel_proto::SessionInfo;
use vessel_proto::workspace::{DiffSummary, FileDiff, GitStatus};

/// Each request retains its complete original pane and authenticated admission.
#[derive(Debug, Clone)]
pub enum Ask {
    /// Hydrate only the exact session explicitly named in the legacy layout key.
    Locate(LocationTicket),
    /// Repository facts are independent of a selected diff comparison.
    Status {
        /// Logical workspace and stable tab, never `TabId` alone.
        address: PanelAddress,
        /// Original selected admission from the canonical Machine snapshot.
        context: ConnectionContext,
        /// Full repository and lifetime request key.
        ticket: StatusTicket,
    },
    /// A summary belongs to a mode/base/whitespace or immutable commit identity.
    Summary {
        /// Logical owner.
        address: PanelAddress,
        /// Original selected admission.
        context: ConnectionContext,
        /// Full query and request identity.
        ticket: ComparisonTicket,
    },
    /// A body also retains its requested normalized relative path.
    File {
        /// Logical owner.
        address: PanelAddress,
        /// Original selected admission.
        context: ConnectionContext,
        /// Full file/query/request identity.
        ticket: FileTicket,
    },
}

/// A typed answer cannot migrate to whichever pane/session is now selected.
#[derive(Debug)]
pub struct Answer {
    /// Same admission as the request, even for local rejection.
    pub context: ConnectionContext,
    /// Same logical workspace and tab as the request.
    pub address: PanelAddress,
    /// Exact ticket plus typed result; no success-shaped empty substitute.
    pub outcome: Outcome,
}

/// Edge failures remain distinct from a genuinely empty successful result.
#[derive(Debug)]
pub enum Outcome {
    /// Original metadata is retained; view validates the exact returned session.
    Located(LocationTicket, Result<Box<SessionInfo>, ReadFailure>),
    /// Repository facts.
    Status(StatusTicket, Result<GitStatus, ReadFailure>),
    /// Complete or explicitly partial summary.
    Summary(ComparisonTicket, Result<DiffSummary, ReadFailure>),
    /// All fields of the bounded body, including continuation and old path.
    File(FileTicket, Result<FileDiff, ReadFailure>),
}
impl Ask {
    /// The actor uses only the originally selected admission, not a new lookup.
    #[must_use]
    pub fn context(&self) -> &ConnectionContext {
        match self {
            Self::Locate(ticket) => ticket.context(),
            Self::Status { context, .. }
            | Self::Summary { context, .. }
            | Self::File { context, .. } => context,
        }
    }
    /// Routing and installation both retain logical workspace ownership.
    #[must_use]
    pub fn address(&self) -> &PanelAddress {
        match self {
            Self::Locate(ticket) => ticket.address(),
            Self::Status { address, .. }
            | Self::Summary { address, .. }
            | Self::File { address, .. } => address,
        }
    }
    /// Used by both send rejection and actor preflight, before any RPC can poll.
    #[must_use]
    pub fn refused(self, why: ReadFailure) -> Answer {
        let context = self.context().clone();
        let address = self.address().clone();
        let outcome = match self {
            Self::Locate(ticket) => Outcome::Located(ticket, Err(why)),
            Self::Status { ticket, .. } => Outcome::Status(ticket, Err(why)),
            Self::Summary { ticket, .. } => Outcome::Summary(ticket, Err(why)),
            Self::File { ticket, .. } => Outcome::File(ticket, Err(why)),
        };
        Answer {
            context,
            address,
            outcome,
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn repository(&self) -> Option<vessel_app_view::workspace::git::panel::RepositoryKey> {
        match self {
            Self::Locate(_) => None,
            Self::Status { ticket, .. } => Some(ticket.repository()),
            Self::Summary { ticket, .. } => Some(ticket.repository()),
            Self::File { ticket, .. } => Some(ticket.comparison().repository()),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use calls::Hub;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
