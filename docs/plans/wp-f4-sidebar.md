# F4 sidebar: one presentation over the shared workspace registry

## Ownership and decision

[Decision Log]
- Purpose: reproduce Paseo 0.15.4's sidebar navigation, workspace projection,
  display controls and fixed footer through one native renderer.
- Existing assets: `view::prefs` and `Shell::prefer` persist changes; the kit's
  bounded ScrollViewport, settings furniture, menus, glyphs and shortcut chips
  already draw these controls. Registry identity, membership, status, rename
  and pin are C1's ADR 0082; attention/read state is A5; archive is A2.
- Alternatives: a second local workspace directory, path-derived identity,
  local menu/scroll copies or a second preference writer would disagree with
  those owners and lose data on reconnect.
- Choice: pure `view::sidebar` models presentation, filtering, ordering and
  navigation preferences. The existing preference document stores native
  changes; app modules adapt the registry and draw the shared controls. No
  new crate, dependency, wire method or feature-local node RPC is needed.
- Impact: plugin and registry consumers wait for their owners' typed contract;
  an unavailable source stays explicit rather than being synthesized from cwd.

## Primary source and behavior map

All source paths below are relative to Paseo's `packages/app/src`.

- `sidebar-nav/model.ts` (whole): default header New workspace, History,
  Search, Schedules; footer Usage hidden. Resolve saved order once, skip
  duplicates/unavailable entries while drawing, append unplaced defaults and
  plugins. Editing retains unavailable keys in their prior interleaving.
- `sidebar-nav/use-sidebar-nav-items.ts` (whole): header/footer independent
  preference fields, one setting updater, current resolved groups at edit time.
- `components/sidebar/sidebar-nav-rows.tsx` (whole): selected History/Schedules,
  Search opens command center, New workspace retains supported active context.
  All hidden means no bordered group; compact close button remains possible.
- `screens/settings/sidebar/sidebar-nav-section.tsx` (whole): two settings
  cards, icon/title/current shortcut, move up/down with boundary disabling,
  visibility switch. Fixed footer icons are not reorderable entries.
- `components/left-sidebar.tsx:370–500,561–609,733–794`: fixed header and
  footer, one natural-height scrolling body; footer summary rows then icons
  Add project / Usage / Hosts / spacer / Help / Settings. Usage summary is
  omitted with no report, even if its setting is shown.
- `components/sidebar/display-preferences/model.ts` (whole): transient view
  filtering is separate from saved row appearance, but the renderer reads one
  projection. Selecting an active trailing choice clears it.
- `stores/sidebar-view-store.ts`: Project default, Created default, background
  hidden, host/project/label allowlists; legacy label grouping becomes Project.
- Survey `.area6-parts/C.md` sidebar sections and
  `parts/p1-projects-workspaces.md:20–24` define the remaining projection,
  row metadata/actions, ordering, callouts, width and compact behavior.

## Skeleton, then fill

1. Navigation section/key/visibility/order types with bounded serde, resolution
   and preservation tests; persistent settings use the existing writer.
2. Sidebar settings body and header/footer consumer of that same projection,
   through the existing E2 page slot, with FTL English/Korean labels.
3. Project/status/pinned presentation, filters, metadata and menus consume C1
   registry projection; visible order is also W22b's numeric-jump source.
4. Footer callouts and external actions register their owners' typed hooks.
   Keep the existing fixed scroll boundary from height package `0690faed`.
5. One complete feature gate/review and one final actual-fleet capture matrix.

No placeholder route or clickable action without an implementation is exposed.
Skeleton states are fixtures and typed model states, not claims of live parity.

## Shared seams and deliberate differences

- W11 supplies WorkspaceId, explicit membership, projectKey, title/status/pin
  projections. Its current draft is not consumer-ready. Never infer ownership
  or a cross-host equivalence key from a filesystem path.
- W16 supplies plugin addresses/contributions and rendering/runtime dispatch.
  UI preference keys are retained independently of installed/connected groups;
  an opaque bounded navigation key is not a second PluginId.
- W5 supplies Usage summary and open callback. One footer preference controls
  the optional summary; the fixed Usage icon is distinct and stays available
  when W5's modal is callable. No duplicate usage state or meter.
- W21 owns Sessions/history and workspace archive; W2 owns children; W7 owns
  read/unread and terminal attention; W10b owns label identity/manager; W8 owns
  services/setup state; W19b owns desktop windows, folder opening and changelog.
