# Startup recovery R2: owned engine and product admission

Current candidate: the accepted R1 values and hidden reply projection are
retained on the announced Quality B head. The passive ordered journal is now
connected to node boot, ordinary person-stop/resume and the same owned settings
engine. Client helpers and the explicit CLI inputs use canonical admission;
recovery's protocol remains Option None and no positive new-field request is
claimed. The final Linux root/web package gate passed on `23cf77eb`;
independent source review and integration remain pending. Sections below
record earlier baseline/staging decisions in their original sequence; their
old staging status is historical, superseded by this paragraph and the final
verification in wp-n5-restart.md. The accepted ADR 0098 contract defines behavior.

[Decision Log]
- Purpose: implement the recovery settings and startup coordinator while its
  RPC fields remain unregistered.
- Existing parts: the Settings engine owns saved-value precedence and runtime
  application; DataDir owns parsing, atomic private writes and the head's
  unreadable/later-file set-aside policy. Recovery must extend these owners.
- Choice: modify/reuse them with a bounded node-owned FIFO operation. Stage
  the node's recovery service under cfg(test), with tests using actual store,
  fake providers and loopback. No duplicate settings engine or second parser.
- Alternatives: a second settings actor, raw JSON in the node, or a test-only
  RPC/authority bypass would lose ordering or cross ownership boundaries.
  Existing Tokio and workspace APIs suffice; no new dependency is planned.
- Consequence: source reviewed, staged code stays on this track until the
  later wiring commit. A staged test pass is not product startup approval.

## Merge baseline

The new node head owns PROTOCOL_VERSION, store schema, method requirements,
all existing tests and the settings-file set-aside rules. R1's feature fact
remains Option None; no request, call or new migration was added by this merge.

Conflicts resolved under the shared merge rule:

- README: retain the complete node-head text, append R1's status paragraph.
- ARCHITECTURE: retain the complete node-head text, append R1's value section.
- rpc/older.rs: retain the head's label-aware projections and tests byte for
  byte; reapply only R1's Get/Set settings projection and its test module.
  The old Park/Wake/Fork/Rewind no-op arms are not restored.

Code and generated declarations otherwise merged without conflict. The
ec5 merged baseline completed its narrow checks and whole gate before runtime
work; its result is recorded below. The later accessor merge is part of the
current changed package. R1's submitted source/record remain unchanged in their own branch.

## Staging boundary to implement

The entry module is cfg(test) until wiring after the shared required_protocol
accessor and Quality B's method table. Keep its ordinary typed interface and
real actor behavior; do not suppress dead code, widen visibility merely to
quiet a lint, introduce a feature flag, or build a second engine for tests.
Cross-crate file/store ownership and the existing settings actor must remain
single owners while the service is staged. Record the precise dependency
boundary before changing code.

Required evidence remains ADR 0098's: cancellation/drop during actual save and
post-save cleanup, Off generation and pending cleanup before later On, fresh
stored-grant and policy/manual-stop checks after awaited load/ready, own-run
cleanup only, clean-stop and killed-daemon fake-provider recovery, bounded
launch concurrency and no prompt for excluded or manually stopped sessions.
The raw legacy setter test seeds a nondefault policy through DataDir and
checks the actual ordinary Admin reply bytes; it adds no recovery RPC field
or special dispatch path.

## Status

The ec5 merge baseline passed source/gate review; its exact source and results
are recorded below. The later accessor merge and in-progress file/owned-lane
changes have not completed their changed-tree gate. The startup service is
not installed. No new client/CLI setting, UI or real-provider execution is
approved. Wiring still waits for Quality B and a separate review.

## Verified ec5 merge baseline

Source `a1d73d11542f2b0b2e2aabb3ab73585cdad53761` ran as
`sessionextras-20261115T034024Z-a1d73d11`; all 18 gate steps returned zero.
Raw workspace totals: 114 summaries, 2,180 passed, 0 failed, 12 existing
ignored. TypeScript had 3 passes and zero regenerated file differences; web
unit 436, browser E2E 63 and forge fixture 4 passed. All Clippy variants,
format/size/boundaries, fresh page/binaries and prebuilds passed. The changed
proto, node and CLI crates actually compiled. One-minute load was 7.32 at
start and 10.18 at end; workspace tests 12.43 to 6.47.

