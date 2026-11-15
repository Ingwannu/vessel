# Schedules: integration with the published desktop base

## Decision and scope

[Decision Log]
- Purpose and intent: resume Automation on the published desktop base
  `56a7cbd3`, retaining the other areas' working interfaces.
- Existing implementation and constraints: the schedule rules are on this
  branch, while its screens and node adapter remain external preparation.
  Several shared controls were taken by other tracks under different hashes.
- Alternatives considered: replace the published menu with the earlier
  extended renderer, or preserve the published renderer and reapply the
  extensions separately.
- Choice: merge the published base, keep its menu as the behavioural baseline,
  and reapply extensions in independently reviewable control commits.
- Why: the published `menu_right`, surface arrival and highlight motion have
  active callers. A second renderer would give those callers different rules.
- Costs and effects: menu extensions are temporarily absent from this merged
  tree. Their original commits and tests remain in history. No schedule screen
  or new control is claimed delivered by this merge.

## Already on the published line

The original hashes are not ancestors of the published commit; these are
checked implementations on that line, not an assertion of patch identity.

| Earlier work | Published implementation |
| --- | --- |
| Destructive button | `5858e70e`; typed button renderer already present |
| Ghost button | `19bbcc2b`; retain its current renderer and tests |
| Calendar-clock | `882622d8` |
| Archive/git action glyphs | `d1b41e2e` |
| Navigation/transfer glyphs | `a8a664c9` |
| Shared header and full-width border | `3bcb790b`, `2d3e2554`; current header is byte-identical to this branch's |
| Stethoscope | `a7ae4055`, with the published drawing retained |
| Schedule preview and node time zone | Integrated node `187dc847`; no editor consumes them yet |
| Schedules navigation seam | `70814634`; registered slot still empty |

Badge and Alert were also already integrated. The schedule view/editor/history
rules (`f8d5df9`, `dfcba41`), MD sizing, ActivityIndicator and menu extensions
are not delivered by the published line. Neither is any Automation screen.

The newer desktop line has the rename control and its token/geometry fixes
(`d103ab39`). That commit is **not** an ancestor of the published base. The
existing rename implementation is retained here without redevelopment;
the later announced desktop integration must take that line's corrections.

## Conflict resolution

- `kit/src/menu.rs`: take the published file, preserving `menu_right`, the
  single renderer, `popup::appear(QUICK)` and `fill_in`. Remove the earlier
  extension modules from this integration; their restoration is separate.
  Keep only the existing shared sizing alias needed by the labelled-button
  regression. No caller is migrated in this merge.
- `kit/src/assets.rs`: retain the published inventory and its
  `assets/drawing.rs` split. Add the five absent variants: MoreVertical,
  Pause, Play, RotateCw and ActivityIndicator. The existing More is horizontal,
  Refresh has two returning arrows, Spinner is the legacy profile; none is a
  duplicate of the added drawing. Retain every published vector byte for byte.
  Move these five original drawings into the same exhaustive match and retain
  the ActivityIndicator alpha/phase regression.
- `kit/src/lib.rs`, `view/src/lib.rs`: additive module/export union; retain
  the published `menu_right` export and all other areas' registrations.
- Native README: retain both areas' documentation, one shared-header
  paragraph, and describe the menu restoration as pending rather than present.
- Header notes: retain the published canonical-breakpoint integration note.
- Slider file-budget entry: reduce its recorded count to the actual smaller
  size required by the existing shared rotation delegation; no budget increase.

## Verification and resume point

Local noncompiling checks pass: native `cargo fmt --all --check`, file budget
(524 files, fifteen still to divide), crate boundaries (four crates, seven owned
external dependencies, two name rules) and token checks (no new literal entry).
The asset audit independently removes the five added arms and reproduces the
published drawing file byte for byte; all 89 variants have unique registrations.
The own-change whitespace check against the published base passes. The full
merge diff also contains inherited generated-wire whitespace; it is not edited.
A whole native compile/test gate is
owed on the designated build machine; it is explicitly not run on the
saturated local machine. This merge carries no new UI captures or approval.
The earlier partial menu evidence and unresolved focus observation remain
recorded in the resume note and do not prove this integrated menu.

The later `f6674d0e` whole native gate includes this merged tree and passed
1,060 ordinary tests plus two doc tests, zero failures and one inherited ignore.
The reviewer's split/import pass is restricted to its seven-path delta; the
merge conflicts themselves are not implied reviewed by that disposition.

Next: the behaviour-neutral split and typed restoration are now committed
separately; follow their notes for exact gates and pending runtime evidence.
The schedules list follows, using the current Paseo
`packages/app/src/screens/schedules-screen.tsx`,
`components/schedules/schedules-table.tsx` and `schedule-row.tsx` as the
reference. The editor, runs and saved prompts are separate later slices.