- E2 supplies the settings frame. Register Sidebar in `settings/sections.rs`,
  not the frame; no extra title or scroll container in the settings body.
- W22b owns command center/keyboard engine after the theme input package.
  Sidebar hints read its existing action registry and overrides.
- Native stores local/sidebar choices in its existing preference document,
  rather than introducing Paseo's separate AsyncStorage stores. Changes still
  replace only their named field. Cross-client sync of these new fields is not
  claimed until the preference transport owner exposes that capability.
- Full row keyboard accessibility is not implied by height repair's minimal
  focus/reveal seam. Additional activation/focus contracts must use the kit.
- The earlier height package preserved native outer/section padding. This F4
  package must compare row metrics to the reference before claiming parity.

## Validation and current disposition

Per commit: changed-crate remote narrow Clippy and focused tests (A14). Final:
one A13 native package gate; actual fake fleet, 1280×900 / 390×844 / 800×600,
dark/light, EN/KO, filters/empty/loading/error/offline and long rows, all original
captures opened. Test order/visibility defaults, unavailable-key preservation,
reordering boundaries, serde restart and switch behavior; filter pipeline and
selected-row visibility; scroll end/footer invariance and owner action dispatch.

Current: navigation preference model passed22 filtered tests at5e3d4a7e,
including three sidebar form/space policy regressions. New controls, projection
helpers and consumers still require their changed-head checks. No complete
sidebar parity pass is claimed. The earlier height package has only bounded
source/gate/inspected-height approval; theme and OS input evidence are separate.

### Shared row checkpoint

The coordinator approved `kit::sidebar_header_row::SidebarHeaderRow` because the
existing private sidebar row had no shared density, hint or leading-slot
contract. Its Header/Compact/Inline variants use one renderer; wrappers are
36/56 with a separator, inset 8 without one, or full width respectively.
Rows are 28/36 high, rounded 8, with 4/8 inner padding and gap 8. Icon slots
are Header 16/20, other variants 14/18, including a null icon. The existing
14/20 Label metrics use normal weight for the reference's body 14, preserving
the tight row height without changing the shared font inventory. Selection
and hover share the reference sidebar fill; pressed surface2 wins. Persistent
trailing content overrides the hover-only shortcut. Activation is supplied by
one callback; optional focus tracking does not claim keyboard activation.
Actual GPUI bounds/press and palette-token regressions are installed. The first
kit strict check returned lint defects; corrected control execution remains
pending, separate from the completed theme prerequisite.

V1 reserved additional shared glyphs History, ArrowDown, Blocks, FolderPlus,
CircleHelp for this package. Gauge remains its existing G2 producer; no local
fallback or second spelling is added. Final glyph first-use captures are owed.

The five new sidebar glyphs append to the shared inventory and asset paths.
Their vectors are Lucide 1.50.0, matching Paseo's dependency and imported aliases;
shared stroke remains Paseo's explicit 1.5 rather than upstream's default 2.
The upstream license is retained with the design assets. Inventory uniqueness,
view box, stroke and actual asset loading have regression tests. No existing
vector, alias or Gauge producer is replaced; first-use runtime proof is pending.

`sidebar.ftl` supplies the shared builtin labels, both settings card headings
and tooltip descriptions, reorder labels and explicit body states in English
and Korean. The closed builtin/section mappings borrow the generated prepared
catalog; no screen string table or runtime formatting is added. The new
catalog/state vocabulary is a skeleton, not proof that registry loading or
all named actions are integrated yet.

First narrow model test stopped in translation generation before compiling the
new model: every Fluent message needs its own translator context comment.
The catalog is corrected per key; this failure remains in the first log.

### Sidebar form and width ruling

The native lead delegated `view::panels`' sidebar-only form/space policy to
this package. Global PHONE=768 remains unchanged for 45 other chrome/area
uses. `sidebar_form` uses Paseo's below-600 boundary; `sidebar_layout` returns
Sheet/Hidden/Docked with clamped width and a typed Sharing result. Workspace
auto-hide applies when its content budget cannot fit. Settings requests the
shared 720-point content minimum but is temporarily exempt from automatic
hiding: its sidebar currently replaces the agent sidebar and is the only path
to its sections. E2 will consume the same Sharing result for its list/back
fallback, then replace this exception. S2 does not build another Settings frame.
The native default264 and frame inset12 are deliberate retained differences,
per the lead's ruling: the prior native shell and its framed chrome keep their
layout. Min200/max600/normal content400 remain the reference limits.

