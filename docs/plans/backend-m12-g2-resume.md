# G2 resume: schedules and the shared controls

Resume on `backend/m12-app-schedules`, in this track's existing worktree. Read the strict/common/app/overnight briefs and current kit ledger before changing anything. Finish the current menu runtime validation; do not install the prepared next controls or start another merge in this session.

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

The first renewed probe build failed because shaping was called on App's TextSystem instead of the platform/window shaping interface. Its failure logs/patch were retained with the `failed-first-font-proof-` prefix. The corrected probe uses the SAME platform text-system instance as its Application. At this checkpoint the corrected build is in progress; do not use an older binary or an older successful capture-build record if it fails.

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

The lead confirmed that its existing titlebar PanelLeft replaces the desktop SidebarMenuToggle inside Paseo's MenuHeader. Desktop screen_header gets no duplicate toggle; compact keeps the callback to the lead's sidebar action. Chrome/drag/navigation policy stays with the lead. The sidebar schedules slot exists. The lead subsequently supplied `f4dfbb1bd774b48998b2f3f08882c79f453e1086`: `sidebar::nav_row(id, label: &'static str, leading: Option<Glyph>, active, tokens) -> Stateful<Div>`, with legacy row delegating None. V1 read its one-file source diff; no gate/runtime evidence was supplied for that commit. It is NOT consumed here because the stop boundary forbids another merge/slice. On resume, consume the reviewed/gated integration, then delegate only the registered schedules slot to the area's call of nav_row with CalendarClock and its wired route handler. Existing native seam is14 in both layouts and retains legacy hover paint; Paseo SidebarHeaderRow compact is pointer14/touch18 and has hover foreground changes. Full sidebar parity remains S2/lead's later stage, not a pass from this seam. S2 has no competing renderer. Do not draw another G2 area nav row or edit sidebar generally. A temporary checking-route patch remains allowed outside the repository until the proper entry lands.

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

N3's reviewed preview/timezone source is `f81798c3`, but integration renumbers it to protocol13 because another node line owns12. Wait for the merged client base and derive availability from request/Method since, never a hardcoded version. Use node preview times only for the edited rule; reuse its returned cadence/interval anchor on save. Saved next_due remains node-authored. The current integrated base has the earlier node line. Webhook calls are still awaited; draw no placeholder. Creation credentials must never be logged/captured, even in a fake run.

Saved-prompt contract preparation is `saved-prompts-contract-plan.md`: list(None) means all scopes; project list includes machine prompts; keep opened revision/project/session spec, exact text and read-only insertion. Composer-card drawing belongs to the lead: return selected text and give its caller seam; do not edit the card. GOAT's partial reference is ComposerStashMenu, not a named create/edit library; do not invent a claimed match. Follow the plan's actual contract and shared kit surfaces.

## Process and resource boundary

Every build/check/clippy/test/run uses `/tmp/work/m12-tools/cargo-slot`, four jobs, one command at a time. Do not cancel/restart a queued build or run another to bypass its wait. Source stays frozen until the command ends. Ignored target configuration and fonts stay in place.

Earlier owned fake-fleet/display PIDs were verified and stopped, and no process from this track's isolated fake-fleet build remained. Any subsequently started capture process/display must also be stopped by its verified PID before finishing. The final runtime disposition and cleanup check will be appended after the in-progress probe ends. Local work runs as the ordinary development account; no production service, deployment or permission change is part of this slice.
