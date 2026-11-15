# 0098: Resume interrupted turns when the node starts

Date: 2026-11-14. Status: accepted contract; implementation split for review.

R1's value/projection boundary is accepted independently. After the announced
Quality B node head, R2's ordinary product wiring is being implemented on this
track. It reuses the canonical call-field requirement and one method table;
the feature constant remains unassigned. Positive new-field RPC/client/CLI
execution awaits integration's assignment, while actual node recovery can be
tested from the saved settings file and through the existing owned engine.
No implementation approval is implied by acceptance of this contract.

## Reference and existing parts

Paseo's `packages/server/src/server/agent/restart-recovery/index.ts` and
`index.test.ts` choose running agents before normalizing stored status,
capture intent before graceful shutdown, clear intent before dispatch, and
resume candidates independently. Archived agents/workspaces, private helpers
and externally owned executions are excluded. Turning recovery off consumes
pending intent; enabling it later does not resurrect that work. Provider
history restores the same conversation. Recovery is best effort, not an
exactly-once continuation or an automatic retry loop.

`packages/server/src/server/config.ts::resolveRestartRecoveryConfig` supplies
the actual defaults: `resumeOnStartup = false`, `resumePrompt = "Resume
working."`. Both are reloadable. The app's entire
`packages/app/src/screens/settings/restart-recovery-card.tsx` shows the host
Agents switch and an Edit action. Editing preserves the exact nonblank text;
switching off does not discard the configured prompt. Bootstrap prepares
candidates before workspace initialization and dispatches after providers
and transports are ready. Shutdown freezes ingress before capturing intent.
The reference starts every candidate concurrently without a numeric bound.

Vessel already has saved, administered `node.settings`, a single store
writer, ordered session persistence, per-session resume claims, runtime
shutdown, and the ordinary resume/Start-only prompt paths. `Running` is
process state, including idle agents; it does not prove an active turn.
The store currently discards parked state and converts every live row to
Interrupted before retention. A normal person's kill ends Exited, but a
crash before that final record commits can leave a live row. These facts
make scanning Interrupted rows or a bounded raw event tail insufficient.

## Decision

Modify/reuse those parts. Add a node-owned recovery coordinator and durable
per-session recovery projection; reuse normal launch, credentials, profile,
model-route, worktree, conversation-load and prompt validation. Do not add a
scheduler definition, provider-history rewrite, comparison resource, remote
prompt proxy or alternate authority. The native/page setting is outside
this slice.

### Settings and compatibility

Add `NodeSettings.restart_recovery: RestartRecovery`, defaulting to disabled
with the reference prompt. Its typed fields are `enabled: bool` and
`prompt: PromptText`. Add optional
`SetNodeSettings.restart_recovery: SetRestartRecovery`, with independently
optional enabled/prompt patches. This retains custom text while disabled,
including when an older caller changes parking/drivers. Reuse `AutomationText::PROMPT_MAX`, PromptText's existing
UTF-8 byte bound, nonblank check and control-character rules; preserve
accepted spaces, tabs and line endings verbatim. This is a deliberate bound
beyond the reference server's unrestricted string. Never log the prompt in
settings diagnostics or Debug; the ordinary submitted turn contains it.