The root adapter also temporarily retains Settings' existing PHONE form. Its
current frame already draws the compact list/back stack at 600–767; docking a
second copy there before E2's effective-form consumer would duplicate navigation.
Workspace sidebar form uses600 immediately, while chrome and other areas keep
PHONE768. E2 will remove the Settings form guard and the auto-hide exception
atomically with its shared-budget frame consumer. No second width constant or
Settings renderer is introduced.

The inline section accessory uses one private settings-section renderer;
existing sections and far-right trailing controls preserve their geometry.
The shared Info button retains icon14, padding4, leading margin-4 and hit
slop8, measuring its visual22 bounds separately from the expanded target.
An explicit tooltip opt-in opens compact explanations on press, top-start at
offset8, using the existing engine and outside dismissal. Legacy touch
suppression and centered composer tips are unchanged. Real GPUI regressions
cover the expanded click target, compact open/outside dismissal, desktop hover,
legacy touch suppression and section heading/card bounds. Execution of these
corrected regressions remains pending. Real platform accessibility/OS back is not
claimed by pointer tests on a virtual window.

The theme/input prerequisite is now bounded source/final-native/inspected-visual
approved at `eddce254` (1227/0/1); its CharacterInput return is closed. This
sidebar package begins after that boundary and does not claim the theme's
platform limitations as solved. Packagebase2 `6036eeb8` is merged at `5e3d4a7e`.
Navigation's corrected first narrow run passed22 tests; the first catalog
failure is retained. New layout/controls still require their own narrow checks.

### Callout renderer checkpoint

The shared `kit::sidebar_callout` follows `components/sidebar-callout.tsx:32–245`
through the existing tokens and GPUI element engine. Its source uses a natural
height, horizontal16/vertical12 inset and a top border only; error changes that
border, while success remains neutral. Title14 normal clamps to two lines;
description12 wraps. Dismiss14 has an expanded8 hit region. At most two typed
actions use8/12 padding, radius6, text12 medium and source pressed/disabled
opacity. These private actions are part of the strip, not another generic
Button API. Actual bounds, one-click dispatch, disabled refusal and the expanded
dismiss target have GPUI regressions; compilation and runtime evidence are owed.

Paseo mounts the viewport in its web slot while its native slot returns null.
The GPUI app will admit the shared strip deliberately; this is a stated
platform mapping. Caller modules own observed facts, priority, persistence and
dismissal; absent setup observations or remote architecture cannot invent a
setup/Rosetta warning. W8 and the desktop owner consume this renderer. No
provider or slot is claimed integrated by this control-only checkpoint.

Packagebase4 is merged preserving its terminal presence registration and the
sidebar-specific form policy. The exact pure-policy checkpoint5e passed its
three sidebar policy regressions within22 filtered view tests. A local kit/view
test ended during dependency compilation with no test result; it proves no
control behavior. The following narrow checks use immutable commit snapshots,
so new small F4 commits do not change their executed source or their scope.

### Workspace projection policy checkpoint

`view::sidebar::targets` consumes the same full open sections as the row
projection, after filtering/pinning/grouping and before each ShowMore paint
limit (`sidebar-projection.ts:67–99`, `utils/sidebar-shortcuts.ts:65–131`).
Collapsed sections contribute no targets. Relative navigation wraps the entire
order; only the first nine have numeric badges. The generic payload will be
the canonical node/workspace target once the registry producer is available;
managed child-agent session rows never contribute workspace identity. The
command owner consumes this list and owns key modifiers and delayed reveal.

`view::sidebar::filters` carries producer-owned host/project/normalized-label
keys without deriving or normalizing identity. Cold host selections do not
fall back until the registry is loaded. Project filters intersect currently
available projects but never rewrite saved keys on disconnect. Labels use OR,
including explicit Unlabelled. A truly empty project keeps its create header
without a label filter; a project emptied by hidden rows or label filtering
drops. The picker must continue reading the unfiltered project catalog. These
helpers follow `sidebar-model.tsx:89–167`, `sidebar-project-filter.ts` and
`sidebar-labels.ts`; real registry hydration and row integration remain owed.

