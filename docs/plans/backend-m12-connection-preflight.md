# One admitted connection for feature requests

[Decision Log]
- Purpose: queued work must stay with the exact selected book entry and
  original physical client when a name is reused, credentials change or the
  same node reconnects.
- Existing assets: the link actor's `Connected` map, parsed `BookEntry`
  equality, `NodeClient::node()` and the shared `view::Machine.node` field.
- Alternatives: look up the first matching NodeId, make an area identity map,
  clone an epoch allocator, or export a raw client for later resolution.
- Choice: extend the existing owner with an irreversible validity watch and
  one non-clonable epoch source; each feature captures one immutable context.
- Why: the existing owner orders observed book changes, client admission and
  retirement. Feature collectors do not become connection managers.
- Tradeoff: retirement prevents later local effects and exposure; an already
  sent remote write may have happened. No cancellation/rollback/replay claim
  is made, and an unobserved external book ABA is outside the observation
  boundary.

## Source and ownership

`view/src/connection.rs` owns toolkit-free context, tag, epoch and local
refusal types. `Machine.node` remains the single welcome NodeId projection;
the same snapshot adds serde-default `connection_epoch`. A legacy snapshot
with either fact absent has no `Machine::connection_context()`.

`link/src/connections.rs` extends the one existing client registry; it owns
parsed binding comparison and retirement. `refresh.rs` only forwards its
existing refresh/connect hooks. `connections/edits.rs` is the successful-edit
hook: rename retains physical client/agents/legacy follow but retires the old
alias epoch; forget retires the affected admission. Failed edits never enter
the hook. E2 owns book writing and route/upsert APIs; this seam adds none.

`connections/snapshots.rs` retains admitted identity when session reads are
semantically refused. Reachability and a denied session read establish no
scope verdict. Actual transport closure retires identity. `DeviceRetired`
carries only alias/epoch; closed reports return through a weak actor command
so an asynchronous worker cannot publish retirement ahead of older snapshots.

Paseo 0.15.4 `runtime/host-runtime.test.ts:751–837` distinguishes replacing
a re-paired client from learning its already-enforced security pin. An absent
book id becoming the original client's actual NodeId retains admission only
when every other parsed field is identical. Route, credential-source or pin
replacement/removal retires it. Changed bytes inside an unchanged referenced
credential file are not claimed to be observed.

## Unary consumer contract

1. Capture `Machine::connection_context() -> Option<ConnectionContext>` from
   the explicitly selected machine, independently of session-read permission.
   Context exposes `machine()`, `node()`, `epoch()` and `tag()`; it contains
   no credentials, paths or guessed scopes.
2. Carry that context and the area's own ticket/root/workspace identity in its
   command. `State::selected_connection(&context)` returns the original
   `SelectedConnection` or typed `Refusal::{Changed, Closed}`. It searches
   only the exact alias and verifies active epoch plus actual client NodeId.
3. `SelectedConnection::call<R: Unary>(request)` checks before polling the
   client future, races retirement/closure, then fences the completed result
   again before exposing any value or error. It never retries another client.
   Callers retain their existing bounded operation/deadline owner.
4. Return the original context and correlation ticket on every outcome,
   including a local refusal. A feature settles the reserved ticket but must
   not install stale data into another admission.
5. `RequestFailure::Refused` is a local typed verdict. `Call` retains the
   client's structured error for the feature's code-only projection; its
   Display/Debug are redacted. Never display/log peer messages or source
   chains. A successful read is not evidence of Admin or a device identity.

Devices is the first real product unary consumer. Other packages reuse this
same actor API; they do not create an alias/NodeId resolver or raw-client
escape. Machine literals elsewhere use the existing constructor or an absent
epoch, preserving the lead's single public NodeId field.

## W12 staged stream boundary

Only `connections/selected.rs`'s literal `#[cfg(test)] mod follow` stages
the currently unconsumed attach/next wrapper. It uses the same original-client
guard: `follow_session(SessionId, EventSeq)`, `retirement()` and guarded
`SelectedFollow::{context, info, next}`. Next fences both before work and
after awaited completion, including buffered records, errors and EOF. No raw
stream/client is exported. This is test-staged source, not a product stream
or native merge exemption. W12's real resident wiring removes the exact
attribute and reviews product visibility/imports under its final package gate.

The shared framed test peer has one tracked task owning reader and writer
futures, a bounded frame channel and explicit write receipts; no read future
is cancelled mid-frame to inject an event. A subsequent reply on the same
reader is the barrier proving an event was buffered before retirement.

## Verification status

Current actor/Page integration is under narrow verification. The historic
normal Devices visual/revoke pass, pure identity review and dialog producer
checks are separate evidence. This document grants no permission, UI,
physical-device, stream-product or final-package approval. Logs and exact
source fingerprints are retained under `/tmp/work/m12-g4/resumed-20261115`.
No production account, file permission, service or network configuration is
changed; compilation runs as the existing ubuntu UID/GID 1000.
