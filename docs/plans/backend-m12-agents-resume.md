# Native Agents: resume after the redraw slice

## Start here

Read the current strict-worker, app-track and overnight briefs before work.
Work remains on the Agents/Models track; the completed redraw slice does not
mean the feature table is complete. Do not treat source review, a passed
gate or a representative capture set as overall visual approval.

## Committed and reviewed

- `a07b4e5`, corrected by `89eb681`: readiness, short typed states, complete
  bilingual sentences, attributed machine diagnostics and typed sign-in help;
  accepted by the coordinator.
- `2d01c28`: ProviderRow rules and fixtures; source reviewed, not a screen pass.
- `70bcfd5`: shared section trailing action and custom row frame corrections;
  source findings resolved, long-title/action first-use review still pending.
- `6416709`: exact red-300 provider-error token; source clear, visual pending.
- `bd08a65`: shared neutral skeleton; source clear. Its original gate did
  not prove elapsed native timing or actual OS reduced-motion preference.
- `d359c072`: Models configured-profile use and refresh correlation, rules
  only; V1 passed it to the coordinator. No Models screen or link was added.
- `b81b7cf88a419ebd07e64639ca8b0b9dd276c7ca`: reviewed source-only integration
  compatibility correction. It depends on G3's existing AgentsHub and is
  not a standalone kit control cherry-pick for a branch lacking that hub.
- `0f46b7d8cbc929757b013b81d0ba854a63d579b3`: the Agents list/detail redraw submitted to V1; disposition
  pending unless a later review explicitly records otherwise.

## What the redraw contains

The provider list follows Paseo `screens/settings/providers-section.tsx`
(`ProviderRow`, `StatusIndicator`, initial-loading rows), with shared
`SettingsSection`/`styles/settings.ts` furniture. The title line contains
chevron, mark, name, separator and one typed status. Only known sign-out
changes its authentication status; other sign-in facts stay in details.
Positive known model counts appear on available expanded rows. Compact rows
drop status words, counts and error paragraphs exactly as the reference.

Details follow the shared BackHeader/ScreenHeader/CompactHeader structure,
with facts in settings rows and a full-width horizontal command surface.
Version/program copies keep independent field identity. The first actual
390-point capture found a clipped Program copy target; compact controls now
cap to their content width, with an actual-window bounds/click regression.

The generic app slot is `page::DetailHeader { title, back }`, where `back`
is a caller-owned UI-thread `Rc` callback. `page::layout` accepts one optional
slot; ordinary compact navigation delegates to the same shared kit renderer.
E2's Machines details can consume this slot after review. There is no second
kit header or area-specific navigation policy in the frame.

Defaults still use the existing `keep_choices` store. The retry action names
what it does: reload the agent's options. Complete failure sentences use
the actual translated action label.

## Verification and evidence boundaries

The frozen corrected tree passed native `cargo fmt --all -- --check`,
whole-workspace Clippy with warnings denied, normal application build,
and 492 runtime tests plus two document tests. The detached gate ended
`G3_EXIT=0` at 2026-11-13 21:03:14 UTC. The changed application was actually
checked and compiled; all 23 frozen source hashes matched before commit.

The earlier integrated compatibility gate passed 491 tests and two document
tests. The corrected compact-width source has its own separate gate and
manifest; that earlier result is not substituted for its verification.

The reviewed capture family contains a list and one real added-agent detail
for EN/KO, dark/light, 1280x900 and 390x844: sixteen captures, each personally
opened against the named source. Earlier rejected-layout captures and
pre-correction compact captures are excluded from this evidence family.
Binary/source/image hashes and exact dimensions are in private handoff
metadata. Real options probes provide 16 Claude Code models and two added
agent models; counts are never inferred from an unread option list.

The compact full launch command copied exactly, 461 bytes. After correction,
the actual compact Program button copied its full 56-byte path. Changing
the default model through the UI saved preferences as UID/GID 1000, mode
0600 under child umask 002. No service-account ownership changes were made.

The production skeleton test sets GPUI's actual App reduced-motion flag and
asserts static opacity, solid furniture and no scheduled frames. This is
not an OS-preference test; the OS-to-App bridge remains the lead's work.
Elapsed pulse/loader phase captures remain a separate verification item.

All own fake fleet nodes, test app and virtual display were stopped by
verified PID/executable identity after captures. Private evidence is kept
for review; do not reuse stale PID files as authority to stop a process.

## Pending review and integration

V1 is the first reviewer for this redraw. Source refs, the corrected capture
family, final gate log and immutable source manifest are in the handoff.
The coordinator receives only the explicitly passed scope. In particular:

- E2's current inherited sidebar still has the old four groups. Its planned
  shared frame puts the machine picker and Agents/Projects/Devices/Models/
  Sync together; G3 must not create a local group. E2 has no reviewed frame
  or typed-dot hash in this session.
- K's fitted official mark is not reviewed for this consumer yet. Keep Bot
  until its immutable settings-row recommendation passes review. Do not
  insert a literal 36-high mark into the 23/24-high content band.
- G2's shared ActivityIndicator profile is source clear but not integrated
  here; current legacy spinner is not claimed to match its 750ms profile.
  The prepared small/loading Outline button, ProviderRow open trigger and
  PackagePlus glyph have no reviewed hashes. No area duplicates are allowed.