The matching pure `sidebar::callouts::State<I,C>` retains provider-owned typed
identity and payload. Replacement preserves source tie order but advances an
opaque token, so old cleanup cannot remove a new registration. Highest priority
wins; ties keep first registration. Durable notices wait for the preference
read before admission; ephemeral notices can show immediately. Dismissing
retains the key, while clearing providers preserves keys and token history.
Tests cover replacement/cleanup, loading, priority, durable suppression, a
different project's key and serialization restart.

Unlike the source's unbounded arrays, this native edge refuses more than32
providers,128 retained dismissal keys or512 UTF-8 bytes per key. The bounds
limit retained file/provider input and never evict older choices. A refusal
leaves the prior state intact and must be surfaced by the caller; the source
provider facts and final persistence hooks remain outside this pure module.
This model is not a claim that setup/update/Rosetta admission is wired.

Alphabetical ties reuse the one shared `view::alphabetical::compare` helper
introduced in449fe38b, adopted as exact source/test blobs without the provider
package's unrelated history. Its producer narrow check76557b9b passed all-target
Clippy. The coordinator's approved Unicode-lowercase then original-scalar order
is deterministic; it deliberately differs from locale/numeric tailored
collation. Sidebar consumers must retain status/time/relevance precedence before this
name tie; the workspace projection consumer is still pending. No second comparator or normalization layer is introduced.

### Manual ordering policy

The pane owner confirmed there is no competing shared list drag controller.
Sidebar row handlers will reuse GPUI typed drag/move/drop and its gesture
lifetime, with canonical project/workspace keys; pane tab identities and pane
edge policies are not workspace identity. Pointer/touch threshold parity and
actual drop captures still require verification at consumer integration.

The pure `sidebar::order` follows
`hooks/sidebar-workspaces-view-model.ts:471–601` and `utils/sidebar-reorder.ts`.
Applying stored order only rearranges its known slots, preserving new structural
rows until reconciliation. New projects append to saved order; new workspaces
prepend. A visible drag reorder puts those rows first and retains hidden/stale
keys afterward, unlike navigation inventory's interleaving rewrite. Unchanged
orders produce no write. Before/after intent moves only a captured visible group;
a same-row or stale-key drop does nothing. Tests cover these exact differences.

### Shared workspace row checkpoint

The approved `kit::sidebar_workspace_row::SidebarWorkspaceRow` owns one frame
for pinned/project/status placement, following
`sidebar-workspace-row.tsx:467–521`, `sidebar-workspace-row-content.tsx:87–548`,
`sidebar-row-backdrop.ts` and `density.ts:23–32`. Min36/44, bottom4, vertical8,
left8/grouped16, right12 and radius8 keep full-width highlights on one rail.
Leading16/20 centers on the title's20 line; title14 normal uses.76 idle,1 hover,
or.92 creating. Selected backgrounds win over hover, and press/drag surface2
wins over both. Metadata stays a caller-owned shared-control slot.

Trailing data preserves width while a temporary number hint hides it. Compact
permanent menu replaces the data; desktop hover overlays its action over a48
wide same-RGB alpha scrim that becomes solid at55percent. The numeric badge is
18 high, top1/right0. No private button is added; action slots consume the shared
button/menu controls and stop parent navigation even for a disabled child.
Literal geometry covers both themes/densities, grouped/flush and metadata;
policy tests cover slot reservation, hints, open menus and source paint order.
The renderer has no app type, provider request or local gesture engine.

The coordinator approved existing GPUI drag activation2 as an explicit interim
difference from Paseo desktop6. The lead will add a per-element threshold with
legacy default2; the sidebar switches to6 when that seam lands. No global
threshold or custom controller changes here. Source native hold180/slop4 is
not claimed by desktop pointer probes. The source1.02 scale is deliberately omitted by the coordinator's native
mapping: GPUI has no general Div transform, so its native preview retains
natural size with source border/surface/shared shadow. This is not a final
inspected drag-visual or full F4 pass.

The coordinator accepted natural-size GPUI drag preview as a deliberate
native difference from `sidebar-workspace-row.tsx:499–506`: no1.02 Div scale,
with source fill/border/shadow and the existing GPUI preview lifetime. No
custom canvas, second layout or global drag behavior is introduced.

The first kit narrow check383e7aba reached compilation and returned strict
lints in new control documentation, Debug and test callbacks. Their corrections
are kept as subsequent commits; the failed run is not relabeled green. The
returned snapshot predates the callout/workspace-row/order additions, whose
own compilation and behavior remain unverified until the next changed-head
checks. The gate package remains unfinished; no final review is submitted.

