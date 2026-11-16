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
