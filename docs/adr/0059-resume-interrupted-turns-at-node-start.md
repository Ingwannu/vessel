# 0059: Resume interrupted turns when the node starts

Date: 2026-11-14. Status: proposed contract; no implementation approval.

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
including when an older caller changes parking/drivers. Reuse PromptText's
16 KiB UTF-8 bound, nonblank check and control-character rules; preserve
accepted spaces, tabs and line endings verbatim. This is a deliberate bound
beyond the reference server's unrestricted string. Never log the prompt in
settings diagnostics or Debug; the ordinary submitted turn contains it.

No new RPC or scope. Get remains Read, change remains Admin, including
ordinary fleet forwarding guards. Add a default-false
`GetNodeSettings.include_restart_recovery` request flag: an explicit recovery
read requires the feature's protocol at every decoding hop, preventing an
old forwarding node from silently stripping the new reply field. A change
naming the nested patch, even an empty one, has the same requirement. An
absent reply field decodes as disabled/default prompt for compatibility,
but is not evidence that an older node supports the setting: the client/CLI
reports unsupported when the explicit read is unavailable. Parking/driver
reads and changes keep their existing minimum versions. Regenerate TypeScript.
On this branch the additive version is provisionally 20 after row 1's 19;
the node integrator owns final renumbering. The projection needs one schema
migration (provisionally 15 after 14); it is not inferred from retained log
text. Existing rows migrate inactive, with no invented interrupted turn.

CLI: `vessel node settings --resume-on-startup on|off` and independently
`--resume-prompt TEXT`. A read shows both effective values. Reuse existing
saved-settings precedence, serialized save/apply, typed client calls and
current-target selection. No new startup flag or environment override.

Settings are read once before opening/recovering the store, then the same
validated snapshot is attached to the runtime. An unreadable settings file
uses the existing diagnostic/fallback policy, whose recovery default is
off. A failed settings-file write changes neither effective policy nor
intent. Once an off value has been durably saved, it immediately invalidates
this boot's remaining recovery dispatches and consumes pending intents
before acknowledging the change. If that post-save store operation fails,
off remains saved/in force and the call reports Internal, explicitly saying
that recovery is off but intent cleanup failed; it must not claim nothing
changed. A later on change first finishes that cleanup, or is refused.
Re-enabling does not reconstruct an invalidated boot batch. Editing the
prompt affects subsequent admitted attempts, never an already admitted
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
current nonrevoked grant, as the existing schedule runner does, and require
the ordinary ResumeSession and PromptSession permissions. Mint a fresh
agent credential bounded by that principal; never reuse an ended agent
secret or elevate a device to Owner. Use the normal profile/route and
conversation ownership checks. A removed/revoked creator, missing folder,
changed route, unsupported conversation resume or unavailable provider
settles that attempt as failed, with no replacement conversation.

The store owns a bounded-size projection per session: active turn identity
and its start timestamp, parked state, recovery eligibility, stop/admission
generation and optional pending/consumed intent. Projection updates accompany normal log appends;
retention of raw tool events cannot erase an active turn's identity. Delete
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
creator grant, profile and folder at that boundary. A manual resume that
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
  disable then enable never revives consumed work; multibyte 16 KiB/control
  and nonblank bounds; save failure and post-save cleanup failure semantics.
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