No new RPC or scope. Get remains Read, change remains Admin, including
the existing host-only, non-forwardable settings guard. A member cannot
administer another node's settings; that host's own paired relay connection
retains its ordinary Read/Admin guards. Add a default-false
`GetNodeSettings.include_restart_recovery` request flag: an explicit recovery
read requires the feature's protocol at every decoding hop, preventing an
old forwarding node from silently stripping the new reply field. A change
naming the nested patch, even an empty one, has the same requirement. An
absent reply field decodes as disabled/default prompt for compatibility,
but is not evidence that an older node supports the setting: the client/CLI
reports unsupported when the explicit read is unavailable. Parking/driver
reads and changes keep their existing minimum versions. Regenerate TypeScript.
`RESTART_RECOVERY_PROTOCOL` is one named `Option<ProtocolVersion>` and remains
`None`. Both Get and Set reply projections suppress the field while unassigned,
including inside Forward/Relay/Deliver envelopes. R1 adds no request flag or
patch field and changes no call requirement. Wiring must consume the shared
`required_protocol`/`UnassignedProtocol` boundary and single method table;
there is no local fallback assignment or sentinel. Integration alone assigns
a version after the CLI and native/web host-settings consumers exist.
The staged durable projection will need an appended migration with one named
schema fact after the node head's existing migrations. R1 adds no migration.
Existing rows must migrate inactive, with no invented interrupted turn.
Rows created before this projection have no trusted creation-time eligibility.
They remain under manual control even after a later turn; automatic recovery
starts tracking only new eligible creations/imports. Reconstructing that fact
from a transcript or a current creator would risk resuming an unattended
execution. This conservative migration difference from Paseo is deliberate.

CLI: `vessel node settings --resume-on-startup on|off` and independently
`--resume-prompt TEXT`. A read shows both effective values. Reuse existing
saved-settings precedence, serialized save/apply, typed client calls and
current-target selection. No new startup flag or environment override.

Settings are read once before opening/recovering the store, then the same
validated snapshot is attached to the runtime. An unreadable settings file
uses the existing diagnostic/fallback policy, whose recovery default is
off.

Extend the existing Settings engine with one bounded node-owned operation
lane: one active change and at most eight queued changes, admitted in FIFO
order. Keep the existing saved-settings serialization, merge, validation,
file writer and runtime setters inside that operation; do not build another
settings engine or independently spawn a second file write. A full/closed
lane refuses before admission or any mutation. Once admitted, the node's
tracked operation owns write -> saved/effective apply -> generation
invalidation -> intent cleanup, including its lock/slot and completion
report. The RPC handler only awaits its reply. Dropping/cancelling that
reply future, disconnecting, or `crates/vessel-rpc/src/server/calls.rs`
aborting the handler cannot abort the operation or release its ordering
slot. An abandoned response
does not turn an admitted change into an unsent one or allow a retry.

This closes an existing cancellation gap: spawn_blocking(write_settings)
can outlive the handler that was supposed to update saved/runtime state.
The same operation must await that write's actual outcome and apply a
successful save even with no RPC recipient. Memory/admission are bounded;
there is no claim that a blocking filesystem write can be cancelled by an
RPC deadline. Neither a caller timeout nor node shutdown abandons an
unknown write and admits a later On ahead of it. Shutdown closes admission
and drains tracked settings operations while the runtime/store they use
remain available. Invalid input fails before writing; a failed write
changes neither effective policy nor intent.

For an Off save, a provisional recovery-admission barrier prevents new auto
launch/prompt admission while that write is unresolved. It is not a policy
or intent change: write failure removes the barrier with the old policy
and intents intact. After a successful save, that same owned operation
applies effective Off and advances the boot generation under the recovery
admission arbiter before releasing the barrier, then consumes pending
intents before replying. No later queued change can pass this operation.
If post-save cleanup fails, Off remains saved/in force and the call reports
Internal, explicitly saying recovery is off but intent cleanup failed; it
must not claim nothing changed. Retain that cleanup obligation even when
the reply is lost. A later On must complete it before its own save or be
refused. Re-enabling never reconstructs an invalidated boot batch. Editing
the prompt affects subsequent admitted attempts, never an already admitted
provider request. Off does not terminate a turn already sent.

### Eligibility, durable intent and deliberate exclusions

The candidate belongs to this node and has a committed active structured
turn, identified by its TurnStarted EventSeq, with no matching TurnEnded or
TurnFailed before the shutdown boundary. Starting, idle, parked, exited,
failed, archived, deleted and retention-pruned sessions do not qualify.
Pending permissions/questions during an active turn still qualify: they
restore through the provider's normal conversation load and fresh requests,
not by replaying old answers. Queued prompts alone do not qualify.
This node line has no archived-project/workspace flag; the available
archive boundary is the session's mark. A removed project folder/worktree
fails the ordinary folder/place check, not a fabricated archive flag.

