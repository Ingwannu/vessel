# E2 actual Devices delivery fixture boundary (proposal)

Status: V1/lead plan boundary accepted; fixture/Cargo source preparation is
installed, with compile/graph/runtime proof pending. Delivery export remains
staged and the actual queued-install guard consumption is not implemented yet. The relay lifetime security package remains first. The native base
is gated e1b73c9e; a later announced gated line replaces it before submission.
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
