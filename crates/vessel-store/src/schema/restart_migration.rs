//! Append-only recovery projection; existing sessions start ineligible.
//! The migration number is provisional and assigned finally by integration.

/// Bounded per-session state and indexed pending/active timestamps.
pub(super) const RESTART: &str = "
CREATE TABLE restart_control (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    enabled INTEGER NOT NULL CHECK(enabled IN(0,1))
) STRICT;
INSERT INTO restart_control VALUES(1,0);
CREATE TABLE restart_state (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    state TEXT NOT NULL,
    active_at INTEGER,
    pending_at INTEGER,
    claimed INTEGER NOT NULL CHECK(claimed IN(0,1))
) STRICT, WITHOUT ROWID;
CREATE INDEX restart_pending ON restart_state(pending_at,session_id) WHERE pending_at IS NOT NULL;
CREATE INDEX restart_active ON restart_state(active_at,session_id) WHERE active_at IS NOT NULL;
";

#[cfg(test)]
mod tests;
