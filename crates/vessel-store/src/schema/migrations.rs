//! Every migration's SQL, oldest first, and the list that orders them.
//!
//! These are separate from `schema`'s runner so neither file outgrows the
//! file budget (docs/QUALITY.md) as migrations are appended; a released
//! entry is never edited, only followed by a new one.

/// Version 1: sessions and their event log.
///
/// - Times are nanoseconds since the Unix epoch (`i64`), so a record's
///   `at` round-trips exactly.
/// - `spec`, `status` and `event` hold `vessel-proto` types as JSON, decoded
///   only by this crate (ADR 0004).
/// - `live`, `first_seq`, `retained_bytes`, `ended_at` and
///   `harness_session` are kept up to date as records are appended, so
///   recovery, retention and listing never scan the log.
const V1: &str = "
CREATE TABLE sessions (
    id              TEXT    NOT NULL PRIMARY KEY,
    spec            TEXT    NOT NULL,
    status          TEXT    NOT NULL,
    live            INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    ended_at        INTEGER,
    next_seq        INTEGER NOT NULL,
    first_seq       INTEGER NOT NULL,
    retained_bytes  INTEGER NOT NULL,
    harness_session TEXT
) STRICT;

CREATE INDEX sessions_live ON sessions (live) WHERE live = 1;
CREATE INDEX sessions_ended ON sessions (ended_at) WHERE live = 0;

CREATE TABLE events (
    session_id TEXT    NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    seq        INTEGER NOT NULL,
    at         INTEGER NOT NULL,
    event      TEXT    NOT NULL,
    PRIMARY KEY (session_id, seq)
) STRICT, WITHOUT ROWID;
";

/// Version 2 (milestone 3): paired devices (ADR 0010; see `devices`).
///
/// - `secret_hash` is SHA-256 of the device's credential; the secret itself
///   is never stored.
/// - `scopes` is a JSON array of scope names.
/// - `revoked_at` is set instead of deleting the row, so a revoked device
///   is told `Revoked` and not `UnknownDevice`.
const V2: &str = "
CREATE TABLE devices (
    id          TEXT    NOT NULL PRIMARY KEY,
    label       TEXT    NOT NULL,
    secret_hash BLOB    NOT NULL,
    scopes      TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    last_seen   INTEGER,
    revoked_at  INTEGER
) STRICT;
";

/// Version 3 (milestone 4, protocol 5): which profile a session runs with
/// now and who created it (ADRs 0011, 0013).
///
/// - `profile` is the current profile's name, `NULL` for the harness's
///   default; it starts as the spec's and follows `ProfileChanged` records.
/// - `created_by` is a `Creator` as JSON; `NULL` (every session stored
///   before this version) reads as the owner, the only creator there was.
const V3: &str = "
ALTER TABLE sessions ADD COLUMN profile TEXT;
ALTER TABLE sessions ADD COLUMN created_by TEXT;
";

/// Version 4 (milestone 4, protocol 6): where a session came from when
/// another node of the fleet had a hand in it.
///
/// - `via` is the home a creating call came through, a `NodeId` as text.
/// - `origin` is a `SessionOrigin` as JSON (`remote` or `imported`).
/// - Both `NULL` for a session of this node's own, and for every session
///   stored before this version, which read as `None`.
const V4: &str = "
ALTER TABLE sessions ADD COLUMN via TEXT;
ALTER TABLE sessions ADD COLUMN origin TEXT;
";

/// Version 5 (milestone 5, protocol 7): the model route a session was
/// launched with.
///
/// - `model` is a `SessionModel` as JSON (`{ route, model }`); `NULL` for a
///   session on its harness's own provider, and for every session stored
///   before this version, which read as `None`.
const V5: &str = "
ALTER TABLE sessions ADD COLUMN model TEXT;
";

/// Version 6 (milestone 10, protocol 9): what a session is called and
/// what it runs with (ADR 0022).
///
/// - `title` is the session's current title, a `SessionTitle` as its bare
///   text: the spec's when it asked for one, then each `renamed` record's.
///   It is the title a stored session reads back with; `NULL` is none (and
///   every session stored before this version, which had none).
/// - `choices` is a `Vec<OptionChoice>` as JSON: the agent's options of
///   the session's last `options_changed` record, as choices (each
///   select's value, each toggle's state), which an export carries on.
///   `NULL` before the first report.
const V6: &str = "
ALTER TABLE sessions ADD COLUMN title TEXT;
ALTER TABLE sessions ADD COLUMN choices TEXT;
";

