CREATE TABLE checkpoint_captures (
    id TEXT PRIMARY KEY NOT NULL,
    session TEXT NOT NULL REFERENCES sessions(id) ON DELETE RESTRICT,
    worktree TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK(generation > 0),
    boundary TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('candidate','ready','unavailable','expired')),
    worker TEXT NOT NULL DEFAULT 'active' CHECK(worker IN ('active','orphaned','settled')),
    image TEXT,
    evidence TEXT,
    checkpoint TEXT UNIQUE,
    reason TEXT,
    reserved_bytes INTEGER NOT NULL CHECK(reserved_bytes >= 0),
    created_at INTEGER NOT NULL,
    CHECK(state != 'ready' OR (checkpoint IS NOT NULL AND image IS NOT NULL AND evidence IS NOT NULL)),
    CHECK(state != 'unavailable' OR reason IS NOT NULL)
) STRICT;
CREATE INDEX checkpoint_session ON checkpoint_captures(session, created_at);
CREATE TABLE checkpoint_garbage (
    object TEXT PRIMARY KEY NOT NULL,
    reserved_bytes INTEGER NOT NULL CHECK(reserved_bytes >= 0),
    created_at INTEGER NOT NULL
) STRICT;
CREATE TABLE checkpoint_plans (
    id TEXT PRIMARY KEY NOT NULL,
    capture TEXT NOT NULL REFERENCES checkpoint_captures(id) ON DELETE RESTRICT,
    owner TEXT NOT NULL,
    current_image TEXT NOT NULL,
    current_evidence TEXT NOT NULL,
    desired_image TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    consumed_by TEXT
) STRICT;
CREATE TABLE checkpoint_restores (
    id TEXT PRIMARY KEY NOT NULL,
    session TEXT NOT NULL REFERENCES sessions(id) ON DELETE RESTRICT,
    plan TEXT NOT NULL REFERENCES checkpoint_plans(id) ON DELETE RESTRICT,
    owner TEXT NOT NULL,
    outcome TEXT NOT NULL,
    phase TEXT NOT NULL CHECK(phase IN ('prepared','complete','recovery')),
    before_image TEXT NOT NULL,
    event_seq INTEGER CHECK(event_seq >= 0),
    created_at INTEGER NOT NULL,
    UNIQUE(session, event_seq)
) STRICT;
CREATE TABLE checkpoint_paths (
    restore TEXT NOT NULL REFERENCES checkpoint_restores(id) ON DELETE RESTRICT,
    path TEXT NOT NULL,
    before_state TEXT NOT NULL,
    desired_state TEXT NOT NULL,
    staged TEXT,
    phase TEXT NOT NULL CHECK(phase IN ('pending','applied','restored','conflict')),
    PRIMARY KEY(restore, path)
) STRICT, WITHOUT ROWID;

CREATE TABLE checkpoint_allocations (
    object TEXT PRIMARY KEY NOT NULL,
    capture TEXT NOT NULL REFERENCES checkpoint_captures(id) ON DELETE RESTRICT,
    owner TEXT NOT NULL,
    reserved_bytes INTEGER NOT NULL CHECK(reserved_bytes > 0),
    phase TEXT NOT NULL CHECK(phase IN ('reserved','orphaned','attached','cleaned')),
    created_at INTEGER NOT NULL
) STRICT;
CREATE INDEX checkpoint_allocation_capture ON checkpoint_allocations(capture);
