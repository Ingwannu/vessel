# E2 actual Devices delivery fixture boundary (proposal)

Current checkpoint: exact 9878 has bounded V1 source/strict/runtime and
normal default App/VM-quality evidence. Actual wasm remains RED, independently
reproduced on the unchanged announced base with the same 27 diagnostics and
two warnings. The coordinator accepts no new wasm diagnostics against that
base, lifting only this inherited-defect adoption hold; wasm is not passed.
The current-line union and its source/check handoff remain due. W2 now owns
the landed product guard exposure and actor registration; reuse them without
duplicate installation. Earlier sections below retain historical checkpoints.
This plan changes no root crate, SDK API, protocol, store or production route.

## Decision Log

- Purpose: the App must receive a real actor-minted Devices Answer, hold it
  across retirement, and refuse installation while its copied Fleet is stale.
- Existing asset: Link owns State/admissions, the typed framed peer and the
  nondefault testing feature; App already enables that feature only as a dev
  dependency. A detached queue cannot mint an admitted lease or DeliveryGuard.
- Constraint: dependency builds do not inherit cfg(test) or dev-dependencies.
  W2 actor.rs must remain blob9bb5858b; its assertions cannot become feature
  code. Current peer/deadline/Home assertions are not feature-safe either.
- Alternatives: a fabricated guard, second codec/connector, SDK reexport or
  normal RPC access in the App would bypass the boundary under test. Merely
  widening cfg would fail both dependencies and strict lints.
- Choice proposed by the lead and coordinator: one optional native Link RPC
  edge enabled only by the existing testing feature, one fallible framed-peer
  core, private unit assertion adapters, and one opaque scenario facade.
- Why: both fixtures use the same State admission/dispatch and framing owner;
  App never receives a client, State, allocator, lease, sender or guard factory.
- Cost: the native crate map explicitly records the optional edge; feature
  and target isolation require actual resolved-graph and build evidence.

## Manifest and architecture edits, exactly

1. `link/Cargo.toml` features: replace `testing = []` with
   `testing = ["dep:vessel-rpc"]`. There is still no default feature.
2. The existing non-wasm dependencies gain
   `vessel-rpc = { workspace = true, optional = true }`. Keep the current
   non-wasm dev-dependency on vessel-rpc. No root Cargo file is changed.
3. App Cargo stays byte-identical: testing remains enabled only in its
   dev-dependency on Link. Resolver3 keeps that test selection out of the
   normal App binary build. There is no capture or fake-invitation mode.
4. `spikes/native/Cargo.toml` changes only the existing vessel-rpc comment
   from dev-only to unit/explicit-testing access. Version/path stay identical.
5. `spikes/native/README.md` Link dependency cell gains `vessel-rpc`, with
   prose limiting this direct edge to non-wasm testing. Production calls
   still go through vessel-client. The diagram annotates the conditional
   test-only edge. Add a Names-only row: `vessel_rpc` in `vessel-app-link`
   only `src/testing/peer.rs`; it cannot enter devices or production transport.
6. `docs/ARCHITECTURE.md` gains a short native test-fixture paragraph explaining
   the conditional edge and data flow, referring to this plan. Root crate
   rows and root ADR0001 edges remain unchanged; no root dependency is added.
7. Cargo.lock must remain byte-identical: vessel-app-link already records
   vessel-rpc for its dev dependency. Any lock change stops implementation
   for owner inspection, rather than being accepted as incidental churn.

## Module changes, exactly

- `link/src/lib.rs`: the existing non-wasm testing module declaration becomes
  `cfg(any(test, feature = "testing"))`, with a documented public module so
  the opaque facade is accessible to App tests. No other normal entry point
  changes and no line-budget increase is allowed.
- `testing.rs` remains the single fixture owner; it declares the fallible
  scenario facade, reexports only deliberate facade values, and keeps unit
  assertion/deadline helpers behind cfg(test) module declarations. Feature
  code uses bounded typed errors, never unit assertions or raw peer text.
- `testing/actor.rs` is installed exactly as blob9bb5858b and stays unit-only
  via its module declaration. Add only its `pub(crate) mod actor` registration;
  do not import W2's history_calls initializer or change its frozen source.
- `testing/peer.rs` retains the ONE framed reader/writer and NodeClient::over
  using existing vessel_rpc::Framed. Refactor that core to typed fallible
  results. Unit-only assertion adapters retain the current connect/controlled,
  Peer field/method shape expected by frozen actor.rs; no copied codec.
  Unit-only observation adapters stay separate from the feature-safe core,
  so unused subscription/injection helpers are not exposed merely for lints.
- A new `testing/scenario.rs` owns exactly one private State and framed peer
  per scenario. Its Link command queue is the queue the actual Shell submits
  to; its pump invokes the SAME State.handle, admission and Devices worker.
  It emits genuine Event::Devices answers and handles original retirement
  through the existing command. No second actor implementation or registry.