/// Version 7 (milestone 11, protocol 10, track A): the conversation and
/// the marks a person puts on a session (ADRs 0024 to 0026).
///
/// - `abilities` (`AgentAbilities`), `usage` (`ContextUsage`) and `queue`
///   (`PromptQueue`), each JSON: projections of the session's last
///   `ready`, `usage_changed` and `queue_changed` records, so an ended
///   session answers `session.conversation` and starts again with its
///   queue without its log being read. `NULL` before the first.
/// - `attention_seq`: the `seq` of the last record asking for attention
///   (a turn's end or failure, a permission or a question asked, the
///   session's end), a projection too; `read_seq` the `through` of the
///   last `read` (only ever raised); `marked_unread` set by `unread`. A
///   session is unread when `marked_unread`, or when `attention_seq` is at
///   or after `read_seq` (none: never read).
/// - `pinned`, `archived_at`: marks. Every session that had ended before
///   this version is archived as of its end, so the board, which from
///   this version keeps ended sessions out of the archive until they are
///   archived, looks as it did.
/// - `attachments`: the files sent with prompts (ADR 0025), each row the
///   size and SHA-256 recorded when its upload finished; the bytes are
///   under the data directory's `attachments/<session>/<id>/`. Deleted
///   with their session.
const V7: &str = "
ALTER TABLE sessions ADD COLUMN abilities TEXT;
ALTER TABLE sessions ADD COLUMN usage TEXT;
ALTER TABLE sessions ADD COLUMN queue TEXT;
ALTER TABLE sessions ADD COLUMN attention_seq INTEGER;
ALTER TABLE sessions ADD COLUMN read_seq INTEGER;
ALTER TABLE sessions ADD COLUMN marked_unread INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN archived_at INTEGER;
UPDATE sessions SET archived_at = ended_at WHERE live = 0 AND ended_at IS NOT NULL;

