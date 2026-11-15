# N5: restart recovery, node facet of lifecycle feature 28

## Scope and status

The node facet of tracker row 28 belongs to N5; W3 retains its app facet.
The package includes the accepted R1 policy values and hidden reply projection,
R2's typed journal, passive projection, node-owned settings and startup runtime.
The implementation is built and its final Linux root/web package gate passed
on `23cf77eb`. Independent source review and integration remain pending.
Existing R1 and narrow source verdicts stay separate from that final execution. No native or page implementation belongs
to this node package.

## Specification and existing owners

Survey: `parts/A-lifecycle.md`, Resume / refresh / reload, feature 28's restart
recovery and shutdown capture, with its lifecycle / settings / provider
connections. The actual reference is Paseo's restart-recovery/index.ts and
index.test.ts, config.ts::resolveRestartRecoveryConfig and bootstrap's
freeze/capture/close/flush order. Keep the reference's disabled default,
verbatim configured prompt, clear-before-dispatch and no implicit unarchive.
The full accepted contract and deliberate resource/authority differences are
in ADR 0098; runtime wiring follows the now-available common boundary and remains unverified until the package gate.

[Decision Log]
- Purpose: continue interrupted committed work without reviving work a person
  stopped, replacing a conversation or minting remote authority.
- Existing owners: Store's atomic settings codec and single SQLite writer;
  Node's shared Settings engine, launch/guard/Devices paths and resume claims;
  Session's ordinary actor, bounded event persistence and input controls.
- Choice: extend those owners with typed internal checkpoints, explicit
  creation eligibility, bounded boot ownership and fresh admission fences.
  Do not add a second settings engine, event log, RPC proxy or authority path.
- Alternatives: scanning a retained transcript loses active-turn identity;
  another checkpoint channel loses stop/event ordering; cancellable RPC
  ownership can leave a saved Off unapplied; cached grants outlive revocation.
- Consequences: explicit commit/admission barriers and an append-only migration
  cost work; intent is best effort and consumed before dispatch, never retried
  automatically after uncertain mutation. No new external dependency.

## Completion boundary

The evolving implementation/result record is backend-m12-recovery-engine.md.
Its accepted file/owned-lane source is only one part. Remaining gates include
actual Off cleanup/caller loss, fresh grants after gated load, exact run-owned
cleanup, clean-stop and kill recovery with fake providers, excluded/manual
work and bounded concurrency. New inputs use the canonical required_protocol
accessor and one pending method-table entry form only after Quality B. The
feature protocol stays Option None until assigned by integration. Use the
announced node head's latest migration fact plus one; keep head SQL/tests.

At package close: one immutable changed-tree root/web/CLI/TS gate, one complete
source/evidence submission and a screen-facing typed call contract. There is
no real-provider, live-preview, UI or other-platform claim from fake-node
checks. Several-model and delivery work follows this package and Quality B.

Quality B's announced node head is now taken at the clean kernel boundary.
Its Option-returning method admission and pending row form replace the earlier
wait; the next filling step may add the recovery field rules/client/CLI, with
the feature number still unassigned. This does not change runtime approval.

## Product wiring candidate

After the announced Quality B boundary, R2 now connects the passive journal
and saved policy to the ordinary node runtime. Both explicit settings fields
contribute the single unassigned feature constant through the existing
recursive call requirement. The ordinary Get/Set reply projection stays in
place. Positive new-field RPC use remains unavailable until integration
assigns the protocol; component policy tests must not be described as RPC
success. No local test assignment or alternate dispatcher is introduced.

The first merged-kernel narrow remote check exposed an append function size
lint after the incoming title projection was combined with recovery. Reading
its existing durable prefix is extracted without changing append SQL or the
projection. The first local product check was cancelled while still queued,
before any Cargo invocation, to use the managed remote lane. Product wiring,
settings failure tests and provider admission tests are not yet validated.

The first runtime narrow check found missing imports, a moved test patch and
an internal resume signature mismatch. The existing three-argument resume
entry is retained; its outer owned operation holds recovery admission through
the actual inner completion. The saved-file compatibility view now decodes
the registered optional patch too, with the same verbatim policy and redacted
parse diagnostics. These adjustments and the expanded tests remain candidates
until the next immutable narrow run and final package gate.