- Shared header requests Light 300, but the lead's new Light-face integration
  is still pending on this branch. Style constants alone do not prove face
  selection. Sized button/dot and first-consumer control phase audits remain.
- The wire has no agent enable/disable call, so no dead switch is drawn.
  Removal does have a node call: its absent menu is unfinished UI, not a
  missing node capability.

The current node cannot ordinarily produce installation Failed/Unknown rows
for listed definitions: `readiness::installed` falls through to Found when
no handshake is retained; Unknown needs no discovery and no program. The
fake failed handshake is a sign-in failure and correctly stays Available in
the list. The coordinator assigned this gap, with unreachable too-old/new
states, to N3. Typed wire-state fixtures remain; never fabricate captures.

## Next authorized work, after resuming

First resolve returned findings on this slice and consume reviewed shared
dependencies at a clean point, then rerun only checks justified by changes.
E2/G2 await the reviewed generic detail-slot hash; K has the provider-row
source for its fitted first/following audit. Track those handoffs explicitly.

Add-by-command, registry and removal UI remain unfinished. Prepared rules
and reference notes are private, uncompiled candidates. Use S2's reviewed
shared search; the single deterministic alphabetical comparator remains G3
owned with no delivered hash, and S2 is waiting on it. Do not add ICU or a
duplicate matcher/comparator. Record accent/tailored-locale differences when
that helper is actually delivered.

Models remains rules-only: configured profile use, no general agent/route
compatibility claim. Link, translated failures, actual reads and screen are
not delivered. No new Models screen was started in the closeout window.

An existing Agents queue-loss boundary also needs a follow-up: the current
fire-and-forget command path can drop a full/closed queue after a view ticket
starts. A checked request entry must settle that exact ticket visibly,
with saturated/closed-channel regression coverage. This was not silently
folded into the frozen redraw gate.

## Resume-only handoff: idle runtime cleanup

After closeout, E2 relayed A2 commit `a3e78dfa`: protocol-12
`node.settings` / `node.set_settings` expose `ParkAfter`. E2 identified
Paseo `screens/settings/idle-runtime-cleanup-card.tsx` as a host > Agents
consumer, making G3 the proposed screen owner; confirm the full contract
and ownership with the coordinator before implementation. E2 is not adding
this card to its current frame slice.

The relayed contract is: switching on sends `Idle` with the existing minute
value or 60 minutes, switching off sends `Never`; `Unknown` displays Custom
and must not be sent. Hide the card on unsupported or `invalid_request`;
an administrator change rejected as `Forbidden` appears as an error below
the row. These are handoff facts, not independently verified source or
runtime results.

No commit was imported, no card registered, and no implementation or gate
started for this notification. Resume by reading A2's full immutable
contract and the named reference, then plan it within the Agents scope.

## Local evidence index

- Final gate: `/tmp/work/m12-g3/screen-redraw-gate-compact.log`.
- Frozen source: `/tmp/work/m12-g3/screen-redraw-compact-frozen/source.json`
  and its adjacent `source.sha256`; base is `b81b7cf`.
- Corrected captures: `/tmp/work/m12-g3/redraw-shots/compact-fixed/`;
  `inspection.json` enumerates the sixteen inspected images, and
  `binary.json` records their executable identity.
- Defaults permissions: `/tmp/work/m12-g3/preferences-permissions.json`.
- Cleanup: `/tmp/work/m12-g3/closeout-cleanup.json`.

This resume document changes no code. The code gate above is not rerun for
this documentation-only closeout, following the overnight resource rule.

## Lead's finishing work (2026-11-13, after `3795020b`)

The lead merged this branch into `backend/m12-spike` (`064804b0`) and closed
the coordinator's four detail-page findings and the marks item in
`53c3c944`; G3 resumes from that head:

1. Fact rows share one trailing edge: the copy button is drawn at every
   width (it was hidden on a pointer's width but kept its room).
2. The program's path is `fact_rows::Value::Path`: its own element of a set
   width (the control's 420 less the copy button; a phone's card width),
   cut in the middle with `fonts::elide_middle` (kit), the whole path in a
   tooltip and in what the copy button copies.
3. The launch command (or an added agent's program and arguments) and the
   installers are rows of their own (`settings::row_with_content`) behind
   the card's divider.
4. A failed refresh is the page's machine alert (`agents::machine_alert`),
   drawn above the list or an agent's details, whichever is open; an
   agent's own check failure stays in its details. Regressions:
   `alert_tests` (refresh → select → failure; a failed check is not the
   machine's alert) and the fact row tests (one trailing edge, a path cut
   to its width that copies whole).
5. The list's rows draw the official marks (`settings_mark`); an added
   agent keeps the generic glyph.

An Agents request the link did not take settles as `Busy` (a full queue)
or `Disconnected` (a stopped link), as the devices page does
(`app/src/unsent.rs`). Captures: the lead's scratch folder `b15` (both
themes, both languages, 1280 and 390, the long added agent, a failed
refresh with details open). The "older node" read-only shape was not
reachable: a protocol 11 node lists agents too.
