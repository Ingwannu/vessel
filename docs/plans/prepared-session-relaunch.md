# Preparing a relaunch before final session admission

Source-only library handoff. The named shared preparation view and generic
launch fence still require their exact combined checks; no Node engine,
settings, journal, new RPC or startup activation is included by this leaf.

[Decision Log]
- Purpose: the node must obtain fresh authority after physical preparation
  and immediately before actual session spawn, without detaching real work.
- Existing assets: the runtime's command preparation, original Arc conversation
  claim, tracker, run_structured and reopen publication gate.
- Choice: prepare_again returns one opaque PreparedAgain; start_again delegates
  to it and starts synchronously. A second resume builder would diverge from
  ordinary session setup and was rejected.
- Consequences: the caller may hold a prepared value across fresh admission.
  It must still own the original node attempt/Store obligation and apply its
  final grant/policy checks; preparation is not session authority.

The preparation method retains the original Arc claim and passes a clone to
the canonical physically retained command resolver. The returned value keeps
that original through actual spawn/live registration, then calls borrowed
keep. Failure or drop releases only its own claim; no replacement is created.
Bounded native-version helpers may run during command preparation, so the API
does not claim preparation starts no process. The restored session process
starts only in PreparedAgain.start, under final caller and generic admission.

The existing recorded state, queue, limits, provider options, setup, resume id,
replay behavior and publication gate remain. The old resumability fixtures are
unchanged. The common adapter owner places this same body behind its ONE
PreparedRuntime view and supplies the actual preparation gate/LaunchAdmission.
The leaf's default route still delegates to the current runtime gate until
that join; it is not a second custom-gate path or a ready Node integration.

No tests/strict/gate result is asserted for this source leaf. Actual shared
lookup/claim/registration/drain and kept-helper fixtures, plus the final
consumer host predicate and enclosing package gate, must qualify the join.

## A preparation-scoped host view

[Decision Log]
- Purpose: the node's existing recovery/listing owner supplies its original
  synchronous admission arbiter without replacing the runtime's canonical
  node/runtime close observation.
- Reuse: `SessionRuntime::with_preparation` returns a limited `PreparedRuntime`
  over the existing command, claim and task-tracker paths. It captures this
  runtime's actual `launch_admission()` internally; callers cannot supply a
  foreign handle. There is no `Deref`, raw command API or second quota.
- Ownership: the same original owner, host gate and canonical close handle
  travel through physical work in private `RetainedPreparation`. Default routes
  use the same view with their existing runtime gate. A temporary view returns
  `PreparedAgain` borrowing the original runtime and holding its original claim
  until live registration. Its final start rechecks the same close observation.
- Host policy: the borrowed cancellation check delegates to that canonical
  handle and the original caller token. The host calls it under its existing
  arbiter beside its deadline/generation checks. Subscribe before checking;
  temporary `Wait` uses that existing watch and rechecks the same owner. Pending
  Off can roll back without permanent cancellation or authority recapture.
- Limits: this is preparation admission, not caller/device/grant/session launch
  authority. The node retains its original Attempt and Store obligations, pins
  actual preparation through observer cancellation, signals its existing token,
  then awaits actual settlement and applies its fresh final session checks.
- Physical boundary: version admission is followed by synchronous process spawn
  in the same poll, with original owners already retained. The spawned child
  transfers to the existing tracked observer; root uncertainty retains ownership
  and withholds completion. Canonical close observation alone does not prove
  this process behavior, atomic check/spawn or descendant death.

The canonical observer/factory comes from the checked node58 head. The new view
and held-work controls need their own exact joined strict/runtime evidence.
The positive temporary-view case checks original conversation exclusion through
registration; separate node-only and combined-drain controls prevent runtime
shutdown from masking a broken node-source observation. Fake fixtures use the
existing lookup hold and fixed version execution marker. None of this activates
recovery policy hooks, voice reload, provider identity/sign-in proof or a wire
method.