## Resume point

Done: base4 merge, corrected navigation model/policy22-test proof, shared
header/info/callout/workspace-row source checkpoints, exact shared alphabetical
helper adoption, pure filter/target/order and callout lifetime models.
First kit strict return383e remains red; documentation, Debug and callback
lints are corrected. Mechanical producer-owned view test-lint corrections are
adopted without changing policy values. The package is not complete.

Next: adopt the lead's gated shared button/menu/per-element drag line preserving
all newer local controls/glyphs; run changed-head kit/view/app narrow checks on
the authorized core lane, then wire settings/header/footer and canonical
workspace projection. Concrete registry/pane/create/plugin/labels/usage action
hooks remain their owners' dependencies. The staged pane1720 handoff is on
HOLD until its self-contained successor; no pane or plugin framework is added.
One final package gate and actual-fleet capture/review remain owed.

The lead's exact shared linef5e9b3d6 is adopted provisionally as a merge, with
its own native gate tracked separately. Shared assets preserve all97 unique
glyphs, both complete test modules and the embedded Lucide notice. Its vendored
GPUI adds `drag_threshold` without changing the default2. The workspace row
now names `kit::drag::LIST_DRAG_DISTANCE` (6) so its caller's normal `on_drag`
uses the reference threshold. A real row-input regression covers5-point no-drag
and7-point drag; no area gesture state is added. The producer's whole gate is
not inferred from source adoption.

### Sidebar settings row profile

The same settings row renderer now has typed Standard/SidebarNavigation
profiles (`styles/settings.ts:31–37`, `sidebar-nav-section.tsx:222–226`).
The existing API delegates Standard unchanged: gap12 and compact wrapping.
Sidebar navigation uses gap8 and no wrapping. Padding16, border/card/frame
remain shared. Both-theme literal tests retain ordinary geometry and check
wide/compact behavior; a consumer never overrides the control's gap.

`keyboard/availability.ts` returns no Shortcut at all while compact. Sidebar
settings derives its own density from the existing shared sidebar viewport
policy, not the Settings list/back form. The prepared consumer suppresses
compact hints and reads the effective keyboard registry; its final W22b
primary-key seam and actual long-label/action fit remain to be integrated
and verified. E2 reserved only three mechanical tooltip-clear calls at its
existing navigation controller entries; its frame/nav/layout remain untouched.

### Narrow check correction at 10:38 UTC

The existing local check at `13c9afbfe` finished with Clippy 101 before the
filtered tests ran (`f4-core-narrow-2`). Two test assertions accidentally gained
`let _token =` while correcting ignored successful registration results. Restore
those assertions as expressions; the production admission policy and both error
expectations remain unchanged. This is an author error, not a base failure.
All subsequent compilation, tests, capture builds and gates use the remote
build helper, following the operator's 10:3x rule. No local Cargo job remains.

### Both-panel owner action and updated shared line

Reuse the existing `Shell::toggle_sidebar` and `Shell::toggle_code` through
`Shell::sidebar_toggle_both(window,cx) -> bool`. The source
`workspace-screen.tsx:2632–2657` and `utils/desktop-sidebar-toggle.ts:15–30`
close each open panel if either is open; only two closed panels restore both.
`_layout.tsx:398–411` falls back to the left list off a workspace. Desktop
intent reads the existing saved left visibility, not whether width constraints
temporarily hide it. No second panel state or key listener is introduced.
The current native workspace bridge remains the Session route with an open
session; canonical empty-workspace activation remains W11/W12-owned and is
not inferred from a path. W22 owns dispatch and calls W12's idempotent focus
exit after a handled sidebar action (`use-keyboard-shortcuts.ts:240–244`).
Two actual GPUI shell regressions cover the four desktop combinations and
the off-workspace fallback. Their execution is pending. The isolated shell
fixture has no link/updater, so this does not probe an external node.

Merged the published line `2e95b7d31` after the previous check ended. It brings
base 5, canonical WorkspaceId/plugin values, W12's conditional shared producer
and G4 preflight without a separate node-head merge. Preserve the source-only
limits of each producer; the canonical registry runtime is still unavailable.

### Callout dismissal persistence