The remote input is the immutable commit. A post-run manifest independently
matched 1,863 selected Git blobs and current files, SHA-256
`695cf30da42b25707053c9fc4d78412d8bed0337ff15bef502e507d8e05500a8`.
There was no local pre-run manifest for this merge; the proof must not be
described as pre/post equality. Selection excludes documentation, native
code, local configuration, dependencies and output.

This is a merged-baseline result, with no R2 implementation. A newer announced
head containing the shared accessor arrived while the gate ran; it is taken
at the next clean boundary. Quality B remains a wiring precondition.

## File and staging decisions before implementation

Keep the legacy DataDir settings APIs and their test expectations. Add a
store-owned saved-record representation for recovery policy with shared
private decode/encode/coverage helpers; do not embed a wire patch with serde
flatten, because that wire patch will later gain the same recovery key. The
node's ordinary legacy request updates only its named fields and preserves
the saved policy. Startup file parsing and atomic/private replacement stay
inside the store. Preserve the head's unreadable/later set-aside rules.

Use one owned settings lane for both ordinary requests and the private staged
policy operation. No recovery input field or test-only RPC is registered. The
node coordinator and its bootstrap/operation mounts are test-staged; lower
store/session APIs follow their actual cross-crate ownership and callers. No
visibility is widened merely to suppress dead code and no second engine is
introduced. Product wiring is reviewed separately after removing staging.

Original local Owner and local Device creators may qualify; a missing or
revoked Device never falls back to Owner. In hard-kill tests, Started is an
ACK, not a durable prefix: establish the committed start/checkpoint before
killing. Test re-execution uses the staged unit binary and real fake-provider
processes; it does not claim regular daemon product-config approval.

## Shared accessor head at the next clean boundary

After the verified ec5 merge, announced `f453ec2e` was merged. It adds the
canonical required_protocol accessor; the complete Quality B table remains a
wiring precondition. No R2 call or input is registered. ARCHITECTURE was the
only conflict: its complete new-head text is retained with R1's value section
appended. README merged automatically and is exactly the new head plus R1's
paragraph; code and generated declarations had no conflict. Head protocol and
schema constants remain unchanged. The next changed-tree whole gate includes
the staged R2 implementation; the earlier ec5 result is not a result for this
newer tree.

## In-progress file and settings ownership implementation

The stored record and legacy wire patch use the same private settings codec,
coverage comparison and atomic writer. Existing file APIs and their tests
remain compatible; the full record preserves the recovery patch when an
ordinary request changes only parking/drivers. Full-record parse diagnostics
redact values but retain the typed source. The existing engine now has one
bounded owned lane, so cancellation of a reply future cannot cancel an
admitted save/apply. Shutdown drains that lane before stopping sessions.
These changes are under narrow verification; Off/generation/intent cleanup
and startup admission still await the staged coordinator.

Node startup reads the saved snapshot off-thread before opening the store.
Session launch preparation was extracted unchanged to node/launch_setup.rs,
a module owning profile/model ingredients and prepared private directories.
This keeps node.rs within its shrinking budget (782 to 738), without a size
allowance increase or behavior/test expectation change in that constructor.

## File/owned-lane narrow checks

The saved-record/file tests pass (10 tests, including the five retained legacy
cases), and Store all-target Clippy passes. Initial node checks caught the new
legacy fixture using the wrong construction for the existing empty settings
request, and the enlarged start function exceeding its existing function
budget. The request is now constructed with its existing empty braces; secure
gateway assembly is a named helper, with no raised size/lint allowance. Node
all-target Clippy now passes, and all six new cancellation, capacity, drain,
policy-preservation and actual raw legacy Get/Set tests pass; no startup recovery
or whole-package pass follows from these narrow results.

The per-commit narrow checks and one final package gate follow the current
package workflow. The complete runtime remains staged until wiring is allowed;
file parsing, saved precedence and the cancellation-safe settings owner are
shared by ordinary requests rather than duplicated for staged tests.

The node check used its actual product and test targets; the tests compiled
the changed Store, Session and Node crates. The saved-file filter passed
10 tests and the settings unit filter passed 6, with no failed or ignored
cases in either selection. These are narrow results for this ownership step,
not a full root/web gate or the complete recovery contract. Original failed
node checks remain separate; no lint allowance, test deadline or assertion was
weakened. No head test, wire call/field, schema migration, client command or
native screen changed. New source/test files and the private temporary
directory were owned by the normal development account; no permission change
or live-service action was required.