CREATE TABLE attachments (
    id         TEXT    NOT NULL PRIMARY KEY,
    session_id TEXT    NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    kind       TEXT    NOT NULL,
    mime       TEXT    NOT NULL,
    size       INTEGER NOT NULL,
    sha256     BLOB    NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE INDEX attachments_session ON attachments (session_id);
";

/// Version 8 (milestone 11, protocol 10, track A): the order retention
/// prunes ended sessions in (ADR 0026).
///
/// - `sessions_retention` is that order: pinned ones kept longest, then
///   those not archived, then archived ones, each by the newest end. The
///   pruning query walks it backwards instead of sorting every ended
///   session each time one ends. It replaces `sessions_ended` (schema 1),
///   which served the old order (by end alone) and nothing else.
const V8: &str = "
DROP INDEX sessions_ended;
CREATE INDEX sessions_retention
    ON sessions (pinned, (archived_at IS NULL), ended_at, created_at)
    WHERE live = 0 AND ended_at IS NOT NULL;
";

/// Version 9 (milestone 11, protocol 10, track B): projects, the worktrees
/// the node made, and the command sets a person approved (ADR 0027; see
/// `workspace`). Track B's branch numbered it 7; track A's two versions
/// came first when the tracks were merged, and nothing of protocol 10 had
/// been released, so no database ever had it under the other number.
///
/// - `info` is the `ProjectInfo` or `WorktreeInfo` as JSON, written whole
///   on every change; `created_at` orders them.
/// - A worktree names its project, and a project with worktrees is never
///   deleted (the node refuses), so the reference holds.
/// - An approval is a project and a digest; deleted with its project.
const V9: &str = "
CREATE TABLE projects (
    id         TEXT    NOT NULL PRIMARY KEY,
    created_at INTEGER NOT NULL,
    info       TEXT    NOT NULL
) STRICT;

CREATE TABLE worktrees (
    id         TEXT    NOT NULL PRIMARY KEY,
    project    TEXT    NOT NULL REFERENCES projects (id),
    created_at INTEGER NOT NULL,
    info       TEXT    NOT NULL
) STRICT;

CREATE TABLE approvals (
    project     TEXT    NOT NULL REFERENCES projects (id),
    digest      TEXT    NOT NULL,
    approved_at INTEGER NOT NULL,
    PRIMARY KEY (project, digest)
) STRICT, WITHOUT ROWID;
";

/// Version 10 (milestone 11, protocol 11, track B2): pull request
/// feedback (ADR 0029; see `forge`).
///
/// - `forge_feedback_on`: the sessions a person turned feedback on for;
///   none is off, the default.
/// - `forge_feedback_sent`: the signature of each event sent to a
///   session, so a restart sends nothing twice; at most `MAX_SENT` per
///   session, the oldest dropped first (`sent_at`).
/// - Both are deleted with their session. Nothing of a pull request
///   itself is kept: the node finds it again by its branch.
const V10: &str = "
CREATE TABLE forge_feedback_on (
    session_id TEXT NOT NULL PRIMARY KEY REFERENCES sessions (id) ON DELETE CASCADE
) STRICT;

CREATE TABLE forge_feedback_sent (
    session_id TEXT    NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    signature  TEXT    NOT NULL,
    sent_at    INTEGER NOT NULL,
    PRIMARY KEY (session_id, signature)
) STRICT, WITHOUT ROWID;
";

/// Version 11 (milestone 11, protocol 11, the join of wave 2's first
/// half): an approval keeps the set it approved (`commands`, the
/// `CommandSet` as JSON), so a set that changed can be shown beside the
/// one last approved (`vessel_proto::workspace::approval`). Approvals
/// from before have none (`NULL`). Track C was given 11 and needed none,
/// so this one took it.
const V11: &str = "
ALTER TABLE approvals ADD COLUMN commands TEXT;
";

/// Version 12 (milestone 11, protocol 11, track D): schedules, their runs,
/// saved prompts and hooks (ADRs 0033 and 0034; see `automation`).
///
/// - `info`, `run` and `delivery` hold `vessel-proto` types as JSON,
///   written whole on every change (schema 9's way).
/// - A run and a delivery go with their schedule or hook; each is trimmed
///   to the newest when one is added (50 runs, 100 deliveries).
/// - A saved prompt's `scope` is its project's id, or `''` for the
///   machine's own; a name is unique within its scope.
/// - `hook_keys` are the delivery keys a hook saw in the last day, so a
///   request sent again after a restart still runs once. A hook's secret
///   is not in the database (ADR 0034).
const V12: &str = "
CREATE TABLE schedules (
    id         TEXT    NOT NULL PRIMARY KEY,
    created_at INTEGER NOT NULL,
    info       TEXT    NOT NULL
) STRICT;

CREATE TABLE schedule_runs (
    schedule TEXT    NOT NULL REFERENCES schedules (id) ON DELETE CASCADE,
    number   INTEGER NOT NULL,
    run      TEXT    NOT NULL,
    PRIMARY KEY (schedule, number)
) STRICT, WITHOUT ROWID;

CREATE TABLE saved_prompts (
    id         TEXT    NOT NULL PRIMARY KEY,
    scope      TEXT    NOT NULL,
    name       TEXT    NOT NULL,
    created_at INTEGER NOT NULL,
    info       TEXT    NOT NULL,
    UNIQUE (scope, name)
) STRICT;

CREATE TABLE hooks (
    id         TEXT    NOT NULL PRIMARY KEY,
    created_at INTEGER NOT NULL,
    info       TEXT    NOT NULL
) STRICT;

CREATE TABLE hook_deliveries (
    hook     TEXT    NOT NULL REFERENCES hooks (id) ON DELETE CASCADE,
    seq      INTEGER NOT NULL,
    delivery TEXT    NOT NULL,
    PRIMARY KEY (hook, seq)
) STRICT, WITHOUT ROWID;

CREATE TABLE hook_keys (
    hook    TEXT    NOT NULL REFERENCES hooks (id) ON DELETE CASCADE,
    key     TEXT    NOT NULL,
    seen_at INTEGER NOT NULL,
    PRIMARY KEY (hook, key)
) STRICT, WITHOUT ROWID;
";

/// The inbox (schema 16): durable attention projected in the same
/// transaction as its source event.
/// A source survives event-log pruning; deleting a session deletes its inbox.
pub(super) const INBOX: &str = "
CREATE TABLE inbox_meta (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    pending_bytes INTEGER NOT NULL DEFAULT 0,
    tool_clock INTEGER NOT NULL DEFAULT 0
) STRICT;
INSERT INTO inbox_meta(singleton) VALUES(1);
CREATE TABLE inbox (
    cursor INTEGER PRIMARY KEY AUTOINCREMENT,
    session TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    at INTEGER NOT NULL,
    request TEXT,
    waiting INTEGER NOT NULL,
    item TEXT NOT NULL,
    seen INTEGER NOT NULL DEFAULT 0,
    UNIQUE(session, seq)
) STRICT;
CREATE INDEX inbox_session ON inbox(session, request);
CREATE INDEX inbox_outcomes ON inbox(waiting, cursor);
CREATE INDEX inbox_unread ON inbox(seen);
CREATE TRIGGER inbox_payload_added AFTER INSERT ON inbox
WHEN json_extract(new.item,'$.kind') IN ('permission','question') BEGIN
    UPDATE inbox_meta SET pending_bytes=pending_bytes+length(CAST(new.item AS BLOB));
END;
CREATE TRIGGER inbox_payload_removed AFTER DELETE ON inbox
WHEN json_extract(old.item,'$.kind') IN ('permission','question') BEGIN
    UPDATE inbox_meta SET pending_bytes=pending_bytes-length(CAST(old.item AS BLOB));
END;
CREATE TRIGGER inbox_payload_changed AFTER UPDATE OF item ON inbox BEGIN
    UPDATE inbox_meta SET pending_bytes=pending_bytes
      -CASE WHEN json_extract(old.item,'$.kind') IN ('permission','question') THEN length(CAST(old.item AS BLOB)) ELSE 0 END
      +CASE WHEN json_extract(new.item,'$.kind') IN ('permission','question') THEN length(CAST(new.item AS BLOB)) ELSE 0 END;
END;

CREATE TABLE push_subscriptions (
    device TEXT NOT NULL PRIMARY KEY REFERENCES devices(id) ON DELETE CASCADE,
    subscription TEXT NOT NULL,
    expires_at INTEGER NOT NULL
) STRICT;
CREATE TABLE inbox_tools (
    session TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    tool TEXT NOT NULL,
    detail TEXT NOT NULL,
    PRIMARY KEY(session, id)
) STRICT, WITHOUT ROWID;
";

/// Version 13 (milestone 11, protocol 12, track A2): the index of what was
/// said in every session (ADR 0037; see `search`).
///
/// - `search_messages`: one row a message piece (a prompt, or a run of the
///   agent's reply chunks, at most 8 KiB of text a piece). `id` is the
///   piece's time in nanoseconds, moved forward past one already taken, so
///   walking it by `id` is newest first. `seq` is the record where the
///   piece's run starts; `role` is `prompt` or `reply`; `indexed` says
///   whether `search_index` has it yet. No foreign key: a removed
///   session's rows are hidden at once (a search joins `sessions` and its
///   `first_seq`) and deleted later, a bounded step at a time, so removing
///   a session never waits on the index.
/// - `search_index`: FTS5 over `text`, external content (the text is kept
///   in `search_messages`), trigram tokens without case. Fed and pruned
///   only by the writer's idle steps, never inside an append.
/// - `search_gone`: what the idle steps still have to delete: a session's
///   rows through `through` (the session gone: every row). Filled by the
///   two triggers on `sessions`, so every way a session or its records are
///   removed (deleting it, retention) is covered.
/// - `sessions.search_seq`: the next record the index has not read;
///   `sessions.search_open`: where the run it has read but not yet written
///   starts (a reply still arriving), or `NULL`. Every session stored
///   before this version starts at 0 and is caught up by the idle steps
///   (`sessions_search_behind` finds them).
const V13: &str = "
ALTER TABLE sessions ADD COLUMN search_seq INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN search_open INTEGER;
CREATE INDEX sessions_search_behind ON sessions (created_at) WHERE search_seq < next_seq;

CREATE TABLE search_messages (
    id         INTEGER PRIMARY KEY,
    session_id TEXT    NOT NULL,
    seq        INTEGER NOT NULL,
    role       TEXT    NOT NULL CHECK (role IN ('prompt', 'reply')),
    at         INTEGER NOT NULL,
    text       TEXT    NOT NULL,
    indexed    INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE INDEX search_messages_session ON search_messages (session_id, id);
CREATE INDEX search_messages_pending ON search_messages (id) WHERE indexed = 0;

CREATE VIRTUAL TABLE search_index USING fts5(
    text,
    content = 'search_messages',
    content_rowid = 'id',
    tokenize = 'trigram'
);

CREATE TABLE search_gone (
    session_id TEXT    NOT NULL PRIMARY KEY,
    through    INTEGER NOT NULL
) STRICT, WITHOUT ROWID;

CREATE TRIGGER sessions_search_gone AFTER DELETE ON sessions BEGIN
    INSERT INTO search_gone (session_id, through) VALUES (old.id, 9223372036854775807)
    ON CONFLICT (session_id) DO UPDATE SET through = excluded.through;
END;

CREATE TRIGGER sessions_search_pruned AFTER UPDATE OF first_seq ON sessions
WHEN new.first_seq > old.first_seq BEGIN
    INSERT INTO search_gone (session_id, through) VALUES (new.id, new.first_seq - 1)
    ON CONFLICT (session_id) DO UPDATE SET through = max(through, excluded.through);
END;
";

/// Side context (schema 14): the latest bounded plan survives log
/// retention and is independent of the partial transcript tail.
const SIDE_CONTEXT: &str = "ALTER TABLE sessions ADD COLUMN latest_plan TEXT;";

/// The usage ledger (schema 15). Reported usage is independent of the
/// event byte cap, but deleted with its session. Baselines belong to
/// native sessions, not agent processes.
const USAGE_LEDGER: &str = "
ALTER TABLE sessions ADD COLUMN usage_cost_baseline TEXT;
CREATE TABLE usage_meta (
    singleton INTEGER NOT NULL PRIMARY KEY CHECK (singleton = 1),
    started_at INTEGER NOT NULL,
    sample_count INTEGER NOT NULL DEFAULT 0,
    pruned_through INTEGER
) STRICT;
INSERT INTO usage_meta(singleton, started_at) VALUES (1, unixepoch() * 1000000000);
CREATE TABLE usage_samples (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    at INTEGER NOT NULL,
    harness TEXT NOT NULL,
    profile TEXT NOT NULL,
    sample TEXT NOT NULL,
    PRIMARY KEY(session_id, seq)
) STRICT, WITHOUT ROWID;
CREATE INDEX usage_time ON usage_samples(at);
CREATE TRIGGER usage_count_insert AFTER INSERT ON usage_samples BEGIN
    UPDATE usage_meta SET sample_count = sample_count + 1;
END;
CREATE TRIGGER usage_count_delete AFTER DELETE ON usage_samples BEGIN
    UPDATE usage_meta SET sample_count = sample_count - 1;
END;
";

/// The labels' schema (lifecycle, ADR 0063): 16 on its track, 17 on the
/// node line after the inbox's 16. The one assignment integration changes
/// when another migration lands first; the table's length checks it.
pub(super) const LABELS_SCHEMA: usize = 17;

/// Current labels are separate from the original creation specification.
const LABELS: &str = "ALTER TABLE sessions ADD COLUMN labels TEXT;";

/// The provisional title-origin schema follows the newest head migration.
/// Integration changes this one assignment if another migration lands first.
const TITLE_ORIGIN_SCHEMA: usize = LABELS_SCHEMA.saturating_add(1);

/// NULL preserves unknown provenance on rows written before this addition.
const TITLE_ORIGIN: &str = "ALTER TABLE sessions ADD COLUMN title_origin TEXT;";

/// One provisional append follows the head; the integrator alone rebinds it.
pub(super) const SUBAGENT_SCHEMA: usize = TITLE_ORIGIN_SCHEMA.saturating_add(1);
/// Current delegation is metadata, never a rewrite of creation provenance.
const SUBAGENT_RELATIONS: &str = "ALTER TABLE sessions ADD COLUMN detach_receipt TEXT;";

/// Provisional next migration; integration re-points this one constant.
const REGISTRY_SCHEMA: usize = SUBAGENT_SCHEMA.saturating_add(1);

/// Schema 21 (`super::listing`), appended after the registry at the node-line merge.
pub(super) const BACKGROUND_SCHEMA: usize = REGISTRY_SCHEMA.saturating_add(1);

/// Every migration, oldest first. Index `n` takes the schema from version
/// `n` to `n + 1`.
pub(super) const MIGRATIONS: &[&str; super::WORKSPACE_LABELS_SCHEMA] = &[
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    SIDE_CONTEXT,
    USAGE_LEDGER,
    INBOX,
    LABELS,
    TITLE_ORIGIN,
    SUBAGENT_RELATIONS,
    super::workspace_registry::SQL,
    super::listing::SQL,
    crate::workspace_labels::MIGRATION,
];