- The facade offers bounded dispatch/pending-call/reply/event/retire/close
  operations. Tests can script typed protocol outcomes and inspect public
  original Context facts, but cannot replace an Answer's private guard proof.
  No raw NodeClient, State/Connected, allocation source or DeliveryGuard
  constructor is public, including through testing feature access.
- Home creation/cleanup is private and fallible. No global environment change
  or real saved book is used. Peer-task abort ownership is installed before
  handshake and retained through borrowed joins on timeout/unwind. The facade
  has named phase5s and total30s limits, frame65536 and channel4 bounds, and
  a named bounded transcript cap checked before recording. A retained queued
  guard must not retain a peer-side NodeClient clone or the original transport.

## Tests required before a handoff

1. Actual Shell device dispatch through its Link queue, framed authority/list
   response, and genuine guarded Answer installed at the production handler.
2. Hold that answer, retire original admission through the actual actor, keep
   the Shell's old Fleet uninstalled, then call the real App receive handler:
   no old grant/list/credential can populate the page before a new frame.
3. Same-node reconnect and alias A-B-A: old answer, callback and revoke intent
   cannot apply to, retire or clear a successor; each request ticket settles.
4. Current sent revoke fenced after reply is uncertain, with input/intent
   recovery preserved; pre-dispatch refusal remains distinct. No replay.
5. Full bounded egress, accepted-send-before-install, physical close and
   sender-drop-lastActive. Weak guard drop/clone never extends transport life.
6. All seven existing/new relay lifetime cases and node work stay separately
   scoped; this fixture plan claims no relay or full E2 feature verification.

The facade exercises framed RPC identity, not TLS authentication or physical
phone behavior. Those claims cannot be inferred from a memory-only peer.

## Feature/graph proof and tool boundary

Run actual VM checks for the default App binary, explicit no-default-features
binary, feature-enabled Link strict all-target Clippy, App tests, and wasm
view check. No local Cargo invocation is permitted. All roots remain unchanged.

An inverse tree for vessel-rpc cannot honestly be expected to be empty on
native App: vessel-client already has its normal RPC dependency. Prove that
only the EXISTING SDK path remains when testing is off, and that the NEW
Link-to-RPC direct edge appears only with testing; record actual graph outputs
for both selections, plus wasm target exclusion. Do not relabel a transitive
SDK edge as feature leakage or claim an empty baseline graph.

The current remote helper accepts only check/clippy/build/test as raw Cargo
verbs. A cargo-tree/metadata proof therefore needs the build-tool owner's
allowlisted read-only selection; do not SSH around it or start local Cargo.
Feature-off compilation alone is not a resolved-graph export. Lock-zero and
strict checks are required in addition to the graph records. V1 confirmation
of this exact plan precedes source/Cargo installation.


## Plan confirmation and proof qualifications

The lead and V1 accepted this plan boundary at9dcba966, not source or runtime.
Normal App binary/graph evidence is separate from all-target/test builds that
activate App dev-dependencies. A wasm View check alone is insufficient: also
check the actual Link target with testing plus its target-filtered graph where
supported, or retain that Link isolation as unverified. The build-tool owner
has now provided allowlisted VM tree/metadata commands; none run locally.
The new facade's bounded blocking test operations borrow the original Link,
so its one runtime stays alive during each phase; they are refused from a
nested Tokio runtime. They never add a product UI blocking path.


## Initial source checkpoint

The optional dependency, single bounded framed core, unit assertion adapters
and opaque scenario are now prepared. The inert detached runtime is not used
to create leases: the facade constructs one I/O/time-enabled Link runtime and
attaches the real State dispatcher there. Each synchronous test step borrows
that exact Link and refuses nested Tokio execution. State retirement and peer
close are distinct controls; no client or validity sender escapes.

The frozen actor blob stays exact and has a real in-crate dispatch regression.
An App regression now submits its actual authority/list commands through this
framed actor and calls the production Devices answer handler. Those are source
cases only, not executed proof. Legacy Answer has no queued DeliveryGuard yet;
this fixture does not close that gap by itself. No runtime/feature-off/wasm or
authority acceptance is claimed from static size/format/token checks.

The next App regression holds a genuine authority response, retires its actor
admission, keeps the Shell's copied Fleet unchanged, and calls the production
answer handler. It must not install Admin/list facts. That source case is an
expected missing-guard regression on the current legacy envelope; it has not
been executed and is not reported as either a red or green runtime result.
The actual receipt/guard consumer must close it before the final feature gate.

## Real Unary preparation on the current line

