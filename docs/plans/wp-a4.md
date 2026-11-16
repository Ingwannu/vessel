# Agent handoff: reference mapping and scope decision

Status: analysis complete; product implementation blocked on scope.

## Intended result

The initial request was a session-menu action that changes the agent during
a conversation, with a handoff and an agent/model picker. Before adding that
action, distinguish three operations with different preservation guarantees:
moving an account, starting a new agent with text context, and replacing the
provider of the existing session. They are not interchangeable.

## Paseo 0.15.4 source findings

Paths below are relative to the Paseo source tree; these are source inspection
results, not runtime evidence.

| Source | Observed behavior |
| --- | --- |
| `packages/app/src/composer/submission/writer.ts:66-72` | `handoffCreatedAgentMessageSubmission` delegates to the local session store. |
| `packages/app/src/composer/draft/create-flow.ts:203-241` | After `createRequest` returns a new agent id, the draft's first user message is passed to that helper. |
| `packages/app/src/stores/session-store.ts:1188-1227` | The helper updates the new agent's local stream head/tail maps. It does not issue a provider-switch request. |
| `packages/app/src/types/stream.ts:881-891` | The message is upserted into the stream tail with incoming presentation and handoff matching. |
| `packages/app/src/composer/draft/workspace-tab.tsx:118-193,468-487` | Draft submission builds a provider/model configuration and creates an agent with the initial message, workspace and draft idempotency key. |
| `packages/server/src/server/session.ts:4351-4378,4485-4516` | The server handles creation and invokes `createAgentCommand`. |
| `packages/server/src/server/agent/create-agent/create.ts:156-214` | A new agent is created, then its initial prompt is started. |

The word `handoff` in the supplied app references describes preserving the
draft message's presentation when creation completes. It does not describe
changing the provider of an existing agent. The client-side idempotent initial
prompt dispatch has not been traced beyond these call sites.

The reference inventory also describes a separate fork-to-draft behavior:
text context becomes a `chat_history` attachment on a new draft. That is a
different flow from the supplied optimistic-message helpers. Its full source
and failure semantics must be read before adopting it for this package.

## Existing Vessel assets and call mapping

The app base is `1d1f11df`. The relevant protocol types were also inspected at
node commit `242076b5`; that does not imply the app base contains every other
method present on the node line.

| Call | Preserves | Can select | Constraint and source |
| --- | --- | --- | --- |
| `session.move_profile` (protocol 5) | Session identity and the harness's native conversation | Another account profile of the same known harness | `crates/vessel-proto/src/profile.rs:105-124`; `crates/vessel-node/src/profiles/moving.rs:44-78` rejects terminal sessions, added agents and a target with a different harness. |
| `session.create` (protocol 2) | A native conversation only when the selected harness can resume the supplied native id | Harness, model, profile, workspace and options | `crates/vessel-proto/src/session.rs:38-63,78-114`; no source-session or portable context-handoff field. A native conversation id is not a cross-provider transcript. |
| `session.fork` (protocol 14) | The source harness conversation, optionally at a completed turn | Source session and optional turn anchor | `crates/vessel-proto/src/forking.rs:25-37`; `crates/vessel-node/src/service/forking.rs:35-44,70-127` copies the source specification and accepts no target harness/model. |

The native link already provides `session::Act::MoveProfile`, `Act::Fork`,
and `Act::SetOption` in `spikes/native/link/src/session.rs`. The move dispatch
at lines 301-304 sends `force = false`, so a UI must retain a busy refusal
rather than silently cancel the running turn. Model changes offered by the
current agent already belong to the composer's running-session options.

## Decision log

- Purpose and intent: implement a real missing user flow with a name that
  accurately states which conversation and agent identity are retained.
- Existing implementation and constraints: account movement, native forking,
  new-session creation and option changes already exist. None of the three
  inspected calls replaces the harness of an existing session with portable
  context preservation.
- Alternatives considered: extend the existing account-move UI; create a new
  agent seeded by an explicit briefing; adopt Paseo's separate fork-to-draft
  flow after reading its implementation; add a cross-agent node operation.
- Selected approach: defer the product choice to scope clarification. Do not
  register an invented wire call or relabel account migration as cross-agent
  handoff.
- Why: the supplied Paseo handoff references implement a different operation.
  An arbitrary replacement would add unrequested behavior and leave context,
  permissions, cancellation and failure semantics undefined.
- Advantages, costs and effects: avoids a misleading control and preserves
  current behavior; implementation waits for an explicit package definition.
- Existing-asset decision: **modify existing assets** if the package becomes
  account movement; reuse create/options and attachment infrastructure if it
  becomes fork-to-draft. A new cross-agent replacement contract requires a
  separate proposed ADR and node ownership. No external dependency or API is
  needed to establish this distinction.

## Conditional structure after the decision

If the desired result is an account move, reuse the native session action
boundary, with pure eligibility/selection rules in `view/src/handoff/` and
kit picker/dialog composition in `app/src/areas/handoff/`. Use typed profile
identities and retain node error results. No new method is necessary.

If the desired result is a new chat seeded by a briefing, first read the
reference fork-context producer and consumer. Reuse creation, attachments and
the draft picker. Pure context and lifecycle rules belong in `view`; bounded
typed commands/results belong in `link`; `app` composes existing kit controls.
Define source lifecycle, context truncation, mode inheritance and retry
behavior in the proposed ADR before implementing this deliberate difference.
The UI must disclose that it creates a new chat.

No new dependency edges, store schema, protocol numbers, production types,
fake wire methods or controls have been added. Shared shell/link files remain
untouched. A speculative production skeleton would prematurely choose one
of these conflicting meanings, so only this analysis is committed now.

## Validation and resume point

Done: read the assigned briefs and source references; map the three calls;
identify existing native actions. No product code, tests, fleets or captures
have run. Documentation-only fast checks are recorded in the task's external
evidence folder; compile gates and visual review remain unverified.

Next: obtain the scope decision, then receive and merge the centrally
validated node/app base. Build the agreed types/state, area, link boundary
and behavioral tests using existing calls. Run narrow checks per commit,
then one whole native gate and one review for the completed package.
