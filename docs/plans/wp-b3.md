# Per-turn file checkpoints: contract-first plan

Status: design draft, 2026-11-15. Product implementation is held pending ADR
0077 review and the central merge of the node contracts into the app baseline.

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

Completed: reference/source research and proposed ADR 0077. No product edits,
merge, runtime test, fake fleet, capture or compiled gate. Contract review is
next; no package UI or node implementation is authorized before it passes.