Exclude side-question helpers, agent-created descendants and unattended
schedule-owned executions, corresponding to the reference's private/owned
executions. Save that unattended creation fact explicitly, rather than
guessing from a log string or current options. Exclude forwarded remote
origins: the saved local creator does not retain the remote caller's
current authority, and startup cannot mint a fresh delegated principal.
This is an intentional, visible scope difference from a single Paseo host;
no missing origin grant is replaced with Owner. Locally imported sessions
can qualify after a new local turn, under their local creator's authority.

The original local creator is Owner or a local Device. Re-read the device's
current nonrevoked grant through Devices, as the existing schedule runner
does, and require the ordinary ResumeSession and PromptSession permissions.
The initial claim/preflight lookup is not authorization for a later effect:
Guard::permit sees a cached Principal, and a synthetic startup context has
no authenticated device socket for revocation to close.

At each new launch admission, and independently at prompt admission after
load/ready or any other await, obtain a fresh stored grant and validate its
current revision/revocation epoch. Grant mutation/revocation and these
admissions share an internal device fence: a check followed by an unlocked
await and reuse of the old Principal is insufficient. Releasing that fence
to wait invalidates the authorization permit; acquire/check again before
the next effect. This is an internal extension of Devices' current-grant
path, not a new grant, scope, authenticated socket or permission RPC.

Within the same final admission decision, check that the policy is still
On, there is no pending-Off barrier, the boot and person-stop generations
still match, and this attempt still owns the session/run. Serialize those
checks and the mutation's enqueue/admission with Off/manual-stop
invalidation. Do not hold that arbiter across provider load/ready waits.
Any intervening await before enqueue requires fresh checks. Once that
atomic admission succeeds the particular effect is in flight and may
finish despite a later revoke/Off; it grants no permission for another
launch or the subsequent prompt. Construct a principal only from this
fresh grant, for this one admission, never from a cached phase or Owner
fallback. Mint a fresh agent credential bounded by that principal; never
reuse an ended agent secret. Keep normal profile/route and conversation
ownership checks.

If revocation/reduction, Off or a manual stop wins during load/ready, do
not send the subsequent prompt. A pending attempt's diagnostic/cleanup is
also fenced by its run ownership: stop a process it launched only while
that exact attempt still owns it, and never kill or append into a manually
taken-over run. No fresh replacement is made after a denied phase. A
removed/revoked creator, missing folder, changed route, unsupported resume
or unavailable provider settles that attempt with its ordinary reason,
with no replacement conversation.

The store owns a bounded-size projection per session: active turn identity
and its start timestamp, parked state, recovery eligibility, stop/admission
generation and optional pending/consumed intent. Projection updates
accompany normal log appends; retention of raw tool events cannot erase
an active turn's identity. Delete
cascades remove its projection. Do not parse message text to reconstruct it.

A person stopping a turn/session must persist a suppression checkpoint
before the cancel/kill effect is admitted or acknowledged, including archive
and same-turn repeated cancellation. A cancel naming a stale turn must not
suppress a newer one. Use one ordered session persistence lane with typed
internal checkpoints and commit acknowledgements; these are not public
timeline events. A stop checkpoint advances the admission generation, so a
late old TurnStarted, old delivery acknowledgement or provider drain cannot
re-arm it. Only a newly admitted turn in the subsequent generation may
re-arm recovery. Cover normal send/prompt, queue SendNow/Resume, and explicit
session resume followed by input; merely steering, editing a queue or
unarchiving does not undo a stop. An unsent/refused prompt does not re-arm.
Internal fail-fast kills and the node's shutdown use their distinct causes;
they cannot masquerade as a new person's prompt or stop acknowledgement.