The final baseline now consumes the announced shared package base 3,
`6c782cb6`, including the separately reviewed tab-session receipt race fix.
The automatic merge preserved the incoming script/port and fake-provider plan
behavior; the recovery load gate remains present alongside it. The previous
Node/CLI all-target narrow Clippy passed before this merge. Two further
regressions cover an already-admitted turn surviving Off, and a held fresh
grant fencing actual device revocation; their execution is still owed.


## Final package base and source preparation

The submission branch starts from announced node `ba45a6e3`. Its one method
table, request contract module, node setup, identity types, privacy correction
and existing tests are retained. Recovery adds no new method rows: only the
two existing settings methods gain optional field requirements in the canonical
recursive call requirement. The feature number remains unassigned.

The earlier frozen candidate's recovery-focused node test run passed 27/0.
It is a narrow result on the previous base, not this package's final gate.
An earlier run retained 22/5: all failures were the pairing fixture's missing
advertised route before any recovery phase. The corrected fixture uses the
same synthetic advertised route and actual ephemeral dial override as the
existing secure tests; no pairing guard or deadline changed.

The journal API's record-only collector adaptations are limited to receive
and extract the typed JournalEntry in the existing fixtures. The private-dir
fixture keeps its existing stop_failure module and adds the new stopping
module; its earlier assertions and deadlines remain. Recovery's ADR is now
0098 to keep the brand's 0059 unique. Candidate revisions and failed logs are
retained independently; final source verification is still required.

## Final gate preparation

The final node all-target Clippy passed after the last fixture result was
asserted. The earlier recovery-focused 27/0 run remains a narrow result on
the previous base. The package gate now covers the announced node base and
the actual regenerated TypeScript. Existing sessions without the creation-time
eligibility projection remain manual even after later turns; neither a log
scan nor a manual resume invents that provenance. This deliberate migration
limit is stated in the ADR and user documentation.

## Combined node runtime before the package gate

The package now also consumes announced node head `d432ef0f`. Its runner-issued
ProcessLifetime and AgentSetup system_prompt field remain intact. The session
parts constructor combines that existing process observation with the recovery
journal; neither is reconstructed from a PID or a second persistence path.
Architecture additions from both branches are retained. No incoming test
expectation or deadline is changed.

The previous whole-gate submission was cancelled by the build machine's
restart while it was still queued. No gate step ran; this is neither a code
failure nor a pass. The source stayed identical to its recorded input hashes.
The combined immutable tree is submitted once under the infrastructure retry
ruling. The earlier narrow results retain their original source scope.

## First complete execution and fixture repairs

The first complete combined run passed formatting, size/boundary checks, all
three Clippy modes, TypeScript freshness, web lint/typecheck, 464 web unit
tests, 67 browser tests and four forge tests. Workspace tests reported
2405 passed, four failed and 12 existing ignored cases. This is a failed
whole gate, not a partial pass promoted to success.

The recovery case subscribed while startup could still expose the stored
ended log. A listening node intentionally does not wait for the boot batch.
The test now waits on the existing boot-completion watch and starts after
the actual committed first-turn event, preventing an old info snapshot from
being mistaken for the recovered turn. The same Off, busy-input, history
count and subsequent-restart assertions remain.

Three older migration cases retained recovery tables while rolling back to
an earlier schema. Their fixtures now undo only the appended recovery
migration before the original inbox/title-origin rollback. Existing SQL,
assertions and deadlines are retained; production migrations are unchanged.

At the subsequent clean boundary the package also consumed announced node
`ede20014`. Its plugin values, event protocol accessor and CLI/web changes
remain intact. Recovery has no AgentEvent since() caller to adapt. The
repaired immutable tree still requires narrow checks and its clean whole
gate before package approval.

## Exact committed-turn setup for recovery tests

The narrow repair check found one unnecessary SessionId qualification in a
new helper; it is removed. A further source audit found that the common
active-turn fixture must also distinguish a live broadcast from the writer's
committed prefix. Its ordinary RPC still starts the turn, but the local test
then waits on the existing persistence acknowledgement for that exact event
sequence and returns the stored session snapshot. No synthetic input, mark,
checkpoint or provider message is used to force progress.

