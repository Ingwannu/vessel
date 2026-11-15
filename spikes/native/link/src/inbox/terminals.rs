//! Terminal attention uses existing typed calls through the same selected
//! connection. Every claim is followed by an exact revision/state recheck.
#[cfg(not(target_arch = "wasm32"))]
use super::{node::problem, transport::Client};
use vessel_app_view::inbox::admission::Admission;
#[cfg(not(target_arch = "wasm32"))]
use vessel_app_view::inbox::{Problem, terminals::Row};
use vessel_proto::TerminalId;
#[cfg(not(target_arch = "wasm32"))]
use vessel_proto::notifications::{
    ClaimNotification, ClearTerminalAttention, NotificationClaim, NotificationSource,
};
use vessel_proto::notifications::{TerminalAttention, TerminalAttentionRevision};

/// Protocol 11 documents 64 live and 32 retained terminals per node.
#[cfg(not(target_arch = "wasm32"))]
const TERMINAL_ROWS: usize = 96;

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn list(client: &Client) -> Result<Option<Vec<Row>>, Problem> {
    if vessel_proto::Method::ClaimNotification
        .since()
        .is_none_or(|since| since > client.protocol())
    {
        return Ok(None);
    }
    let list = client
        .call(vessel_proto::terminals::ListTerminals { place: None })
        .await
        .map_err(problem)?;
    if list.terminals.len() > TERMINAL_ROWS {
        return Err(Problem::InvalidReply);
    }
    let mut seen = std::collections::HashSet::new();
    list.terminals
        .into_iter()
        .map(|info| {
            if !seen.insert(info.id)
                || info
                    .attention
                    .is_some_and(|state| state.terminal != info.id)
            {
                return Err(Problem::InvalidReply);
            }
            Ok(Row {
                terminal: info.id,
                title: info.title,
                plain_shell: matches!(
                    info.kind,
                    vessel_proto::terminals::TerminalKind::Shell { .. }
                ),
                attention: info.attention,
                failed: matches!(info.status, vessel_proto::SessionStatus::Failed { .. }),
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// Exact-revision terminal operation, retained with its original admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// One local notification opportunity at the observed revision.
    Claim {
        /// Exact node terminal.
        terminal: TerminalId,
        /// Observed non-reused attention revision.
        revision: TerminalAttentionRevision,
    },
    /// Explicitly read only the revision that was shown to the person.
    Clear {
        /// Exact node terminal.
        terminal: TerminalId,
        /// Observed non-reused attention revision.
        revision: TerminalAttentionRevision,
    },
}
/// A claim decision or a confirmed current clear result.
#[derive(Debug)]
pub enum ResultState {
    /// Only Admitted authorizes best-effort local display.
    Claim(Admission),
    /// Current authoritative terminal state after exact-revision clear.
    Cleared(TerminalAttention),
}
#[cfg(not(target_arch = "wasm32"))]
impl Action {
    pub(super) async fn run(self, client: &Client) -> Result<ResultState, Problem> {
        match self {
            Self::Clear { terminal, revision } => {
                let state = client
                    .call(ClearTerminalAttention { terminal, revision })
                    .await
                    .map_err(problem)?;
                if state.terminal != terminal || state.revision < revision {
                    return Err(Problem::InvalidReply);
                }
                Ok(ResultState::Cleared(state))
            }
            Self::Claim { terminal, revision } => {
                let decision = client
                    .call(ClaimNotification {
                        source: NotificationSource::Terminal { terminal, revision },
                    })
                    .await
                    .map_err(problem)?;
                let admitted = match decision {
                    NotificationClaim::Admitted => {
                        let info = client
                            .call(vessel_proto::terminals::GetTerminal {
                                terminal,
                                changed_from: None,
                            })
                            .await
                            .map_err(problem)?;
                        info.id == terminal
                            && info.attention.is_some_and(|state| {
                                state.terminal == terminal
                                    && state.revision == revision
                                    && vessel_app_view::inbox::terminals::announces(state)
                            })
                    }
                    NotificationClaim::NotRecipient
                    | NotificationClaim::Suppressed
                    | NotificationClaim::NoAttention
                    | NotificationClaim::AlreadyClaimed
                    | NotificationClaim::NoPresentClient
                    | NotificationClaim::Unknown => false,
                };
                Ok(ResultState::Claim(if admitted {
                    Admission::Admitted
                } else {
                    Admission::Suppressed
                }))
            }
        }
    }
}
