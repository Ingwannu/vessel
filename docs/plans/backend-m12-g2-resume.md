# G2 resume: schedules and the shared controls

Resume on `backend/m12-app-schedules`, in this track's existing worktree. Read the strict/common/app briefs and current kit ledger before changing anything. The published desktop base `56a7cbd3` was merged in `29a9834b`; see `backend-m12-g2-published-integration.md` for conflict decisions and verification limits. The schedules list is the next feature slice. Earlier stop-only instructions below are historical, not a ban on the resumed work.

## Resumed published-base checkpoint

- `29a9834b` merged published `56a7cbd3`. The newer desktop line, including
  `d103ab39`, is intentionally not consumed until its announced gate. Keep the
  existing rename control rather than redeveloping it.
- `fe86e090` split the published menu without behavioural changes;
  `f6674d0e` removed unused imports. The reviewer passed that isolated seven-path
  source delta and its A13 whole native gate: 1,060 ordinary plus two doc-test
  passes, zero failures, one inherited ignored test. Run
  `schedules-20261115T022318Z-f6674d0e`, evidence under
  `/tmp/work/m12-g2/remote-menu-split-20261115/`. Merge-conflict and UI approval
  are not implied.
- `ef91b954` restores typed leading/feedback/muted/dismissal/measured bounds
  on the published renderer, retaining menu_right and motion. Remote native
  Clippy passed; its whole run `schedules-20261115T024307Z-ef91b954` is still
  pending. No ProviderRow/modal/passive/button-content control is added.
- Current GUI artifacts are uncommitted external probes from that immutable
  restoration. The corrected probe has a fresh caption and no duplicate
  desktop toggle. Old pilot images remain excluded; the new matrix still
  needs personal inspection and review. Shipping fonts and weights are
  unchanged. Font selection is tested through actual same-platform glyph
  rasters against proof-only Light aliases with identical outline/weight/
  metrics tables; this is not Windows or whole-UI proof.
- New list labels and feedback go into Fluent. Accepted cadence/error/state/
  action words stay in their existing table; the translation track owns their
  later separate migration. The external list draft must not copy them.
- The source's partial-host error banner has no equivalent current Alert
  geometry/presentation. Ownership lookup found no active extension; wait for
  assignment, with no area override or parallel notice renderer.

The original list remains outside the tree and unverified. Automation screens
remain undelivered. Next is the schedules list, followed by its editor, runs
and saved prompts in separate feature slices. Current source preparation and
owned-process details are kept in `/tmp/work/m12-g2/STATE.md`.

## Committed and reviewed

| Commits | Scope and disposition |
| --- | --- |
| `f8d5df9`, `dfcba41` | Schedule view/editor/history rules and renderer-neutral fixtures; accepted. The screens are not delivered by these commits. |
| `4d40a5c` | Destructive button; accepted. |
| `8f01148`, `f3c4e99` | Calendar-clock and six action glyphs; V1 passed their limited visual scopes. The six-glyph wide images are actually1278×800, requested1280; evidence was corrected without changing image bytes. |
| `1698aab`, `bcd22bd`, `cee2c70`, `4d2415d`, `e3d7851`, `2d3ecfc` | Ghost, menu leading/pending, single-line labels, Git trigger, own SVG ink, Close/KeepOpen. Source reviewed; runtime/first-use approval remains pending. Preserve G1's separately accepted `menu_right` seam through the same renderer when integrating. |
| `60816ee`, `af648e4`, `d45c1ada`, `29f6137c` | Header/transfer glyphs, shared header, full-width desktop border correction, Schedule hitSlop8 and input propagation correction. Source reviewed; runtime approval pending. |
| `7006bfb5`, `dcf48666`, `8fef6122`, `22918fa8`, `0ddc3122` | Sized MD buttons, static host rename, menu Success, Stethoscope, documentation correction. Whole403 gate/source reviewed; provisional reuse, not UI approval. |
| `35b40e40`, `743af0c8` | Additive RNWeb ActivityIndicator profile and explicit menu pending16. Whole407 gate/source independently reconstructed and reviewed. Native alpha/phase/reduced-motion proof remains separate. |
| `7f56176cfd4309359c7290b67dbd7510252f2fd9` | Merge of reviewed desktop base `fc71dcf7`. V1 source-clear: native471 tests +2 doctests; canonical breakpoint44 and forwarding aliases, nine embedded faces, both sides' registrations/guides preserved. Imported root/client/node/web/release trees match the lead; no local root/web test claim. |
| `ab1983e4aff134bbc642ed145b7ac93ba2fd16f4` | Selectable muted menu presentation and explicit ordinary14/16 glyph choice. V1 source-clear/provisional consumer reuse:474 tests +2 doctests, fmt and Clippy green. Runtime remains pending. |