On a graceful node stop, freeze new input/create/recovery admission and
capture each actor's active-turn checkpoint in that same ordered lane
before signalling its provider. Drain commits as today. The checkpoint
defines the stop boundary: final turn cancellation/status records generated
by shutdown cannot erase its saved intent, whereas an earlier person-stop
generation still excludes it. On a kill, committed active-turn projections
supply the corresponding intent. Work not committed before a crash is not
claimed recovered. Startup chooses/normalizes candidates transactionally
before existing Interrupted normalization and retention. Pending intent
protects the candidate from startup retention until consumed; deleted or
archived work is never restored to defeat retention or a person's action.

### Start order, ownership and outcomes

Candidate order is oldest interrupted TurnStarted first, then SessionId as
a deterministic tie break. Read a bounded page of ids from the projection
index, not complete transcripts or an unbounded task list. A single tracked
coordinator has at most two active recovery attempts; no unbounded spawned
tasks or semaphore waiters. One hung provider occupies only its own slot.
The listener becomes usable without waiting for the entire recovery batch.

Each attempt atomically claims and consumes its pending intent before any
launch, while sharing the existing per-session resume claim and an input
admission lease. Recheck policy, generation, archive/delete/retention state,
creator grant, profile and folder at that boundary; launch and later prompt
still require the independent fresh admissions above. A manual resume that
claims an as-yet queued candidate consumes the pending auto intent and
takes over; the worker skips it. Manual normal input waits for an admitted
auto attempt to settle, then follows its existing delivery mode. Manual
cancel/kill invalidates the attempt's generation before its remaining
dispatch, rather than waiting behind a hung provider. If the auto prompt
already reached the provider, cancellation stops that actual turn through
the existing cancellation path. A late worker cannot prompt, fail, or kill
a later manually owned run. Old cursor/input checks remain intact.

Restore the same SessionId, provider conversation, profile, model route,
choices, marks and log. Keep inherited queued prompts held (Ended) during
recovery; they are not substitutes for the resume prompt and are not
automatically released afterwards. Use the normal Start-only request with
one text block holding the configured prompt and notify false. No queue,
steer, interrupt, fresh session, native slash rewriting or synthetic parent
context. The new ordinary TurnStarted persists the configured prompt once;
provider-native history restoration must not duplicate old turns.

Bound an attempt's load/ready/prompt admission to 120 seconds, constrained
also by the existing handshake limit. A timed-out in-flight mutation is
uncertain, not unsent. Keep ownership until its launch/request and any
cleanup settle; never free a slot and create a third potentially live
attempt while an earlier one still owns its launch. Stopping a newly
launched process is allowed only while that same recovery attempt still
owns it, using the existing bounded stop escalation. Shutdown cancels new
phases and joins tracked work; no detached continuation survives the node.

Failures before launch keep the session ended and append a final Failed
status with the ordinary bounded reason, under its claim. A normal launch
failure uses its existing failed run/log. A failed or uncertain prompt after
launch adds a bounded HarnessLog diagnostic through the live actor, with
the reason and whether dispatch may have occurred; do not fabricate a
Failed process status, successful turn or dead-session append while a live
actor owns sequence numbers. No credentials or prompt text in diagnostics.
Thus session.get/read/subscribe and CLI log show the failure using existing
types. A successful attempt is its ordinary restored run plus TurnStarted.

No automatic retry of a failed, timed-out or uncertain attempt in that boot.
Clear before dispatch as in Paseo: a crash between consumption and prompt
admission can lose a continuation. If a resumed new turn was itself
committed and interrupted by another node stop, it can qualify at the next
boot as new active work. This is best effort, not exactly once. Disabling
recovery at startup consumes pending intent even without loading providers;
enabling it later does not start those old sessions.

## Required evidence before implementation approval

Carry the reference tests' broken/stuck/working/idle/initializing cases and
their exact custom prompt `Continue verbatim.\n`, plus its disabled,
archived/private/owned/archived-workspace exclusions mapped above. Add:

- Real fake-agent clean-stop and process-kill restarts: same session and
  provider conversation, exact prompt, original records once, no new session.
- Default off; custom prompt saved while off and retained across toggles;
  disable then enable never revives consumed work; multibyte PromptText-bound/control
  and nonblank bounds; save failure and post-save cleanup failure semantics.