Reuse the existing `Preferences` document, `PreferenceChange`, and
`Shell::prefer`; do not create the source's separate AsyncStorage file in
native. `sidebar_callout_dismissals` is a typed bounded band, omitted while
empty and read leniently so malformed dismissal input does not erase unrelated
preferences. Keys stay provider-owned; admission still suppresses a matching
key after restart. The same named 128-key bound refuses growth without evicting
prior dismissals. Tests exercise JSON restart, duplicate keys, malformed and
oversized bands, retained theme, and the admitted update through the existing
preference change. E2 authorized precisely one exhaustive read-next-frame arm
in `settings/prefer.rs`; no field/frame/application behavior was copied there.
W19's canonical multiwindow preference delta merger must include this band
when its checked producer lands; no alternative save path is introduced.

### Native callout registration seam

`areas/sidebar/callouts.rs` joins the pure admission store, the shared kit strip
and the existing preference setter. Providers register an exact provider-prepared UI
`SourceId`, priority, optional durable key, current-theme/current-language drawing
callback, dismissal permission and optional callback. Domain targets are captured
by the provider; the registry never resolves a current machine alias to authorize
an old action. The returned token is required for cleanup, so an old owner cannot
unregister its replacement. A default-empty slot sits above the fixed footer.
No notice appears until an owner registers actual facts and working actions.

Native preferences are already loaded before Shell exists, so the first
registration loads the typed dismissal band synchronously, without a second
AsyncStorage read. Dismissal removes the admitted entry, persists a changed
durable band with `Shell::prefer`, then calls the provider's callback. Ephemeral
removal does not save an unchanged band. Pointer/retained-key/cleanup regressions
use a real GPUI strip around an isolated shell with no link or updater. They
prove in-memory preference dispatch, not disk-write or multiwindow completion.
W19's future canonical preference-delta consumer must synchronize this band;
no second save or event mechanism is added here. Setup, Rosetta and update
admission remain with W11/W8/W19. This default-empty viewport is not evidence
that those providers or unsupported platform facts are implemented.

Scoped callout source ids must remain dynamic: Paseo's missing-setup source uses
node/project scope in both id and dismissal key. The typed `SourceId` therefore
accepts an exact bounded provider string, with no decoding, case folding or
trimming. It is distinct from `DismissalKey`, which trims at its source boundary.
Both share the one existing 512-byte key bound and control-character validation;
there is no second id normalization rule. A regression distinguishes two project
scopes and verifies that source spaces stay exact while dismissal spaces trim.
The app's fixed static-id prototype was corrected before any checked adoption.
A real nondismissible GPUI fixture also verifies that a provider's accidental
raw dismiss callback cannot bypass the shared admission/persistence boundary.

### Ordered menu hierarchy handoff (prepared, not installed)

The coordinator assigns the existing shared menu engine to S2 only after G2
checks and hands off its current trigger/modal delta (`d377`/`1bb` successors).
No competing draw or item change is present in this tree. Read the complete
source `ui/menu/menu-navigation.ts`, `menu-context.ts`, `menu-sub.tsx`,
`menu-item.tsx`, `menu-surface.tsx`, `menu-overlay.tsx`, `menu-root.tsx` and
the source path regressions. Prepare typed state/data outside the renderer.

The engine keeps one outward page path for desktop flyouts and compact
replace/back navigation. A sibling replaces its branch, a missing definition
prunes descendants, and reopening returns to a newly anchored root. Existing
flat MenuItem/handlers, menu_right, KeepOpen, G4 ReturnFocus/modal seam and
QUICK remain delegates into one renderer. An additive entry wraps the legacy
item with an action/check or submenu role and optional caller-prepared shared
leading content; a submenu is navigation and cannot become a close-on-select
application action. New callback addresses carry page/row; owners retain the
authoritative captured action target instead of resolving a visible label.

Source values to preserve: hover open 90 ms, close grace 260 ms, flyout overlap
5, check rail 16, chevron 14, current-value gap 6 and indicator 6. An open
hoverIntent:false page locks hover-close across the surface. Root widths are
232 for Display and 260 for workspace actions. One outside check includes all
visible surfaces, so entering or pressing a child cannot dismiss its parent.
`menu-geometry.ts` now explicitly says 28 desktop/40 compact; the existing
reviewed native flat engine uses 44 compact. Keep that legacy API/default
while the same row renderer accepts the hierarchy's explicit source profile;
there is no claim that 44 equals 40 and no area override. CompactMode defaults
to Popover; Display explicitly chooses Sheet. The existing shared 60/90 sheet
is not by itself evidence of source dynamic-content sizing up to 80% with
0.45 backdrop. Reuse and minimally extend its common frame/gesture policy,
without introducing menu-specific physics or a second scrolling engine.

