//! Independent workspace identity and explicit resource ownership (ADR 0082).
//! Migration number is assigned by the integrator; released SQL is append-only.

pub(super) const SQL: &str = "
CREATE TABLE workspace_registry (
    id TEXT NOT NULL PRIMARY KEY,
    project TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 0),
    descriptor TEXT NOT NULL,
    recovery TEXT
) STRICT, WITHOUT ROWID;
CREATE INDEX workspace_registry_project ON workspace_registry(project, created_at, id);
CREATE TABLE workspace_registry_resources (
    kind TEXT NOT NULL CHECK (kind IN ('session', 'terminal')),
    resource TEXT NOT NULL,
    workspace TEXT NOT NULL REFERENCES workspace_registry(id) ON DELETE CASCADE,
    stage TEXT NOT NULL DEFAULT 'ready' CHECK (stage IN ('pending','ready','failed','uncertain')),
    lease_revision INTEGER NOT NULL DEFAULT 0 CHECK (lease_revision >= 0),
    PRIMARY KEY (kind, resource)
) STRICT, WITHOUT ROWID;
CREATE INDEX workspace_registry_resource_owner ON workspace_registry_resources(workspace);
CREATE TABLE workspace_registry_meta (
    singleton INTEGER NOT NULL PRIMARY KEY CHECK (singleton = 1),
    revision INTEGER NOT NULL CHECK (revision >= 0)
) STRICT;
INSERT INTO workspace_registry_meta(singleton, revision) VALUES (1, 0);
CREATE TABLE workspace_registry_reservations (
    id TEXT NOT NULL PRIMARY KEY,
    intent TEXT NOT NULL,
    worktree TEXT,
    stage TEXT NOT NULL CHECK (stage IN ('reserved', 'creating', 'ready', 'uncertain'))
) STRICT, WITHOUT ROWID;
";