- Gate a real settings write, cancel/drop/disconnect its RPC reply while
  blocked, then release the write: the one node-owned operation still
  applies saved/effective Off, invalidates generation and finishes cleanup.
  Repeat with cancellation during post-save cleanup, failed cleanup and a
  later queued On; no ordering slot is released early, old pending intent
  never fires, and later On succeeds only after cleanup or reports refusal.
  Include queue-full refusal before mutation and shutdown draining a change.
- Active turn vs idle/starting/parked/queued-only/archived/deleted; projection
  survives a tool-heavy retained tail; legacy migration stays inactive.
- Stop/cancel/archive before crash, including crash before final status;
  stale-turn cancel, late old TurnStarted/ack, failed new prompt and a valid
  new Start/queue Resume re-arming exactly the new generation.
- Ordering and two-attempt maximum with a stuck provider, a failed one and
  a working one; listener and unrelated RPCs remain responsive; deadline
  ownership does not silently admit a replacement launch.
- Local Owner and currently authorized Device; revoked/reduced device,
  agent/side/unattended/remote exclusion; missing provider/folder/route and
  unsupported resume; independent success remains intact after failure.
- Use actual fake-provider load/ready gates on a real node: after initial
  authorization and launch, revoke the device or reduce its stored grant,
  or admit Off/manual stop, then release readiness. There is no subsequent
  resume prompt. Check fresh permission before launch too, and the race at
  the final atomic prompt-admission fence. A previously admitted request is
  reported as in flight, not fabricated as unsent. Cleanup may stop only
  its own launched run; a manual takeover survives stale worker completion.
- Manual resume takeover, manual prompt during recovery, stop/off during
  load and during prompt, lost prompt acknowledgement, shutdown/restart
  between intent consumption and dispatch. No automatic mutation replay.
- Read/Admin guards, older peer defaults, new-field refusal through actual
  direct/member/relay hops, generated TypeScript and a real CLI settings
  round trip. Whole required root/web gate after changed implementation.

No new external package or license. Two-way concurrency and deadlines are
deliberate resource bounds beyond the reference's unbounded fanout. The
durable checkpoint path costs a write barrier for a person's stop, rather
than risking revival after acknowledging it. Persistent projections cost
one small row per retained session and a schema migration. Implementation,
test results, screen integration and whole-feature parity are not asserted
by this contract.


### R1 value/projection boundary

`vessel-proto::restart` owns the disabled/reference-prompt defaults and the
independent partial policy value. Prompt validation reuses the existing
`PromptText` parser and `AutomationText::PROMPT_MAX`; it preserves accepted
spaces, tabs and line endings and redacts text from Debug. No new dependency
or duplicate string validator is introduced.

`NodeSettings::for_protocol` strips nondefault recovery values for old or
unassigned peers. The existing reply walker invokes it for both Get and Set,
and recursively traverses forwarding envelopes. Missing/default fields retain
the existing wire shape. A private value-projection helper accepts an assigned
boundary for unit coverage; product projection always reads the single feature
constant. That helper never participates in request admission or runtime policy.

Adding an owned prompt makes `NodeSettings` Clone rather than Copy. The node's
existing settings builder only completes its response literal with the disabled
default. Its file format, save/apply engine and request types are unchanged.
Raw legacy setter evidence with a nondefault saved policy belongs to R2's real
staged engine; a synthetic reply test does not claim that product evidence.


### A person kills an already admitted recovery turn

Prompt enqueue records its exact session run under the same short attempt
arbiter. An authenticated KillSession after that boundary may enter the
existing durable suppression and kill path for that exact run while the
recovery reply is unresolved. Keeping the ordinary operation lease and the
original attempt alive preserves both pools; dropping the reply or releasing
capacity early was rejected. A pre-enqueue kill still cancels preparation and
waits for its actual ownership to settle before ordinary execution. A delayed
post-enqueue kill carries the pinned handle, never a fresh id lookup that could
select a later manual run. Off still only invalidates future admissions and
does not kill the already admitted turn.
