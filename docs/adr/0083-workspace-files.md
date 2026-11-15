# ADR 0083: bounded workspace file panels and live source state

Status: proposed.

[Decision Log]
- Purpose and intent: supply native file panels with byte-preserving
  downloads/PDF data, duplicate, live file versions and directory discovery.
- Existing implementation and constraints: `file.read` sends only UTF-8
  text or a binary marker; `file.find` indexes files, not directories.
  Existing `Root`, regular-file open, content hashes, git ignore checks,
  and bounded blocking disk work remain the only filesystem boundary.
- Alternatives considered: use local client filesystem paths (incorrect
  for remote nodes); shell out to copy/download (argument/process surface);
  poll every file from the app (wasteful and loses disconnect ownership).
- Choice: additive typed requests under one pending feature constant,
  version assigned at integration. Binary pages carry exact offsets and a
  stable version; later pages require that version. Duplication reserves
  a new sibling exclusively and removes incomplete output on failure.
  Single-file observation waits finitely and reports ready/missing plus
  typed failures. Suggestions are bounded root-scoped discovery, preserve
  exact-path retrieval and honor git ignores and the reference hidden-dir
  allowlist. No persistent schema is needed.
- Why: reuse reviewed path and file primitives, avoid a second dispatcher,
  keep all work authorized by the existing root and principal, and make
  incomplete transfers or unsupported older peers explicit.
- Advantages, costs and effects: binary pages avoid whole-file responses;
  content version checks refuse a mixed transfer. Duplicates do not follow
  links into another tree. Resource bounds are named constants. A watcher
  belongs to the request/connection and cannot survive disconnect forever.
  Existing path checks accept same-user directory swap races (ADR 0027);
  this does not claim an OS filesystem sandbox.

Dependency decision: reuse `notify 8.2.2` (CC0-1.0), already locked for
`vessel-lsp`; its unpacked source is approximately 332 KiB. No second
version is introduced. The standard library has no portable filesystem
notification API. The new owner is `vessel-workspace` for workspace-file
observation, while LSP keeps its separate document-cache lifecycle. Added
app binary size is not measured yet; source size is not binary growth.

Stated differences from the reference: suggestions require a registered,
authorized workspace root, rather than exposing arbitrary home paths;
copying refuses symbolic links, special files and nested `.git`, and is
bounded by total bytes, entry count, depth and candidate-name attempts.
Paseo preserves copied symlinks and has no equivalent whole-copy budget.
The stricter rule prevents a file action from importing data or metadata
outside its ordinary root boundary. Binary pages rehash their bounded
source before and after each page to guarantee a coherent transfer; this
costs I/O compared with Paseo's one streamed handle. A future transfer
actor may amortize that cost without changing the content identity.

Reference: Paseo 0.15.4 `server/file-explorer/service.ts` (workspace scoping,
copy naming and revision guarded reads), `file-explorer/observer.ts`
(file versions), `utils/directory-suggestions.ts` (limits and discovery).
App-facing details and renderer dependency analysis are in the companion
`docs/plans/wp-files.md`. Protocol and schema numbers are never reserved
locally. This feature uses no database migration.

### Integration ruling update

The coordinator replaced the earlier permission to exclude these five rows
from `rpc/tests.rs`'s protocol-10 family assumption. The integrator's next
shared head will replace that assumption centrally with an explicit legacy
list. This package carries no exception to the head test. Its own tests
continue to pin all five rows' pending constant, scope, unary shape and
required protocol through direct/Forward/Relay/Deliver calls. The five
FORWARDABLE names remain appended after the head's entries. W17's shared
`rpc/contract.rs` extraction must be merged from the announced head; this
worktree's temporary equivalent must not become a feature commit.


### Client acquisition and bounded task lifetime

The SDK exposes `read_workspace_byte_page`, `inspect_workspace_directory`,
`duplicate_workspace_entry`, `observe_workspace_file` and
`suggest_workspace_entries`, with a 70-second call deadline above the node's
60-second operation deadline. Timeouts explicitly do not prove that a write
failed: refresh before retrying. The node's tracked task owns its operation
permit until actual completion, even if its caller disconnects or times out.
Observer tasks likewise retain their bounded principal wait and observer
slot until they finish. No second task registry or store is introduced.

`fetch_workspace_file` validates the first page before creating temporary
output on a blocking worker. Its adapter checks every revision, offset,
source-size limit, exact continuation and nonempty nonfinal page before
passing bytes to existing `fetch_chunks`. The caller's existing `ChunkSink`
contract verifies the entire manifest hash in `finish` before publishing
output. This intentionally reuses the fleet transfer writer contract rather
than introducing a second download loop or buffering whole files by default.
Tests cover mixed revisions, malformed continuation, oversized declarations,
empty files, sink refusal and SDK pending-call admission without any frame.
These additions have not yet been compiled or run; all results will be
recorded after the shared RPC extraction/head is adopted.

### Shared head adopted

The companion now includes announced Quality B gate40 head `ba45a6e3` and
base3 `6c782cb6` (including N2's exact c5da5305 e2e receipt fix). The local
RPC extraction was discarded; `rpc.rs` and `rpc/contract.rs` have no feature
diff. All five pending rows are appended to shared `rpc/table.rs`. The
central protocol-10 loop remains unchanged; only FORWARDABLE gains the five
names. Local narrow attempts before base/head merges were cancelled while
still waiting, with no compiler result. The next check covers this committed
wiring and the new typed client/transfer tests, not those cancelled attempts.
