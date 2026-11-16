# Provider-child observations and native Codex history: receiving checkpoint

This is the Node/driver portion of A4. ADR0090 owns parentage, observation identity,
resource limits and the pinned Paseo references; ADR0053 owns the one history
intake and resume path. Native child panes, parent-resident admission and draft
seeding are separate consumers, not established by this root checkpoint.

[Decision Log]
- Purpose: expose real native child observations without turning provider ids
  into managed SessionIds, and let Codex list its own native conversations through
  the existing agent.history method.
- Existing assets: AgentEvent/EventRecord/EventSeq and their protocol projection,
  the existing canonical driver translations, bounded parent log reads, profile
  resolution and session.create resume. The common prompt-free Codex probe is
  reused rather than adding a second runner or provider-file scanner.
- Choice: retain typed ProviderChild observations in the parent log; project
  bounded read-only descriptors and child activity with parent sequence cursors.
  The parent conversation/status/search/inbox/web projections ignore the wrapper
  while advancing the same cursor. Pending provider read rows stay behind
  SUBAGENT_RELATIONS=None until the integrator assigns the feature.
- Limits: descriptors128/2MiB serialized, tracked ids2048/1MiB text, nesting8,
  scan4096 records/8MiB/5seconds, returned items1..200, four non-queuing reads and
  bounded read batches. Partial/missing/unsupported outcomes are explicit; there
  is no eviction-induced replay, synthetic timestamp or invented first sequence.
- Codex history: initialize/initialized/thread-list only, validated opaque ids,
  absolute supplied cwd, genuine activity times and bounded metadata/cursor.
  Account-scoped import mapping remains the existing authoritative node path.
  No conversation is started, loaded, signed in or modified by the listing.
- Tradeoffs: only observed retained parent history is projected; this does not
  provide an unlimited provider transcript store or managed child lifecycle
  rights. Native thread-list title/preview metadata is not first/last prompt
  preview parity. Shared process-settlement cleanup and quota retention have a
  separate owner; normal fake-process success is not uncertainty/cleanup proof.

## Exact whole checkpoint

Corrected86b01391 passed all18 steps in ordinary run
wp-transfer-history-current-20261116T011941Z-86b01391: Rust2743/0/12, web466/0,
browser80/0, forge4/0, strict feature checks and TypeScript freshness/diff0. All
18 raw step hashes were checked against their immutable receipts. The prior
cc7374c1 whole failure at the missing parent-web provider-child switch is retained;
its corrected isolation regression now passes. The canceled queued cd5 whole is
not test evidence. Portable fake-history fixtures replace Unix-only paths without
weakening validation, assertions or phase budgets; no Windows execution follows.

After that gate completed, announced d5bfde87 was merged cleanly as2708f86a
(empty remerge diff, no manual source resolutions). Exact2708 then passed strict
all-target checks for Agent/Proto/Node/Session/Client/Rpc/Web and TypeScript5/0,
with zero generated difference. Actual Node provider-child projection tests11/0/0
also passed, including first-record deadline/size boundaries, sparse cursors,
sibling isolation, retention/saturation and sticky omitted metadata. Their binary
preparation and cargo steps all passed; Node compiled freshly.

The whole result is exact86b, not a second whole result for2708. The announced
ee8e3450 head arrived during these immutable receiving checks and is not consumed
by this checkpoint. V3 final source review and integrator receiving remain owed.
The independently reviewed SQL scalar and the later detach invalidation leaf are
separate changes; this checkpoint does not silently import their newer ancestry.

## Qualification

No actual account/provider, provider-file scan, credential/configuration change,
service/permission mutation, physical UI, assigned-positive pending wire path,
native child renderer, resident authority or complete A4 approval is claimed.
The common Codex process cleanup/retained-quota producer is still separately
owned and must remain explicit in receiving review; it is not established by the
successful fake transport runs. Automatic import open and real prompt previews
remain separate native/history follow-ups.
