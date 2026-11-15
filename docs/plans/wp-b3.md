# Per-turn file checkpoints: contract-first plan

Status: ADR 0077 design contract accepted, 2026-11-15, on `a852bde3`.
Node implementation is authorized on base `d954fa3a`, skeleton first. New
calls remain unregistered and services stay test-only until Quality B is
announced. The app waits for the centrally supplied app baseline.

## Reference and existing assets

The exact source mapping, alternatives, scope and stated differences are in
[ADR 0077](../adr/0077-per-turn-file-checkpoints.md). Paseo's files mode and
Claude SDK rewindFiles define files-only behavior; GOAT supplies the surveyed
turn-associated presentation. This is not conversation rewind. The existing
removed-worktree snapshots restore into a fresh worktree, not into a turn.

Decision: modify existing session/workspace infrastructure, with new focused
checkpoint modules. Reuse RelPath, hardened Git enumeration, regular-file
checks, bounded channels and existing hashing/storage crates. Do not introduce
a provider SDK or infer snapshots from tool-call counts. Native link session
and workspace asks already carry conversation rewind and snapshot list/drop;
reuse their typed dispatch pattern only when the new node contract exists.
Kit dialog, alert and toast already exist; no new general control is needed.

## Sequence after contract acceptance

1. Central owners supply the updated app baseline. Do not independently merge
   node history into this package or invent a protocol/schema number.
2. Implement/review bounded node capture, private persistence and lifecycle
   handshake with fakes; no advertised ready checkpoint until durable publish.
3. Implement/review authenticated preview, restore journal/status and protocol
   adaptation with crash/race/permission tests. A failed restore is not success.
4. Only on the reviewed node contract: pure native checkpoint state and tests,
   typed link commands, turn marks, complete paginated confirmation, and result
   presentation. Shared shell/link roots receive registration lines only.
5. Follow the current package gate/capture procedure once at package end;
   report real tested behavior, explicit scope/platform limits, and unknowns.

## Resume point

Completed: reference/source research and accepted ADR 0077 design contract
(`a852bde3`). Preview/journal/verification share the frozen effective
permission-policy image. The actor's capture wait is bounded without claiming
blocked I/O is cancellable; timed-out workers retain their bounded slot and
reservation until cleanup. Expired generations cannot publish ready checkpoints.
Non-root metadata and held-I/O barrier tests remain required evidence for later
implementation, not results of this document.

The coordinator subsequently assigned the node implementation on `d954fa3a`.
The staged code and its current verification status are recorded below. No
registered capability, real prompt-actor wiring, user-facing restore or app
capture is claimed. The final schema/protocol assignment remains integrator
work; app code waits for the shared app base.


## Node implementation: skeleton

The node base is `d954fa3a`. The accepted contract is imported without the
older app history. Per the central numbering rule, the checkpoint SQL appends
one migration after this base's title-origin schema; the integrator re-points
its constant in merge order. No gaps or reserved placeholder migrations.
The migration runner is unchanged; rollback fixtures remove checkpoint tables
first. No RPC is registered, FILE_CHECKPOINTS remains None and Quality B is an
explicit service wiring prerequisite.

Skeleton modules: proto/checkpoint validated images/metadata/limits/identities;
store/checkpoint one SQL schema and transaction tests; node/checkpoint admission,
quota and publication fence; session/checkpoint bounded acknowledgement seam;
workspace/checkpoint named ignored filesystem proof obligations. Service,
store and session mechanisms remain `cfg(test)` (A10). Existing crate edges
are unchanged. Ignored tests represent missing behavior, never a pass.

Current staged implementation also contains real SQLite candidate/plan/restore
admission and idempotency operations, strict Git path enumeration, Linux
handle-relative private raw blobs, aggregate streaming budgets, canonical
image fingerprints, effective-target change classification and asynchronous
capture deadline/publication/cleanup seams. Scope is still staged: there is
no registered RPC, service auth path or real prompt-boundary wiring.

Narrow checks so far found and prompted fixes to serde macro attributes,
public error documentation and a migration-fixture item placement. No green
compiled gate or runtime pass is claimed yet. Filtered tests are queued.
Next: obtain narrow green; fill durable publication activation/recovery and
per-file journal restore/rollback, real session/workspace exclusion, bounded
preview pagination/quota/retention and service authorization. Replace remaining
ignored proof obligations only when the corresponding behavior exists.
The app stays untouched until the shared app base is announced. Runtime
permissions, held I/O and crash proof remain owed.


### Verification and next fill-in point

The proto crate's local all-target clippy completed without errors. The broader
frame has had compile/lint failures, preserved in the evidence outside the
repository; fixes include explicit path inference, private schema reexports,
checked deadlines, row-count handling, bounded heap buffers and test assertion
syntax. Five-crate clippy and filtered checkpoint tests are pending; these are
not passing runtime evidence. No whole package gate or code review is claimed.

Remaining actual mechanisms: durable Ready activation after accepting a live
candidate, per-path restore and guarded undo, full preview/pagination/auth,
retention/pins and quota release after real cleanup, and real session/workspace
exclusion. The previously accepted held-I/O and metadata proof obligations are
not waived. A test-only callback seam does not prove actual prompt delivery.

### Frame checkpoint and baseline update

The five changed crates' all-target clippy passed on `fa83036e` (no lint
changes). Filtered behavior tests are still pending; no runtime pass follows
from that clippy result. Guarded filesystem restore/exchange/undo sources and
their real-file tests are now written but not compiled yet, preserved before
consuming the announced Quality B baseline. They remain test-only.

The announced node baseline is `29eebb8b`; the app package baseline is
`8d5513e4`. Merge them in the separate owned worktrees. After Quality B, add
pending unary rows with FILE_CHECKPOINTS still None, typed request/response
values and explicit ownership/forwarding policy. Registration is not protocol
assignment or permission to advertise an unfinished capability.
