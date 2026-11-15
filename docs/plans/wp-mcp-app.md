# Orchestration settings (W18, ADR 0065)

The coordinator assigned both sides of F2, including tracker 594, to W18.
W15 owns onboarding and consumes this owner's typed tasks/events; its wizard
choices remain deferred until Continue. App base: 9e18cf0db, with the owned
pending node implementation merged for compilation only. No readiness is
inferred from that merge; MCP_PARITY is None and runtime calls must refuse.

[Decision Log]
- Purpose: port the host Skills install/update/uninstall/picker and tools switch.
- Existing assets: kit settings cards/switches/dialogs, settings Overview route,
  prepared Fluent words, typed node MCP/skills contracts and bounded link tasks.
- Alternative: duplicate configuration in app prefs or reuse onboarding's draft
  as persisted state. Rejected because actual state belongs to the chosen node.
- Choice: pure view state, typed owner tasks/events in link, existing kit drawing.
  The target includes book key plus returned NodeId; consumer, Visit and attempt
  correlation reject stale A-to-B-to-A answers. All calls have deadlines and a
  fixed concurrency bound. Raw node diagnostics never become screen text.
- Consequences: unsupported/disconnected machines cannot mutate. Timeout is an
  unconfirmed result, not proof of rollback. Focus reads actual state again.

Reference: Paseo packages/app/src/agent-skills/{index,use-agent-skills,
selection-sheet,legacy-migration}.tsx and screens/settings/paseo-tools-card.tsx.
The survey .area6-parts/A.md is an index; implementation files are the source.

The view skeleton reuses proto SkillName/Selection/Snapshot and McpSettings.
Picker drafts seed only on open; leaving all selects all currently available
names. First Save carries no stale removal approval. The exact fresh names
are confirmed and retried once; a second changed plan is an inline failure.
Uninstall confirms whole folders including user additions. Maintenance confirms
only non-delete ops, sorted add then update/name. Controls are disabled while
working; no selected skills disables install but leaves the gear available.

Next: in-memory fake owner tests, bounded actual link adapter (pending gate),
Fluent EN/KO, host rows and dialogs, focus/identity/rejection tests. One whole
native gate and the required private captures at package close; root gate is
separate. No cargo/rustc executes locally under the coordinator's VM-only rule.

The link now owns bounded tasks (eight, including answer sends), one named
15-second deadline and exact welcome NodeId preflight. The Node trait fake
proves unsupported/changed targets cause zero effects. The real adapter uses
the pending MCP_PARITY and existing client methods; it never invents a wire
call or changes provider policy while toggling inject. Error text is reduced
to typed codes at the edge. Tokio test-util is enabled only in link's existing
dev dependency for paused-time deadline tests; no new dependency is added.

Existing idle-release fixtures explicitly use mcp=None after merging the
additive settings field. Their parking values and assertions are unchanged.
The controller keeps its previous node-settings call; the new orchestration
owner patches only mcp.inject, so neither owner erases the other's setting.

The host Overview now renders Tools and Skills through kit cards/switches,
with EN/KO Fluent messages. Disconnected tools are hidden; skills explain
unavailability. Unsupported capabilities are never invoked. Each drawn
button/dismiss handler captures its original Scope before dispatch, rejecting
a stale opener as well as stale replies. One adaptive dialog is reused for
picker/removal confirmation, with captured return-focus dismissal; a pending
save cannot close it. Maintenance excludes deletes; confirmation names remain
data rows rather than formatted peer diagnostics. Controls select the shared
Pointer/Touch sizing at the app edge. Native accessibility retains the kit's
known lack of a focusable/labelled button tree; no new renderer is introduced.

The shared owner Event is registered in shell dispatch. W15 must attach its
onboarding consumer to the unclaimed Consumer::Onboarding answer; no active
onboarding caller is imported or claimed wired in this tree. Existing idle
settings stay on their prior path; no copied settings storage is added.
