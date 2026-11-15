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

Current: navigation preference model and regression tests are installed as
the first skeleton step; narrow compilation remains pending. No complete
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
Actual GPUI bounds/press and palette-token regressions are installed but not
yet compiled while the preceding package's whole gate remains queued.

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
legacy touch suppression and section heading/card bounds. These tests remain
uncompiled at this checkpoint. Real platform accessibility/OS back is not
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
collation. Sidebar status/time/relevance precedence is preserved before this
name tie. No second comparator or normalization layer is introduced.