A later migration must consume the announced head carrying the existing
latest-schema test correction, rather than quietly changing head assertions.
The current schema and protocol constants remain the head's.

## Ordered journal and passive projection (in progress)

The existing bounded persistence lane is being extended to typed Event and
Checkpoint entries. Internal checkpoint types belong to proto's shared type
layer because Session and Store both need them; they add no RPC or public
event. A record-only runtime still produces event entries only. The node
writer ends a batch at the first barrier, commits preceding events and the
projection change in one transaction, then acknowledges. Person stop/input
admission shares a short gate, with channel capacity reserved before it.
The actor's shutdown capture precedes its first provider signal.

Store owns the passive per-session active-turn/generation/eligibility and
pending/consumed-intent projection, with delete cascade and indexed startup
order. The migration is appended after the head's table through the named
RESTART_SCHEMA fact. Existing rows start ineligible; a transcript scan never
invents activity. Lower Store APIs prepare candidates before Interrupted
normalization and retention; the startup service mounts are still pending.

The current head's ordinary append path also coalesces attention wakes after
commit. Checkpoint batches must preserve that behavior: Effects::appended is
shared, and a new real-store regression checks that a committed TurnEnded
still wakes observers even when its delayed Arm checkpoint is stale.

The two original Session record collectors receive only the authorized type
adaptation and fail explicitly on an unexpected checkpoint. Store rollback
fixtures gain only the new migration's table drops before their existing SQL:
approvals::an_old_approval_survives_current_migrations_and_new_ones_keep_their_set;
devices::a_version_1_database_gains_the_devices_table;
log::sessions_from_schema_3_have_no_origin;
log::sessions_from_schema_2_read_as_the_owners_on_the_default;
log::a_sessions_model_is_stored_and_older_ones_have_none;
marks::sessions_that_ended_before_schema_7_are_archived.
Their existing assertions, order, counts, statements and deadlines stay.
The labels latest-version correction is the isolated identical head-owner
fix. The inbox rollback-position helper must come from its announced landing.

These changes are under narrow compilation and tests, not a journal/runtime
or package pass. The settings generation/Off cleanup and coordinator mounts,
fresh authority admission, process restart/kill tests and wiring remain owed.

Creation now writes the session row and its explicit eligibility in the same
writer savepoint. This avoids a second-write failure leaving an orphan
Starting row. The ordinary internal create seam keeps its signature for the
existing tests; unattended, agent and forwarded service paths select the
manual-on-restart variant from the authenticated principal and the existing
unattended creation argument. There is no synthetic authority fallback. A
new trigger-failure test checks rollback and a later successful create with
the same id. A schema-local test seeds real legacy Running history before
the new migration, then checks that preparation does not invent active or
pending projection while preserving that original event. Neither test has
completed verification yet.

The four changed crates' all-target Clippy check now passes on the journal
source (proto, session, store and node). The additional Node models fixture
collector required the same record-only conversion as the two Session test
collectors; that exact single-line adaptation was authorized and made, with
no other model-test edit. The new schema fixture checks actual SQL row counts
and splits setup from migration assertions; no lint suppression was added.
Runtime tests have not completed yet.

The newly announced node head includes title-origin storage and the upstream
labels/inbox fixture correction. It will be merged at the next clean boundary
before the package gate, and the recovery schema fact will follow that head's
newest named constant. No unknown input field or protocol number is registered
by these lower journal APIs.

## Journal candidate before the Quality B merge

The changed crates' all-target Clippy passed (2m58 actual compilation after
the build wait). Store's complete new restart integration binary passed all
15 cases (1.14s tests after 1m01 compile), including actual atomic-create
rollback and stale-checkpoint attention notification. These are lower-layer
results, not startup-service or whole-package approval. A later source audit
named the new actor-barrier test timeout without changing its value; final
merged-source narrow verification includes that naming change.

The queued proto/Store library selection was canceled before execution after
the build-line switch. It returned 143 and supplies no test result. The schema
legacy-row unit test, generation boundary tests and Session runtime tests are
still owed on the merged tree. No failed or canceled run is relabeled green.

The current kernel candidate is committed to establish a clean merge boundary.
Next: consume announced Quality B head, preserve its title-origin migration
and fixture fixes, point RESTART_SCHEMA to its newest fact plus one, adapt
only this track's callers to Option-returning since(), and run remote narrow
checks. The recovery service/Off cleanup/fresh grant admissions and process
restart tests still follow; no new request field or CLI is registered yet.