This remains preparation. Renderer installation, path/timer/keyboard/geometry
regressions, actual flyout and compact Sidebar use, the final F4 gate and
capture matrix are still owed. Source fill/text-form pages must remain an
explicit separate capability until a shared typed content slot actually exists.

### Exact remote return and row correction

The immutable `636cb39f4` strict Kit/View/App run
`shell2-narrow-20261115T104115Z-636cb39f` finished native-fonts 0 and
native-cargo 101. It reached actual Kit compilation and returned a nonexistent
`radius::SM` name in the numeric workspace badge plus five unnecessary path
qualifications in the new header/workspace-row tests. The source badge is
`borderRadius.sm == 2` (`sidebar-workspace-row-content.tsx:299`,
`styles/theme.ts:613`); use one named control constant with that exact value,
not a nearby token or an icon-size constant. Remove only the reported redundant
test paths. Tests did not execute; this is not a control or app pass.

### Shared menu path state checkpoint

Merged exact reviewed G2 producer `8158be6d..6ece4a76` plus result docs
`36a84b9f`; its six Kit menu source/test blobs matched the reviewed head before
this additive work. README conflict retained both base-5 allocation refusal
and the shared trigger/modal descriptions. No full F1 branch was consumed.
The coordinator's base 6 `18eedad4` is also merged after a clean checkpoint,
including W12's checked action/focus/query follow-up. The native lock is known
stale on that base; do not use --locked until the lead's correction.

`menu::hierarchy::{PageId,PageDepth}` and the one private path now belong to
existing MenuState, porting `menu-navigation.ts:11–71`. New public operations
open/close a branch, pop Back, expose current/outward pages and prune missing
definitions. Both original opening functions reset to root/fresh anchor. The
legacy const close API is preserved: closed getters expose no page and a later
open clears stored path, so a closed branch cannot reappear. Pure source
regressions cover nesting, sibling replacement, same-branch no-op, depth, Back
and missing parent; a real GPUI owner regression covers close/open_end cycles.
The action/focus/flat row engine stays untouched. This is control-only state
and API, not a page renderer, flyout, hover timer or compact-sheet completion.
The additive row/page data and renderer are the next step on this same engine.

### Shared row extraction before hierarchy presentation

Move the existing `row_body` and `leading` functions into `menu/row.rs` before
adding hierarchy content. There is one row renderer for old and new callers;
`draw::ordinary_glyph` and its published test paths/constants remain in place.
An independent source comparison confirms both moved bodies are byte-identical
after only the function rename and helper qualification. This preserves old
child geometry, input handler, QUICK/fill and layer logic while keeping each
product file under 500 lines. The existing literal geometry/feedback/focus/
modal tests remain the regression suite; no mirror implementation test is added.
Runtime/narrow proof of this extraction is still pending.

### Additive hierarchy data and the same row frame

`hierarchy::{Hierarchy,Page,Entry,Role,Subtrigger,Check,Handlers}` wraps the
existing MenuItem instead of changing its public fields or flat callback.
An entry is an action with check semantics or a submenu, so setting a value on
an action cannot silently do nothing. Caller-prepared leading content stays
in the same sixteen-point rail and pending/success feedback takes precedence.
The new addressed callback is page plus row, retaining captured owner targets.
This is list-page data; no custom form/fill-page implementation is claimed.

The extracted row body now accepts additive check/leading/trailing slots and
an explicit RowProfile. Legacy remains 28/44; Reference uses source 28/40.
A leading check reserves sixteen points when unchecked; a trailing unchecked
value adds no column. Checked values add no selection fill. Leading checks
use neutral foreground and trailing checks muted foreground independently of
a destructive label or success feedback. The original flat entry delegates
with default-empty slots. Literal GPUI fixtures in both themes check 28/44/40
heights, the 13-point content rail, and the 37-point label rail after a reserved
check. Execution remains pending; this is not a complete hierarchy renderer,
page input/hover controller, compact sheet, Sidebar first-use or F4 approval.