The portable branch is based directly on c3429952. It excludes unrelated old
G4 Alert/HostErrors and historical cumulative notes. Actor blob9bb remains
exact; root/Cargo.lock changes are zero. Existing pending351 checks remain
prior fixture-source evidence, not checks of this guarded consumer.

The product exposure removes only the Delivery module/export, selected guard
constructor and bounded queue staging attributes. Resident follow stays literal
cfg(test). Devices Answer fields are private: only actual actor completion may
carry successful Outcome with its original weak guard. Local unsent/browser
refusals expose a safe error-only factory, never a guard or successful value.
The bounded producer races retirement through egress and drops its selected
strong lease before a code-only fallback publication. Original per-feature
work capacity remains held until publication finishes.

App first matches the full original context, then synchronously receives the
sealed guarded result before any installation. It retains only the weak guard
and clears old facts before drawing or dispatching a retained control if that
proof has retired or its transport closed. No copied Fleet value substitutes
for validity. Pending invitation/revoke state is captured as one bounded typed
unconfirmed notice before clearing credentials/authority; input remains in the
same field. The notice is shown only for the explicit same alias and NodeId,
including reconnect, never for a replacement physical machine. A confirmation
that was never dispatched is not uncertain. Preflight refusal is definite;
remote permission refusal stays a permission result, not a disconnected scope.
No automatic write retry occurs. A local write deadline also returns a typed
unconfirmed result while preserving live authority; read deadlines remain
retryable failures. This is not a node-side exactly-once guarantee.

The old detached success-Answer tests have been replaced with real framed
actor callbacks. Added source cases cover held authority before Fleet, access
refresh removal, before-frame callback refusal, queued successful revoke then
retirement with retained input/unconfirmed result, and an old list versus a
same-node successor's pending revoke. They have not yet run on this source.
Symbolic W12 queue/EOF tests remain distinct from these App Unary cases and
from live resident stream/EOF/error-frame proof, which this packet does not own.

The T-approved fixed native-testing-graph step (label devices, nonpriority
short, six locked/offline records) is now available. Every result is a graph
record, not a compile pass. First direct-Link-off graph98 passed on prior source;
combined graph/default/test/wasm source proof is still owed. The earlier local
boundary-script cargo-metadata process deviation remains recorded separately.


A follow-up source correction makes permission refresh unavailable while a
sent invitation/revoke is pending: it cannot silently discard its result or
confirmation. Authority refresh first checks the retained weak proof as well.
The corresponding pure case and a genuine framed synthetic-invitation case
are authored, not executed: the latter holds the returned credential in memory,
retires the actor, and verifies that no Pairing/link/QR reaches the page.
The synthetic invitation is not written, logged, or used in captures.


The browser receipt has only the local error body; native-only success/retired
constructors and their private body variants are target-gated. Public guard
representation stays uninhabited on wasm, with no demo minting path. This is
source preparation; the actual Link testing-target check is still required.


Local invitation/revoke deadlines now have a separate typed Unconfirmed
outcome rather than a fabricated remote Unavailable refusal. The pure reducer
settles only the matching current ticket, preserves live authority, and merges
uncertainty for both kinds in one bounded record. It never retries or clears a
successor. The whole recovery sentences cover retirement and a deadline alike.
The source tests distinguish write deadlines from read failure and current
versus wrong/already-consumed tickets; actual timer execution remains owed.


## First fixture check, retained red

The351 fixture-source remote strict check ended native-cargo101 after actual
View/Link checking; App did not reach its own check. Thirteen Link findings:
large State-dispatch futures propagated through the async phase wrapper, one
missing semicolon, and four direct reads of the existing underscore-named
Link keep-alive field. No runtime case ran. Evidence stays fixture-clippy-01;
it is not a result for this later guarded consumer or the new base.

The correction boxes the typed phase/driver futures with static dispatch,
adds the semicolon, and privately projects the original Link-owned runtime by
pattern rather than renaming the product keep-alive field or exporting a
handle. No second runtime, assertion weakening or lint allowance is introduced.
The queued fixed graph remains pinned to e284 (identical Cargo/lock edges).
Changed-source strict checks are next on the corrected straight package head.


The corrected9f141b87 strict run then stopped in the View catalog build:
all four new uncertainty entries need their own translator-context comment;
one preceding group comment is insufficient. The next correction gives each
message its place and semantics, without changing copy or generated APIs.
No current-source Link/App check or runtime result is inferred from this red.


The8fa strict run next reached View and returned unused_self for the new
whole-message helper. It is now a Translator method, selecting that catalog
instead of taking an unused legacy DevicesWords receiver; no dummy self read
or lint suppression is added. This failed run executed no runtime tests.