Latest gate evidence is under `/tmp/work/m12-g2/`: `menu-presentation-{fmt,clippy,tests}.log`, `menu-presentation-source.json` and `.sha256`. The350-file aggregate is `827544882fa5c611bfc8bc722aabdfcac180ecac922e61234b30b3f8e9af2449`. `native-manifest.py menu-presentation-source check` reproduces the selection and per-file sizes/hashes after the temporary example is removed. Actual Kit/App Checking and Compiling are present in the logs. Do not repeat this unchanged gate.

V1 is the first reviewer; the coordinator receives its source/UI dispositions. Source-clear is not visual approval. All old invisible-trigger, pre-hit/border and pre-strict list images are excluded from current evidence; see the probe folders' exclusion notes. The list screen itself has no accepted commit.

## Current runtime probe

`/tmp/work/m12-g2/control-probe-final.rs`, `build-control-probe.py`, `control_qa.py`, `capture-controls.py`, `capture-hit.py`, and `capture-keep-open.py` are uncommitted external probes. The build wrapper temporarily installs one example and removes it even on failure. Its patch/artifact/source hashes must accompany captures; nothing that supplies fixture data or opens a checking route belongs in an installable build.

The first renewed probe build failed because shaping was called on App's TextSystem instead of the platform/window shaping interface. Its failure logs/patch were retained with the `failed-first-font-proof-` prefix. The corrected probe uses the SAME platform text-system instance as its Application and built successfully. Its external observational frame counter emitted one unused-return-value warning; no production lint was weakened and no unchanged gate was rerun. Exact build/probe/patch/binary records and24 personally inspected PNGs are in `/tmp/work/m12-g2/shots/shared-controls/`, with the limited scope and exclusions in its README. The artifact is from product source350; the temporary example was removed.

Actual Light300 identity is still unverified. The lead's Light faces are embedded and were supplied through the ignored font workflow; do not fetch fonts again or change300 to400. Fontconfig metadata lists Light-only aliases, but fontdb prefers typographic family names, so an alias-based runtime comparison may be unavailable. The probe logs failure explicitly and keeps other input/geometry evidence separate. Requested weight or converter fixtures alone are not selected-face proof. Preserve this distinction when reporting to V1 and the lead.

The probe includes state/choice witnesses, measured inner trigger bounds, both ordinary glyph sizes, muted actions, pending/success, sizing/rename controls and an ActivityIndicator page. Validate actual counter changes before claiming a stable prepaint count proves no frames. Capture1280 and390, both themes/languages, personally inspect every image, and record actual PNG dimensions. Expected values in capture notes are not proof without the observed witness.

## Original list work: preserved outside the repository

The original49 files were byte-verified and set aside before the lead merge:

- `/tmp/work/m12-g2/list-wip-before-lead.json`: per-file hash/size/mode and original base.
- `list-wip-before-lead.patch`: tracked changes; SHA256 `f1c391006ee387217ea82ebb88eeb14d710379b6e4672444ddbb140bb52c9308`.
- `list-wip-before-lead-untracked.tar`: the27 new files. V1 independently reconstructed the old407-gate source from these and its immutable base.
- `list-original-files/`: verified original reconstruction.
- `list-rebased-files/`: later UNCOMPILED preparation against the merged base, with age/menu/recovery/drawing changes. Do not rerun `prepare-list-rebase.py` over it without backing it up; that would erase the later preparation.

The prepared rebase preserves device/localization registrations and the newer sidebar signature. Its Cargo.lock conflict is intentionally unresolved: retain the current lock and resolve the actual added dependencies at installation. Restore source with current timestamps using plain writes/copies; never preserve old mtimes. Check actual changed-crate compilation. Never use cargo clean or the shared stash stack.

Preparation includes current-clock relative ages and seven exact Paseo time fixtures, no dispatch count/progress parts, a pending cadence action that does not reverse when a poll races its reply, shared header/large36 loader/Ghost retry, and safe read retries. It is not built, reviewed or captured. Finish its rules/tests and reconcile old test expectations before treating it as a slice. The native link already enables jiff `tz-system`; a brief contrary finding was withdrawn after checking the verified original manifest. No timezone fix is claimed. Actual platform discovery still needs fake-fleet evidence.

