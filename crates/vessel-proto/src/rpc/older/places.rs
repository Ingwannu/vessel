//! Terminals in ADR 0121's workspace places (`worktree`, `folder`), left out
//! of the lists a peer below `WORKSPACE_TERMINALS_PROTOCOL` reads.
//!
//! [Decision Log]
//! - Purpose: an older peer decodes `TerminalPlace` with two kinds; a third
//!   would fail its whole frame and drop its connection.
//! - Choice: such a terminal is not in its lists, as if it were another
//!   client's private shell; the calls naming one by id answer `not_found`
//!   to that peer (the node's terminal handlers). It cannot create one: its
//!   `Call::since` gate refuses the place.
//! - Cost: a person on an older client does not see the shells a newer one
//!   opened in a worktree or an unregistered folder.

use crate::ProtocolVersion;
use crate::terminals::TerminalList;

/// `list` without the terminals an older peer cannot decode; `None` when
/// nothing is left out.
pub(super) fn listed(list: &TerminalList, protocol: ProtocolVersion) -> Option<TerminalList> {
    if list
        .terminals
        .iter()
        .all(|info| info.place.readable_by(protocol))
    {
        return None;
    }
    let mut older = list.clone();
    older
        .terminals
        .retain(|info| info.place.readable_by(protocol));
    Some(older)
}
