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

Exact-source actor/Page narrow verification passed as recorded below; V1
source review and final package verification remain pending. The historic
normal Devices visual/revoke pass, pure identity review and dialog producer
checks are separate evidence. This document grants no permission, UI,
physical-device, stream-product or final-package approval. Logs and exact
source fingerprints are retained under `/tmp/work/m12-g4/resumed-20261115`.
No production account, file permission, service or network configuration is
changed. Local source preparation used verified ubuntu UID/GID1000;
the approved remote runner used its configured builder account. Its numeric
UID/GID was not separately collected in these retained run logs.


The first immutable remote candidate `93aa3fb3a` failed Clippy compilation:
the generic typed revoke returns wire Ack rather than the previous client's
unit wrapper, and two test barriers incorrectly used Default on the unit
ListSessions request. The correction explicitly discards Ack after success
and constructs the unit request. Failed run
`devices-narrow-20261115T084711Z-93aa3fb3` remains retained; no pass is
claimed from it.

The second candidate `d736c1c56` compiled product types but its test target
was refused by Clippy: the shared peer bootstrap exceeded the 100-line
function budget and one barrier ignored a returned session vector. Its node
metadata construction is extracted as a named fixture helper and the barrier
now checks the returned empty vector. No lint allowance or size record grew.
The red run `devices-narrow-20261115T085439Z-d736c1c5` is retained.

The clean-base candidate `4de868b0` exposed App fixture compatibility
errors: old two-argument shell constructors and a sidebar Machine literal
needed to remain valid, and internal closure-report settlement must return
the existing typed settlement model. The legacy constructor remains intact;
a separate test-only detached-link helper serves Devices. The extra epoch
is absent in unrelated fixtures. Actor-only bookkeeping is ignored by UI
settlement, with no synthetic retirement event or notice.

The shared RequestFailure also deliberately has no automatic raw-call error
source chain: retaining structured detail for a feature mapper must not let
a generic error reporter bypass redacted Display/Debug. A canary regression
covers all three surfaces; no real credential is used. Core imports use
view::connection directly, keeping Devices aliases as compatibility only.

The first runtime App suite on30af passed74, failed1, ignored2; cargo then
stopped before Link/View suites. The failing no-frame Escape case changed
input modality: pinned GPUI window.rs5647/6053 redraws a dirty keyboard tree
before dispatch. Its fixture now enters keyboard modality first and sends
the actual KeyDown event without running the executor; it also asserts no
render occurred during delivery. This is a stronger precondition check, not
a skipped assertion or a relaxed production stale-intent guard.

An observed successful rename also retires a stale cached destination, not
only its source; cached destination absence may not have been refreshed before
the book edit. Both retire before event-channel backpressure. Source follow
is preserved and renamed; an overwritten destination follow is cancelled.
No second resolver or client-book writer is added.

The2782c216 probe stopped at the new rename tests' constructor: the shared
validated book Name uses its explicit parse method, not FromStr. Only those
fixture calls are corrected; the failing compile log is retained.

The f267 runtime batch completed1186/1/2: Link78/0/0 and all View sections
passed; one App no-frame assertion remained. Frame measurement now occurs
inside the input-dispatch update, before a callback's notify can flush a
normal post-input redraw. The newer confirmation/context assertions remain
unchanged. This probe is still red, not a waived runtime result.


## Immutable producer result (2026-11-15)

The minimal source line is `backend/m12-shared-device-connection`, based
exactly on gated `074ade16`, through
`3f4fc4888f99ac90b10697f017b3fde0599bad3c`. It excludes the unrelated
HostErrors kit presentation and the moving full Devices branch. Core
App/Link/View blobs match the authored worktree; only this selected producer
is proposed for adoption. The gated base already supplies captured dismissal
and ReturnFocus, with no new kit source changes in this producer.

Exact-source remote all-target Clippy for View/Link/App passed:
`devices-narrow-20261115T093416Z-3f4fc488`. The same source's test batch
passed **1187/0/2**: `devices-narrow-20261115T093535Z-3f4fc488`. Both runs
have native-fonts/cargo zero and no skip. App75/0/2, Link78/0/0 and View
sections1034/0/0 are separate; no real-node permission claim follows from
the framed semantic-Forbidden fixture. Actual changed App/Link compilation
is retained; unchanged View sources reuse valid artifacts and run all tests.

The input regression measures frame identity inside window dispatch; a normal
notify-driven redraw at update completion is not mistaken for a newer tree
handling the input. All six intent/connection by Escape/scrim/glyph cases
pass, retaining the newer confirmation. The earlier red runtime probes remain
red; no assertion, deadline or failing case was skipped.

Evidence: `/tmp/work/m12-g4/resumed-20261115/preflight-remote-{clippy,test}-07/`
contains raw logs/steps; `preflight-final-evidence.json` independently sums
every test section. `preflight-source-3f4f.json` pins2398 tracked Rust,
Fluent, Cargo and toolchain source files; selection and derivation from the
prior manifest are explicit, and all changed blob bytes are hashed anew.
V1 source review is pending. No whole gate was rerun on unchanged source.

Limits: W12's follow wrapper remains literal cfg(test)-staged, requiring
real resident wiring, attribute removal, visibility checks and source review.
The guard's synthetic ready-result EOF/error tests are not real server EOF
or error-frame coverage. Device effective-scope/self UI, pairing, relay
consent, physical devices, captures and final E2 gate remain unverified.
No live node, fake fleet or application was started for this checkpoint; no
real invitation/QR was generated. Synthetic peers only. Local preparation ran as verified ubuntu UID/GID1000; the remote runner
account was configured by the approved tool, without separate numeric UID/GID
attestation retained here. No live service-account flow was exercised and
no production service or permission changed.