The crash test's child owns the same real writer observation. A typed request
on its private stdin pipe asks only to observe that prefix; the child reports
the actual stored snapshot before the parent sends SIGKILL. This neither adds
a product RPC nor bypasses authority for an effect. The fake provider and all
existing recovery assertions/deadlines retain their behavior. These changes
are still candidates until the next immutable test and package gate.

## Selected tests passed; final combined source

The repaired node/store test suite passed 723 tests, with zero failures and
two existing ignored tests. It compiled the changed node tests and passed
the real-node kill case, Off after admission, both inbox rollback cases and
the title-origin rollback. No warning was emitted in that execution. Its
pre/post selected source manifests matched. This remains a selected-crate
result; it is not a whole-workspace or UI pass.

The final source also consumes announced node `82ae7c91`: staged method rows,
early stop-signal registration, the shared CLI output seam, provider-child
identity values and the comparison tests' later-record observation. Recovery
adds no method row; its field requirement still uses the single unassigned
feature constant. The existing input ownership and checkpoint shutdown are
retained around the head's guard/signal changes.

README's conflict kept the incoming output-seam description and updated
recovery's earlier value-only summary to this package's current scope and
ADR 0098. No other head document or test assertion was changed by that merge.
The combined source still awaits its required clean full package gate and
independent source review.


## Final package verification, 2026-11-15

The immutable combined source `23cf77eb`, based on announced node
`82ae7c91`, passed all 18 required root/web steps with exit code zero. The
workspace test log has 117 summaries: 2450 passed, zero failed and 12 existing
ignored tests. TypeScript generation ran four tests and left no declaration
difference; web unit tests passed 464, browser tests 68 and forge tests four.
Formatting, file budgets, dependency boundaries, all three Clippy modes,
required binary builds, web lint/typecheck and font validation passed. The
changed crates were actually rebuilt; no compiler warning was emitted.

The whole run used one managed build slot, with test fakes and the node
prebuilt in the same environment, eight build jobs and eight test threads.
Its first step began at 14:39:02 UTC and its last ended at 15:15:57 UTC.
The recorded one-minute load was 40.30 at the start and 37.48 at the end;
the workspace test step recorded 44.25 to 42.59. These are sampled loads,
not claims about unsampled minima or the cause of earlier failures.

Before and after the frozen run, all 2031 selected source files matched both
each other and their Git blobs. The selection covers root manifests,
crates, scripts, web, AGENTS, README, ARCHITECTURE and QUALITY. It excludes
native, other documents, ignored configuration, dependencies and build output;
it is not an attestation of the entire environment. The TypeScript artifact
hashes and its three declaration files matched the committed source.

The earlier whole run remains recorded as failed (2405 passed, four failed),
and the intermediate qualification lint remains a failed check. All four
repaired cases passed in the final whole run. No deadline or assertion was
weakened and no additional test was ignored. The final run needed no failure
exception or isolated replay.

This result certifies the executed Linux checks, not independent approval,
a real provider, other operating systems or a native/page flow. The feature
protocol stays unassigned; explicit recovery settings requests still return
UnassignedProtocol. Integration owns assignment and the combined-line gate.
The node implementation remains built and awaiting source review; the app
facet remains with its owner. No live service, permission or deployment was
changed, and no repository push was made.

## Review finding: admitted prompt and a person's kill

Independent source review identified a stop-admission defect after the green
execution: the recovery arbiter cancels an attempt but waits for its delivery
reply before allowing an ordinary KillSession through. That reply can still
be unresolved after actual prompt enqueue. Keeping its owner and capacity is
necessary; delaying the person's durable stop until its reply is not.

The forcing regression holds the fake provider's actual prompt response and,
separately, the real committed Arm acknowledgement. ACP accepts at send rather
than at provider completion, so holding the latter is necessary to retain the
actual session delivery reply. The writer remains free to commit suppression;
the observer forwards only the actual commit result and releases it on drop.
The test uses authenticated KillSession, checks retained attempt capacity and
no third launch until real settlement, then checks replacement survival and no
reconstructed restart intent. It is prepared for execution against the
unchanged recovery admission before the repair; no reproduction is claimed yet.
