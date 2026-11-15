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
