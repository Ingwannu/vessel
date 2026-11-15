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

## Current producer/consumer boundary

The app tree has consumed package base 6 (18eedad4) through its own line merge.
The node producer remains on its separate checkpoint branch. Its latest source
includes physical admission for ordinary writers and retained startup/lifecycle
operations, actor pause and turn hooks, and pending checkpoint metadata. These
are source changes under validation, not a reviewed runtime capability. The
FILE_CHECKPOINTS protocol requirement remains None and no checkpoint UI has been
implemented or captured in this app tree.

The app will consume the reviewed node contract through the package line, using
Machine.node and the shared pane/dialog/preflight APIs already on this base.
Remaining app work is per-turn marks, all-page changed-file confirmation, typed
busy/stale/limit outcomes, verified result presentation and durable reconnect
status. No local Rust compilation is used; the base's native gate is baseline
evidence only and does not validate this unfinished checkpoint package.


### Package line fixed lock update

Consumed dd13ce37 from the authorized app line after confirming no native gate
was running. It includes G2's reviewed trigger/menu changes and the fixed native
Cargo.lock; --locked is now usable for future app checks. No node worktree head
was merged into this tree and no checkpoint UI has been implemented yet. Node
contract/runtime review and a consumer-ready producer remain prerequisites.