Remaining list work includes the prepared row/menu integration, per-row anchors, current-clock metadata callers, recovery tests, strict reference comparison, actual fleet shapes/actions and the complete capture matrix. Reference files are Paseo `screens/schedules-screen.tsx`, `components/schedules/schedules-table.tsx`, `schedule-row.tsx`, `utils/schedule-format{,.test}.ts`, `utils/time{,.test}.ts`, and the aggregate/body-state tests. Rendering wraps long identities/reflows trailing controls on phones; record necessary differences explicitly. Only wired actions appear; editor/history arrive in their own later slices.

The lead confirmed that its existing titlebar PanelLeft replaces the desktop SidebarMenuToggle inside Paseo's MenuHeader. Desktop screen_header gets no duplicate toggle; compact keeps the callback to the lead's sidebar action. Chrome/drag/navigation policy stays with the lead. The sidebar schedules slot exists. The lead's current seam is `708146348934f814fc2123b4ec8164a006e3a929`, superseding `f4dfbb1bd774b48998b2f3f08882c79f453e1086`: `sidebar::nav_row(id, label: &'static str, leading: Option<Glyph>, active, touch, tokens) -> Stateful<Div>`, with legacy row delegating None, and `schedules(shell, tokens, touch, cx)`. It now uses pointer14/touch18, matching the compact DensityIcon sm step. V1 reviewed source/gate and limited None-region evidence; Some-glyph, phone and full-sidebar parity are not approved. It is NOT consumed here because the stop boundary forbids another merge/slice. On resume, consume the reviewed/gated integration, then delegate only the registered slot to the area's nav_row call with CalendarClock, received touch and wired route handler. Legacy hover/text/padding/radius/shortcut differences remain S2/lead's later stage. S2 has no competing renderer. Do not draw another G2 area nav row or edit sidebar generally. A temporary checking-route patch remains allowed outside the repository until the proper entry lands.

## Next controls: prepared only, not approved or installed

All scripts below are under `/tmp/work/m12-g2/`. They are preparation, not finished uncommitted product work. Recheck against the current tree and ledger; do not blindly execute all scripts or overwrite another track's changes.

| Waiting consumer | Preparation and exact scope |
| --- | --- |
| G1 | `ab1983e4` is now source-clear: Presentation::Muted and LeadingSize::Regular. Remaining requests are modal menu priority3 with origin-focus restoration/outside consumption (`add-menu-layer.py`, `menu-layer-tests.rs`) and passive geometry/space (`add-trigger-space.py`). Preserve default screen priority1 and menu_right; actual modal overlap/hit/Escape/outside/focus/clip and Some/None file-header counts/height are owed. |
| G3 | `add-button-content.py` plus `button-content-types.rs`: one renderer, explicit Small28/32 px12 versus existing MD32/44, Outline and Ready/Loading content; loading is small20 and inherently disabled. Legacy signatures/label layout stay. `add-provider-trigger.py`: existing horizontal Glyph::More14, visual32/radius8/hit48 in both layouts, explicit open presentation, hover/open surface2 and pressed surface3, no invented disabled opacity. PackagePlus20 is also owed. |
| E2 | `add-gauge.py`: Usage uses Gauge via actual SidebarHostSectionButton, explicit stroke1.5, pointer16/touch20. Existing rename/Stethoscope source-clear is already available. E2 owns dot_at6/8 and picker SizedDot; do not duplicate it or overwrite its widgets section. |
| Shared glyph consumers | `add-package-plus.py` and `add-gauge.py` are original shared24-grid drawings with registry/render tests, intended as glyph-only scope. PackagePlus effective1.5 comes from LucideProvider; Gauge's actual consumer explicitly supplies1.5. Neither is compiled or visually approved. No duplicate MoreHorizontal vector/alias: Glyph::More already exists. |
| S2, later | Existing width220/open_end/leading14 solves imminent Shortcuts. The fixed32 keys rail is page-only, not a menu API request. Sidebar display preferences later needs232/end, current value/subpages/check/back/Escape/compact sheet and multiselect KeepOpen. API not implemented. |

`next-controls-prepared/` is only a parse-tested scratch composition of some earlier script revisions, not a gated snapshot; Gauge/space/latest modal tests and documentation are not all represented there. All production changes need their own small control-only commits, documented decisions, whole gate, V1 first, then first-use evidence. Start none in this session.

## Feature work after the list

Editor, run history screen/session links, saved-prompt screen/composer selection, recurring prompt session entry and webhooks remain undelivered. Do not call the milestone complete.

The node integrator subsequently supplied `187dc847` on `backend/m12-node`, reporting its whole gate passed. This is the integrated protocol13 preview/timezone dependency, based on N3's reviewed `f81798c3`; it is NOT merged into this track. On resume, derive availability from request/Method since, never a hardcoded version. `schedule.preview` is Read/forwardable, takes cadence, optional UTC after and count1–10 (default5), and returns exactly count instants strictly after after, never a partial list. Reuse the returned cadence/interval anchor on create/update to preserve phase; saved next_due remains node-authored. Typed invalid_request detail distinguishes unknown_zone, unknown_cadence, no_future_run and time_out_of_range, with future reasons decoded as unknown; four occupied previews return unavailable. An older machine gets no preview call: the client refuses it locally, so the editor must display a supported older-machine path without guessed preview times. NodeInfo.timezone is optional machine IANA metadata, absent when unknown/older. The current G2 base still has the earlier node line. Webhook calls remain awaited; draw no placeholder. Creation credentials must never be logged/captured, even in a fake run.

Other closeout dependency notices, also NOT consumed: G3's `0f46b7d8cbc929757b013b81d0ba854a63d579b3` supplies app-side `page::DetailHeader { title, back }` and optional layout slot over this kit renderer, but was submitted to V1 with review pending, not UI approval (reported native492+2 gate). Its resume is `docs/plans/backend-m12-agents-resume.md`, commit `67892a4c`. K's `e8ffeaa2`, `d99a376d`, `d9eda48f` on `backend/m12-logos` are explicitly WIP, not a consumer hash: the last completed test gate failed and the corrected fixture/full gate/new UI evidence remain unverified. Keep the existing agent-mark fallback until V1 passes a consumer hash. K's handoff is `docs/plans/backend-m12-logos.md:335`; its later theme work/API/list has not started.

Saved-prompt contract preparation is `saved-prompts-contract-plan.md`: list(None) means all scopes; project list includes machine prompts; keep opened revision/project/session spec, exact text and read-only insertion. Composer-card drawing belongs to the lead: return selected text and give its caller seam; do not edit the card. GOAT's partial reference is ComposerStashMenu, not a named create/edit library; do not invent a claimed match. Follow the plan's actual contract and shared kit surfaces.

## Process and resource boundary

Every build/check/clippy/test/run uses `/tmp/work/m12-tools/cargo-slot`, four jobs, one command at a time. Do not cancel/restart a queued build or run another to bypass its wait. Source stays frozen until the command ends. Ignored target configuration and fonts stay in place.

Earlier owned fake-fleet/display PIDs were verified and stopped. The renewed capture display and every probe process have also been stopped; no process from this track's isolated fake-fleet or probe executable remained. Local builds/captures ran as UID1000 with new isolated home/runtime directories700. No production service, deployment or existing service-account permission change is part of this slice.


## Final runtime disposition at this stop

The24 images cover normal/danger/plain-muted ordinary14/16 rows and Normal pending/success16 at both widths/themes/languages. Each run observed index3 chosen once by pointer and once by keyboard; pending/success stale Enter produced no extra choice and fresh open-state witnesses retained selected2. All24 original images were personally opened and their dimensions/hashes verified. This remains PARTIAL evidence, not UI approval: muted-danger/disabled combinations, real Git Info/no-write first use, complete trigger hit/neighbor/clip tests, other controls' phases, activity phase/no-frame and Light300 identity are still outstanding.

An observed exit/focus limitation is recorded in every input witness: after pointer-close, keyboard-close and Escape the root F5 callback produced no fresh event; reopening the menu restored it. Stale state was never substituted for a fresh observation. The current close remains state-only; establish parent/root focus restoration and production impact before claiming the retained-keyboard behaviour passes. Do not apply an area close/reopen workaround. This finding needs the next reviewed shared-menu slice, not an untested last-minute source change.

No additional product Rust was changed after `ab1983e4`. All future controls and the original list remain external, unverified preparation as listed above. G1 confirmed that it consumed none of the new stack after the stop notice and that its real first-use evidence is still pending. The current slice's committed source/gate remains available independently of these remaining runtime obligations.

### Typed published-menu gate completed

`ef91b954` now has a passing A13 whole native run:
`schedules-20261115T024307Z-ef91b954`, six zero steps/no skips,
1075 ordinary plus2 documentation tests passed/0 failed/1 inherited ignored.
The driver completed; no replacement run is needed for documentation edits.
V1 review remains pending. New v2 probe evidence and its unresolved after-close
keyboard routing observation are documented in the restoration note. The
partial-host-error Banner geometry still awaits explicit shared-control
assignment: G4 confirmed no equivalent active profile and is frozen on its
own slice. No area override or parallel notice was implemented. Automation
screens remain undelivered; resume with the schedules list, its new Fluent
labels and the separately retained accepted cadence/error tables.

### Clean boundary after the activity alpha correction

- `ef91b954` isolated typed-menu source and A13 gate passed V1. Its UI is held
  for after-close root-key delivery and the formerly incorrect activity alpha;
  V1 has not independently passed the author's88+16 pictures or Light aliases.
- `e2827533` separately corrects ring compositing. Its exact A13 run
  `schedules-20261115T031952Z-e2827533` is green:6 steps/no skip,
  1076 ordinary+2 docs/0 failed/1 inherited ignored. New activity16+menu4
  inspected pictures and their final file manifests are with V1; review pending.
- All probe apps and the owned virtual display are stopped, authority removed.
  No fleet was started for these shared-control probes. The remote driver ended.
- The coordinator now requires the lead's0.0.0.0.19 line at the next clean
  boundary, after this frozen gate/evidence hand-off. Do not continue building
  registrations on the old published0.17 line. Merge, do not rebase. Remake the
  pure menu split and typed restore over the lead's current single renderer,
  preserving every lead-defined seam and policy. Earlier gates are scoped to
  their immutable commits, not proof of the future merged tree.
- LinkError extraction is delegated conditionally on that new base. Recheck
  whether it remains in link/lib.rs and whether a registration seam already
  exists. If needed, move only the unchanged type to its own module with root
  public re-export, exact body hash, unchanged variants/Display/API/actor and a
  lowered measured size record. Gate/review it as move-only; registrations are
  a separate commit. Send the lead those two hashes only in the0.19 hand-over,
  not before. The external0.17 proposal is preparation only, not a patch to apply.
- K's c679 palette producer overlaps the existing theme/palette.rs public
  GREEN_500 used by menu success. Coordinator/lead own a single-colour-module
  integration preserving that fixed meaning and K's Palette API. Do not copy
  the literal or widen visibility without review; do not change a frozen tree.
- The partial-host-error Alert profile remains unassigned, not implemented.
  No area override or parallel renderer is permitted. List view/Fluent/target
  preparation remains external and uncompiled; Automation screens remain0/6.


## Active resume point — 2026-11-15, after base8

- Main branch backend/m12-app-schedules is on announced91407173 via07c8e9145.
  Latest source ce1b0e04c; create/edit sheet and original FieldId/visit guards,
  caches, retained/off-route read-only engines and reference row-menu fixes are
  committed. They have NOT passed current App compilation, package gate or UI.
- Exact715 ordinary run finished ViewClippy101/no tests. Its full batch is
  retained at /tmp/work/m12-g2/f1-foundation-v1-20261115-clippy; own source fixes
  are committed. Generated PlainSchedules resolver length awaits L's checked
  typed64-field chunks (984fd5b1 candidate, NOT adopted/green here).
- Isolated shared picker: backend/m12-shared-picker-lease, worktree
  /tmp/work/m12-g2/picker-lease-producer. Exact range914..a1b1..eceae..99ad57b8.
  A1/ece REDs are preserved. V1 source precheck99 found no remaining blocker;
  adoption still HOLD pending exact requested lib tests. Clippy run
  picker-control-20261115T144446Z-99ad57b8 is0 (ordinary short, priority0).
  Tests picker-control-20261115T150539Z-99ad57b8 are queued, same exact99.
  Driver /tmp/work/m12-g2/run-picker-control-99ad57b8.sh PID110244 (verify actual
  command before touching). Label picker-control is temporary; drop only after
  checked handoff to V1/lead/W2, never while busy. No further priority is granted.
- Picker user-facing per-open admission and weak return focus use the SAME
  engine/geometry, no F1/width/Form dependencies. Evidence scratch contains
  immutable source pins and raw remote logs; final99 packet/outcomes still owed.
- Multiline580 already has V1 source/Kit narrow pass and was handed G3. G3 is
  sole SearchField writer now; G2 must wait for its checked handoff before the
  assigned Body/Code form-text profile. Do not edit shared field files meanwhile.
- L generator, G3 SearchField, W12 actual product delivery/opening are external
  dependencies. DeliveryGuard is still test-only in current base8; historical
  candidates remain unopened, no legacy alias opener or fake proof allowed.
- W11 reaffirmed no announced production provision/launch adapter at this
  checkpoint (combined node49b pending; W3 internal driver still test-only).
  W21 also has no reviewed runtime archive/restore head. Keep isolation hidden,
  use typed ports, After=Keep explicitly skips cleanup, no cwd ownership or
  duplicate provision/receipt/archive path. This is an in-scope remaining gap.
- No new whole/package run until source/dependencies are coherent. F1 tracker
  UI rows remain skeleton. New screen words are generated schedules.ftl; old
  cadence/error tables stay. Typed docs-links schedules destination is None:
  Vessel user docs do not exist yet; it is a known gap, not a parity exception.
- Every cargo/rustc/build/capture compilation is VM-only. A local boundary
  script mistakenly ran cargo metadata (no compile); disclosed to coordinator
  and stopped. Do not repeat any local Cargo-backed quality script. Direct
  rustfmt, source-size/token scans and Git remain available.
- No own fleet, virtual display, app or capture binary is running. No UI captures
  for current F1 exist; no service-account/OS/physical input proof is claimed.


## Current resume — after checked picker and Search baton, 2026-11-15

- Earlier active-resume section is historical. Main merged announced c3429952
  as d24c74099 (1450/0/2 is inherited, NOT F1 combined evidence). Current source
  includes b39 FormGlyph option wiring;14e1 catalog browse/selection separation;
 98a666 selected Search13c integration;35ef Code form control;8c173 raw-cron
  opt-in;59f7 combobox header length correction;5a15 L Shape compatibility.
- The isolated914..99ad57b8 picker is V1 SOURCE/requested Kit PASS: strict0 +
 186/0/0. Source/manifest/raw run packet is /tmp/work/m12-g2/
  picker-admission-99ad57b8-packet; V1 audit /tmp/work/m12-review/
  g2-picker-admission-99ad/audit.json. Final7c4f60f6 docs-only includes actual
  handed-off/retired result. Lead/W2/W5 received ONE selected producer range.
- picker-control label was DROPPED rc0 (logs under /tmp/work/m12-g2/
  retire-picker-control-20261115.*), no reuse/new jobs. No App/consumer UI/native
  Linux focused-field F5/OS/IME proof is implied by the isolated186 result.
- def6 and66735 Kit-only source jobs both cancelled while queued (143, no
  compilation/tests); raw logs retained.66735 was superseded using WORKER
  standing approval after static105-line combobox header correction plus newer
  Code/Search source. Replacement stays ONE ordinary short schedules suite.
- G3 checked Search source2e3/2b999/13c only is adopted preserving canonical
  field/export union. G3 has released field writer. G2 Code now exists as
  FormText::Body/Code +with_source_text on the SAME identified line engine;
  existing constructors/search/multiline/composer defaultBody unchanged. All
  new Code/consumer tests remain authored/unexecuted until next exact check.
- L exact123e60a00 producer is green; supplied Shape compatibility delta applies
  unchanged to our byte-exact base8 generator and all five manifest hashes
  match. Compatibility itself is source-only until OUR combined compile/gate.
  No General/F6/Contract downgrade or schedules.ftl word migration was taken.
- Model browsing now retains one selected catalog and uses the existing one
  options slot independently. Drill/cancel cannot replace full selected spec;
  choose_model requires original current request +offered Model option. Pure
  preservation/stale/initial-view tests are authored, not executed yet.
- F1 still owes provider error/loading/retry source presentation, final combined
  narrow/whole/export/actualENKO390wide both themes/pending/stale UI evidence.
  Historical F5 remains HOLD. No current F1 capture/build is available.
- W11/W3 provision, W21 runtime archive, W12 admitted opening/delivery remain
  unavailable as previously stated; isolation is hidden/in-scope pending, not
  a parity exception. No cwd inference or old alias opener is allowed.
- Only remote gate tool may run Cargo/rustc. Do not run boundary script locally
  (it calls metadata); prior accidental metadata disclosure remains preserved.