Explicit normal Scenario close also drains the SAME bounded device-call hub
permits after dropping State/admission, closing egress and joining the peer.
It does not claim workers are gone from an abort request or an ignored handle.
The feature-only drain has a named Workers phase; cleanup attempts peer,
worker and Home completion even if an earlier cleanup result failed. No new
registry, counter, raw client or guard constructor is exposed. The new actual
App source case closes with a request still in flight while keeping the App's
weak proof, then checks before-frame refusal and its unconfirmed write notice.


The0153 strict run checked the changed View, Link and App, then returned two
unit-test qualification lints created by the promoted SendFailure reexport,
plus the App receipt field visibility, three deliberately consumed retirement
booleans, and two test-only formatting/checked-date lints. The correction
changes no assertion, capacity, deadline or state decision. The two e42 test
paths now name the same enum directly; that is the only shared-test change.
No all-target green or runtime result is claimed for0153.


## Exact Unary delivery verification before the line union

The isolated source is 9878a4b0. Strict run
`devices-narrow-20261115T155113Z-9878a4b0` completed with zero warnings.
Kit, Link and App were actually checked in that run; View was cached from
the preceding unchanged View source. Runtime run
`devices-narrow-20261115T155547Z-9878a4b0` actually compiled View, Link and
App, with 1296 passed, zero failed and two ignored across eight sections.
All eight framed App actor regression cases passed. This proves the authored
Unary admission, queue installation, uncertainty and cleanup cases; it does
not prove TLS authentication, actual principal clipping, GUI/focus, streams,
physical phones or other platforms.

The same source default ordinary App binary and VM quality completed in
`devices-narrow-20261115T162155Z-9878a4b0`; fonts, quality and build all
completed zero, with actual View/Link/Kit/App compilation. No capture or
installed-service behavior follows from this binary build.

The fixed six locked/offline graph records in
`devices-20261115T153639Z-e284af90` completed zero and their manifests
remain byte-identical through9878. They show the new direct Link-to-RPC
edge only in native explicit testing, the existing SDK-to-RPC App baseline,
and no RPC/client edge in the wasm graphs. These are graph observations,
not compilation evidence.

Actual wasm Link/View with testing enabled in
`devices-20261115T164815Z-9878a4b0` FAILED: cargo101, 27 errors and two
warnings. Static source comparison attributes the reported modules to
inherited source; classification still requires the separately authorized
announced-base testing-OFF check. The package testing-OFF leg was never
submitted after that failure. Do not label graph isolation as wasm PASS.

The subsequent W2 A4 packet provides the product guard export, selected
constructor/queue and frozen actor registration. On the actual line union,
reuse those source bytes, preserve its protocol getter and history hub,
and add only this package's real Devices caller and explicit-testing
Scenario dependency/module boundary. The preserved isolated evidence above
is not a check of that future union.

### W2 integration adjustment

The original fixture plan described an isolated c342-based installation and
excluded the then-unrelated history initializer. That is historical preparation,
not an instruction to remove the now-landed A4 history owner. The current line
union must preserve its history State field and initialize that existing hub in
the same fixture State factory. Its actor remains unit-only, blob9bb5858b.
The selected protocol getter and all existing A4 guard/queue exposure stay
byte-identical. The fixture still closes/drains the Devices hub it exercises;
it does not claim to drain every other feature's outstanding work.


### Unchanged-base wasm classification and the coordinator's ruling

The authorized ordinary run `devices-20261115T171319Z-82ca07a7` checked
exact announced 82ca07a7, without package changes and with testing OFF:
`check --locked -p vessel-app-link --target wasm32-unknown-unknown`. Fonts
completed 0, actual View compilation/Link checking ended cargo101. Its 27
primary error records match 9878's testing-ON failure exactly in code, message,
source path, line and column. The same LinkError and SessionId imports produced
two warnings. All nine primary diagnostic source blobs are also identical.
The author verified exact spec/checkout, both step receipts and raw log SHA256
values; the baseline native-cargo log is
`2c50911d7a3c410af23f87ce8b8d1dadad8fbaf43c76900ba0a9f6445f35b28a`.

This is actual unchanged-base reproduction, rather than static attribution.
Per the explicit classification rule, no second baseline ON check or package
OFF check was submitted and no cross-owner platform repair was attempted.
The coordinator accepted the exact diagnostic match: the existing Link wasm
defect belongs to a separate base row under the lead, and no longer blocks
Devices adoption on the criterion of no new wasm diagnostics against base.
This does not make Link wasm green, prove testing-OFF package compilation,
or upgrade graph records to compile/runtime evidence.

The older isolated checks remain exact 9878 evidence. The receiving line must
retain W2's protocol getter, product guard/queue exposure and history hub,
while the fixture actor remains the unchanged unit-only 9bb blob. Only the
actual Devices caller and conditional testing Scenario edge are added by
this packet. The receiving-base union gets its own changed-source checks and
V1 review; no earlier check is renamed as a combined-line pass.
