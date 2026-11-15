# Per-turn file checkpoints: contract-first plan

Status: ADR 0077 design contract accepted, 2026-11-15, on `a852bde3`.
Product implementation remains held pending central ownership assignment and
the merge of the node contracts into the app baseline.

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

No product edits, merge, runtime test, fake fleet, capture or compiled gate.
Next: central owners assign baseline, implementation, schema and protocol
integration. Contract acceptance does not assign that work or prove runtime
capability; no package UI or node implementation starts before that sequence.


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

Next: narrow skeleton compile/clippy and behavior tests, then implement private
capture storage, durable journal/idempotency and asynchronous worker seams.
The app stays untouched until the shared app base is announced. Runtime
permissions, held I/O and crash proof remain owed.
