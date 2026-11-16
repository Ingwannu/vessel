# Shared live launch admission

Status: scoped prerequisite reviewed and received in the checked node line.
Consumer preparation, policy authority and the common Settings producer remain
separate qualifications.

[Decision Log]
- Purpose and intent: a preparer held across node stop must not start a new
  process while existing actors remain alive for orderly settlement.
- Existing assets: Runtime already owns final actor shutdown and Node already
  owns its early stop token. Reuse those sources and the existing spawn paths.
- Alternatives: a second stop token or preparer-local predicate would drift;
  closing actors first loses the required settlement order.
- Choice: one cloneable read-only observer, one live predicate and cancellation
  observation. It supplies liveness only, never caller authority or a permit.
- Cost: check/spawn is not atomic. Existing cleanup remains for a start racing
  the final check. Held physical preparation and caller/policy freshness remain
  independent obligations of their own consumers.

The existing runtime factory moves unchanged to node/setup before extension,
so the owning node file's size allowance shrinks. No settings writer, disk/SQL
model, recovery coordinator, Journal or restart runtime joins this prerequisite.

Paseo packages/server/src/server/daemon-config-store.ts:655-659 serializes
settings operations. The explicit Rust early-stop/final-runtime distinction
and this shared pre-spawn observer are implementation choices for that
ownership order, not claims that Paseo has the same API.

The factory relocation preceded the admission extension so the move could be
reviewed independently of the new stop observation.

The implemented API is `vessel_session::LaunchAdmission` with private source
fields, `check() -> Result<(), SessionError>` and `cancelled().await`.
`SessionRuntime::launch_admission()` clones the actual sources; Node's runtime
factory supplies the original early-stop token through `with_admission`.
The factory token regression is synchronous and starts no task or process.
Both real held-lookup modes and the positive-control fake case use the same
unchanged `ADMISSION_CASE_BUDGET`, preparation-aware `within` and named phase
waits. Failure closes the existing owners; abandoning the lookup gate refuses
before any fake spawn. Normal and unexpected-success paths still settle the
owned durable stream and runtime. No new task/reader quota or physical-capacity
receipt is introduced.

The scoped check set passed: Node/Session all-target strict on the import
correction, current Session strict after the test-only phase correction, and
five actual focused Node/Session cases. The latter includes both held lookup
modes and the real open-admission control. The checked node receiving gate also
passed. No common Settings/SQL result is used as proof. Recovery and kept-version
work consumers remain unqualified. Real agents, other operating systems and
service-account behavior are not exercised by these isolated fixtures.

The first changed-source strict check found the factory move's now-unused
PathBuf import and an unnecessarily qualified prelude Future bound in the new
phase fixture. Both are removed without changing behavior or assertions. Its
red log is retained; the corrected-source ordinary checks passed.

The fixture phase clock is independently named and strictly shorter than the
unchanged outer case safety net. Otherwise the earlier-started outer clock
would hide the phase's diagnostic. The helper checks that ordering before
waiting; semantic assertions, actual fake controls and product waits stay the
same. Earlier immutable strict evidence remains separate from this correction.