## Quality B merge before continued filling

The coherent journal candidate is `d42fd02f`. Announced node `29eebb8b` is
merged at that clean boundary, bringing Quality B, the title-origin migration
and upstream labels/inbox fixture fixes. The head's protocol remains its
named constant; the recovery feature remains Option None. New field rules
can now use the canonical Option-returning since() boundary. The only current
caller adaptation is this track's ordinary raw-settings fixture requiring
Some for assigned existing calls; it never substitutes a protocol for None.

Conflict resolution:

- node.rs: retain the head's setup module, remove the duplicate launch_setup
  extraction and preserve the shared settings opening/drain. Extract only
  the unchanged runtime binding to node/runtime.rs to honor the smaller head
  budget; the record shrinks again.
- session/handle.rs: retain the head's title module and exports, add the
  checkpoint/stop interfaces beside them.
- session/lib.rs: retain TitleUpdate and every head export, add only the
  journal/stop/prepared-resume interfaces.
- schema/migrations.rs: retain every head entry and SQL byte for byte; append
  only recovery after title origin. The sole recovery schema fact now lives
  beside the head fact and derives from it, so its predecessor stays private.
- ARCHITECTURE: retain the complete head text, append only the R2 sections;
  the R1 section already in the head is not duplicated. README has the full
  head plus the ordinary shared-settings paragraph.
- rust-size-allow: retain all head reductions and lower node.rs from its head
  record to its actual smaller count. No allowance is raised.

Independent byte checks preserve the head setup, migration runner, labels
fixture and canonical generated TypeScript. Removing only the explicitly
authorized collectors and recovery rollback-table prefixes reproduces the
head's seven affected test files exactly. No other assertions or deadlines
changed. Generated TS retains its three canonical whitespace warnings; it is
not hand-edited. Merged-source narrow and whole-package evidence are pending.


## Current Quality B filling

The node bootstrap reads the same validated settings snapshot before store
normalization. The runtime uses the journal lane for actual checkpoints;
startup takes at most Recovery::MAX_ATTEMPTS independent attempts. An attempt
consumes its intent before dispatch and holds normal session claims and an
operation arbiter through all admitted effects. Fresh Devices grants and the
shared policy/stop generation are checked separately at launch and after
provider load before the Start-only prompt enqueue. Manual takeover never
shares cleanup authority with an older boot attempt.

Both existing settings request fields are registered only through the shared
recursive Call requirement; there is no added method or assigned protocol.
Existing Get/Set response projection remains per peer. Ordinary legacy
parking/drivers mutations preserve the full saved policy. Explicit client
reads do not infer capability from an absent decoded default; CLI reads report
unsupported separately. A requested mixed mutation is refused as one typed
call before dispatch if recovery's number is unassigned.

Component tests of Off/save cancellation intentionally call the same Settings
engine, not a test-only server route. They cannot stand in for a successful
new-field RPC while assignment is pending. The real raw legacy setter regression
continues to exercise server encoding. Clean-stop, killed-process, real paired
creator grant changes and held provider load tests belong to the final package
run. No native/page code or real provider is used.


## Final runtime verification

The complete combined source `23cf77eb` passed all 18 Linux root/web steps:
2450 workspace tests, four TypeScript tests with no declaration difference,
464 web unit tests, 68 browser tests and four forge tests; zero failures and
12 existing workspace ignored tests. All three Clippy modes, formatting,
size/boundary checks and required builds passed with actual changed-crate
compilation. The exact scope, earlier failed runs and sampled load are
recorded in wp-n5-restart.md. Pre/post selected sources matched their Git
blobs; the selection is not a full-environment attestation.

The actual legacy Get/Set byte projection, clean stop and real node SIGKILL,
settings write/cleanup caller loss, partial failures, fresh grants after held
load, Off/person-stop admission and same-run-only cleanup regressions all
passed in that whole run. The repaired committed-turn observers and rollback
fixtures are exercised there. No deadline/assertion was weakened, no test
was newly ignored and no failure exception was needed.

Independent source review, protocol/schema assignment and combined integration
remain distinct obligations. Option None still refuses explicit new-field
RPC/client/CLI requests. There is no native/page, real-provider, other-OS or
live-service certification, permission change or deployment in this package.