The follow-up line lock merge `1783b8bd5` accidentally committed README
conflict markers after a paragraph-comparison assertion failed and later
commands were still invoked. Restore the prior complete union in a separate
correction commit. Independent normalized paragraph-content comparison proves
it contains every lead paragraph plus the F4 staged-inventory text, once each;
no Rust source or lock correction is reverted. Subsequent dependent commands
must stop on a failed validation. The final feature range includes this
correction; the intermediate merge is not a clean review endpoint.

### Existing overlay admission marker dependency (unpassed)

Adopt only W22's four-path marker packet from exact source
`5a54c0aca892d29e5aecdc80ed0fed5f3c82272b` against `dd13ce378`, independently
verified against its Git diff and SHA-256 `098cfd8557652904fa4b54609c012d272f396a9d2998eaf541003764062b6224`.
This exports `popup::KEY_CONTEXT` and marks the actual focused flat-menu,
picker/dialog roots and existing popup layer. It changes no geometry, defaults
or key listener. W22 maps exact KeyContext membership to Admission::Overlay
through its one host adapter; no invented Menu scope or composition cache.
The hierarchy will mark one real focused root with this same constant. This is
an explicitly unpassed source dependency, not app/OS/native-input approval;
W22's checked producer/package remains a final integration prerequisite.


## Checked menu lifecycle dependency integrated

V1 bounded SOURCE / requested Kit NARROW passed exact d0954f4b + docs2599:
private per-open allocation lease, automatic pointer focus fence, recognized-key
and modal-outside containment. Both direct-cycle and nested-parent P2 closed;
Kit173/0/0 and strict Kit Clippy0 with actual changed crate compile/check.
Audit /tmp/work/m12-review/s2-menu-lease-d095/audit.json. The isolated producer
was handed to the lead separately; this F4 union is not that checked source.

Merge keeps the F4 path model/reset, the one extracted enhanced row body and
W22's provisional KEY_CONTEXT on the same real focused menu root. Input router
and seven lease tests retain exact producer bytes. Future hierarchy timers must
validate the same render lease before mutation. Historical native/Linux focused
TextField F5 HOLD is separate open; no hierarchy/Start/description/OS/full F4
approval. Main combined narrow/consumer evidence remains owed.

The previously adopted W22 marker leaves picker.rs at828 counted lines; lower
its stale829 record to828, never increase a size exception. No picker renderer
or behavior is changed by this bookkeeping correction.


## Sidebar settings and first shared navigation consumer

Read the whole pinned screens/settings/sidebar/sidebar-nav-section.tsx and
components/sidebar/sidebar-header-row.tsx again. The two existing bands use the
one typed Navigation inventory; settings rows retain source icon16, gap8,
single-line main label/hint, G2 Ghost MD icon14 arrows (32 pointer/44 compact),
boundary disabled state and the existing switch. No settings screen header,
card frame, button or preference store is copied. The source arrows remain
visible on compact widths; only unavailable shortcut hints are omitted.

Register Sidebar in settings/sections.rs::page only; E2 authorized three
mechanical sidebar_settings_clear_info calls in open_settings/settings_section/
settings_back. No other settings frame/nav behavior changes. Preserve L's
Translator seam and the canonical preference setter.

Search is the first live consumer of the same inventory and shared Compact
SidebarHeaderRow. Preserve stable nav-search id and Shell::open_palette.
The palette/entry.rs sidebar adapter now returns Option<Div>; title_trigger is
byte-identical. Hints use current legacy bindings_of, not copied defaults,
until W22's checked primary_keys producer. Sidebar600 determines density and
hint availability, independent of global PHONE768.

Compact rows own their outer8 inset. Move legacy padding out of the common root
into the legacy route group, one body wrapper, Add and footer wrappers; this
prevents double header inset without another scroll container. The sole scroll
viewport retains its previous inner width and vertical budget. Existing actual
height tests retain all vertical/focus/wheel assertions and additionally compare
the measured viewport to the body wrapper's literal8 horizontal inset.

New real GPUI tests bind actual source rows, press Up/Down/visibility, verify
boundary refusal and persisted ordering, then hide/restore/click the actual
shared Search opener. Both themes, EN/KO,390 and700 (the600-767 boundary) check
literal action-frame dimensions32/44 and shared header28/36. Registration uses
the actual existing Settings entry. These tests are installed, NOT executed yet.
Combined F4 Kit/View/Link/App strict narrow check and first-use captures are
still owed. Other builtin openers, plugin groups, workspace projection and the
final hierarchy remain pending their owner contracts; no full F4 pass.
