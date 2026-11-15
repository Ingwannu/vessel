# F5 Appearance package

Status: typed preparation and shared-tile consumer committed; compile and
full screen skeleton remain pending. The existing eight-choice palette package is
reviewed separately through executable source `fb711444` and its bounded
visual disposition. This package adds the remaining Appearance contract;
it does not reinterpret that earlier approval as complete F5 parity.

## Scope and source

Paseo 0.15.4, snapshot `1c00b365`: survey `gaps.md` sections
“Built-in appearance themes and system theme”, “Plugin-contributed themes”,
and “Fonts, text sizes, content width, syntax palette and preview”. Assigned
tracker rows: 709–711 and 742–744. Related Settings navigation, localization,
plugin runtime, editors and terminal owners retain their existing modules.

Primary references:

- `appearance/theme-picker.tsx`, `appearance/provider.tsx`,
  `plugins/themes/index.ts`: built-in order, contribution identity and
  eligible-host deduplication, retained missing selection and System fallback.
- `screens/settings/appearance/appearance-section.tsx`: family and numeric
  drafts, blur/submit, content-width reset, syntax selection and preview.
- `hooks/use-settings/storage.ts`: finite/floor/clamp numeric parsing,
  200 UTF-16-unit font-family validation and stored defaults.
- `appearance/apply.ts`, `appearance-preview.tsx`, `preview-snippet.ts`,
  `packages/highlight/src/themes.ts`: noncompounding UI ramp, independent
  content/code axes, resolved syntax colours and local preview overrides.

## Existing assets and decision

[Decision Log]
- Purpose: complete Appearance through the existing stored preferences and
  sole preference-to-token adapter, without a second theme or editing engine.
- Existing constraints: `Theme::ALL` and the seven concrete kit palettes are
  already present. The existing text field, selector, settings rows and code/
  Markdown renderers are reusable. Font sizes currently expose narrower
  ranges, content width has three legacy presets, and plugin contributions
  have no connected consumer here.
- Alternatives: a second settings store, custom input engine, arbitrary CSS,
  or one token table per contributed theme. All duplicate existing ownership.
- Choice: modify/reuse the current preference/store/adapter and controls;
  pure typed validation and edit states in view, local file persistence in
  link, semantic colour/type application in kit, composition in Appearance.
- External solution: the standard library, existing serde/thiserror, existing
  text engine and existing syntax renderer suffice; no new dependency planned.
- Consequences: legacy preferences require explicit compatibility tests;
  contributed theme removal must retain the saved selection, and changing a
  palette must reapply typography/syntax overrides rather than erase them.

## Ownership and integration seams

The plugin runtime owns contribution decoding, transport, feature gates and
host identity. Appearance consumes its typed validated theme snapshots and
owns selection/fallback plus the one kit-token adapter. No dedicated theme
RPC or duplicate plugin registry is introduced. The runtime contract is
coordinated with the plugin owner before wiring.

This package owns the Appearance content-width row and its reset semantics
because they are in the assigned font/preview section. Other Settings layout
pages remain with their owner. A shared numeric shell, if needed, wraps the
existing text engine; it must not create another input implementation.
Historical theme-tile/preview reservations have no implemented source and
are transferred with this assignment. Any new kit control requires the
coordinator's approval before implementation.

Existing proposed ADR 0070 supplies the palette/type separation. Its earlier
font/content/syntax deferrals are superseded by this package's assignment;
implementation decisions and remaining deliberate platform differences will
be recorded there with the code, rather than leaving contradictory deferrals.

## Build order

1. Skeleton: typed font family, text axes, nullable content width, syntax id,
   draft/commit/restore states and contributed-selection fallback; fixtures
   and Fluent words; screen shows each state with the existing controls.
2. Fill: real saved preference changes, typography/syntax application and
   noncompounding preview; consume the plugin runtime's actual typed catalog.
3. Close: focused private-display captures, one complete native package gate,
   one combined review. No whole gate per commit or repeated broad matrix.

The validated application-base announcement is the integration prerequisite;
no arbitrary unpublished base is treated as approved. Small pure model work
can be prepared while that base is being validated. Existing cargo configs,
font assets and other owners' pending changes are preserved.

## Resume point

- Done: original eight-choice palette producer/consumer is source/native/
  bounded-visual passed; actual F5 survey and reference assets audited.
- Missing: the remaining family/content/code/width/syntax draft controls,
  preview and plugin contribution consumer.
- Prepared: token-only tile `d266db93`, pure draft/type preparation
  `4884b549`, new range-edge regression `589c56e4`, breakpoint `3e47b364`
  and the single tile consumer `cc43236c`. Package base `8d5513e4` is merged
  at `f556e55c`, without a conflict. These are not compile/runtime approvals.
- Next: narrow checks, complete the field hooks after the shared field
  owner's validated split, and compose all remaining rows and preview.
- Not claimed: complete F5 parity, plugin runtime connectivity, IME/AltGr,
  hardware GPU or mobile system-bar support.

## Shared-control implementation

The theme-tile and Appearance field presentations are approved for this
package. The first control-only source adds `kit::theme_tile` with
`State::{Chosen,Idle}`, `Preview::{Single,Split}`, grid/cell geometry and
small12 label. It takes only tokens and caller-owned content/action data.
The following consumer change removes the old area-local miniature; the
final feature has one renderer. Headless geometry checks exercise Single
and Split, both choices and both colour schemes.

The existing field owner retains the pure presentation/style split and the
single select-all/editability implementation. This package consumes those
hooks rather than racing a second implementation. Its one later addition
is generic blur notification through the existing engine subscription;
existing constructors keep their no-extra-callback behaviour. No field
source is changed before the split's validated handoff.

## Preference preparation and differences

The existing Appearance preference gains independent content size, interface
and mono families, and a syntax id; unchanged new defaults are omitted so
the existing document remains byte-compatible. Numeric width extends the
same `content_width` enum, not a second preference field. Existing
Comfortable/Wide/Full files retain their intent. Comfortable follows the
single default bound, custom Pixels carries a validated width, and reset
returns Comfortable. Unlike Paseo's nullable number format, the native file
retains its existing enum spelling and writes a tagged numeric variant;
the following/reset behaviour is the same, and a null reads as following.
Legacy Wide/Full remain readable until explicitly changed or reset.

The old area-local tile and its Body14 departure are removed. The shared
tile uses small12 and the source's viewport breakpoint of 600, independently
of Settings navigation's compact breakpoint. Family placeholder/hints say
default font rather than system default because native retains the owner's
embedded Geist/Pretendard default. No extra content-family preference is
introduced; content follows the interface family, with its independent size.

Range references: `hooks/use-settings/storage.ts:62–73`. Approval to change
the two inherited refused inputs (UI21→22, code9→8) was obtained; all other
inherited cases remain unchanged, with positive endpoint regressions added.

Validation status: two local checks expired in the build queue before
compiling. Remote focused kit test preparation is waiting in submit (no run
id yet); it pins `f556e55c`, before the later breakpoint/consumer changes.
Neither is a compile result for current source. The new preference changes
are preparation only: their renderer and typography/syntax application are
not complete. Plugin scalar staging and the shared field split are likewise
not imported before their owners send narrow-green handoffs.

### Syntax and family data preparation

`kit::theme::syntax` (also reexported as `kit::syntax`) owns the eight
syntax palettes and complete twenty-role source values. A single ten-role
expansion serves the compact tables; GitHub keeps its hand-tuned twenty-role
exceptions. These data do not add a highlighter or renderer. The actual
`resolveSyntaxColors` implementation pairs Nord light/dark even though an
introductory comment calls it dark-only; the implementation wins. Dracula
alone uses one palette on both schemes. The existing code frame/diff colours
remain outside this palette; syntax consumption/preview are still pending.

The family input now retains decoded ordered names at the validation edge,
including quoted commas, CSS hexadecimal escapes and generic intent. Native
resolution must query installed/embedded names only; it never loads a file
or a network address from this input. Generic categories use readable
embedded fallbacks. Raw spelling remains the saved/editable value.

Focused run `themes-20261115T065416Z-f556e55c` compiled the kit and returned
Rust E0599: GPUI's Stateful wrapper does not expose the Div child-bounds
hook used by the new test. No test ran. The correction keeps the production
layout body in one private Div builder and measures it before the opaque
hover/click wrapper. It changes no authored geometry or expectations.

The prepared font-stack API keeps resolved names and shared fallback lists
in one small snapshot, read by type styling and the same shaping-based
measurement. Empty/unknown stacks use the existing embedded defaults;
custom stacks retain Pretendard. No host font enumeration happens in paint.
The current control commit exposes only kit type-step configuration; its
Appearance adapter and component-layer projection remain pending the shared
field handoff, and are not claimed as a working family preference yet.

### Contributed-colour preparation

`Tokens::contributed(Scheme, ContributedColors)` accepts only decoded UI
colours; contribution ids, hosts, validation and optional-accent resolution
stay outside kit. Its shared neutral-to-token mapping also serves the five
existing authored tints. The four shipped Light/Dark constructor bodies are
not edited. Light custom popovers/destructive foreground use the supplied
background; dark popovers use control and destructive foreground stays
white, as the source builders specify. Status/diff signals retain their
scheme defaults, and alpha in supplied colours is preserved.

This is token preparation only. The typed plugin catalog/selection is not
imported yet, and no plugin tile, saved choice or runtime connectivity is
claimed. Existing page-only aliases without a Paseo counterpart retain the
same scheme defaults used by the accepted named-palette mapping.

Focused run `themes-20261115T070649Z-1eeba47d` compiled current tile/syntax
preparation: breakpoint test passed, frame test failed (actual 106.5 versus
unsnapped mathematical 106.625). The fake window uses scale 2 and GPUI's
physical-edge snapping. The correction keeps the same tight tolerance and
uses independently calculated snapped bounds for scales 1, 1.25, 1.5 and 2,
including pre-layout snapping of the two-point border at 125%; no product
geometry changes. The new four-scale expectations still require execution.

### Current form and typography seam

The form's pure `edit`/`commit`/`saved_text` functions return parsed preview
axis deltas, filtered input text and at most one PreferenceChange. The old
generic draft-state prototype is removed: the TextField engine remains the
sole live owner of raw text, caret/IME/history, with no second form editor.
Size fields filter ASCII digits; width deliberately retains exponent/radix
notation as the source does. Invalid input restores CURRENT saved values,
not an opening snapshot. Legacy Full width has a typed translated-display
case until edited/reset; it is never guessed from a label string.

`fonts::{FontSize,TextSizes}` adds nonzero generic committed units and
independent content/code size/line-height accessors. View owns source ranges
and defaults. The app's sole typography adapter resolves decoded names
against the platform's available faces on hydration/commit, never in paint.
Unknown names fall through to embedded defaults; generic aliases resolve to
actual embedded faces, so the kit receives no generic-font/file request.
Its new size/family projection is partial until the shared field style
handoff and chat/code/preview consumers are connected.

Focused kit run `themes-20261115T072159Z-9f0ff80f` is green: native-fonts0,
selected kit library tests121/0/0, changed kit actually compiled. It covers
the four-scale tile frame, earlier stack/syntax/contributed paints, not the
later text-size/form/app adapter edits. No whole native package gate or
screen capture is inferred from this focused result.

Package base2 `6036eeb8` merged at `31952112`, preserving the shared
Machine.node/Editability/events seams. Remote selected kit/view/app Clippy
run `themes-20261115T073515Z-31952112` returned101 after compiling view:
missing semicolon in the new family assignment, and an integer-division
lint in the UTF-16 test. Both are corrected without a lint exemption or a
changed bound/expectation. App/complete feature validation is still pending.

The corrected local view check was cancelled while still waiting for its
slot, before any compiler/log started, using the owned process group after
command verification. It supplies no compile evidence. Reviewed plugin value
types are now available and will be imported at this clean dependency point;
subsequent selected checks must pin that combined dependency state instead.

### Shared field handoff and component projection

Imported only the standalone field controls (`934c1790`/`f231ef26`, local
`b859beaed`/`8cf5511f4`), excluding unrelated Settings consumers. Their
source matches the owner's selected Clippy and20 engine-test proof; that is
a provisional dependency result, not this package's final gate or UI pass.
Package base4 `3cd0d3fc` merged at `82b47c788`; both terminal and syntax token
modules are preserved, both README additions retained, and the session size
record shrinks with the combined source rather than raising a budget.

The next control-only source adds `TextField::with_events` with optional
window-aware edit/focus/blur hooks through the SAME subscription. Legacy
constructors use their unchanged callback shape/order; a typed notification
variant prevents two change callbacks from firing. Appearance profiles reuse
that engine and the owner's shared settings-frame/select-all/Editability.
Family is flexible/max280; number64, width80, min36, source insets12/8/r6.
Input alignment is projected on the existing engine. Generic hooks do not
filter IME text; the consuming form must leave composing edits alone.

Component typography now reads active resolved families and independent
content/code units. It uses the component's supported mode/update adapter so
its Base projection and color tokens cannot remain stale after palette or
family changes. This intentionally changes the previous Markdown body14 to
the source desktop content default15; the four Light/Dark palette constructor
bodies remain unchanged. Independent syntax ink defaults to the source One
palette, deliberately replacing the old page-only code ink while retaining
existing code frame/error/diff paints. No new Markdown highlighter ships.

The selected `bcb0f07c` Clippy result remains FAILED (strict documentation,
float-operation, unnecessary-qualification and borrowed-large-value lints).
Current corrections remove them without exemptions; focused checks still
need execution on the corrected controls. Engine callback/profile tests and
preview geometry are included but have not yet run here.

### Current consumer seam (not yet validated)

The existing six registered line fields now compose the Appearance rows.
One window-aware editor stream filters numeric edits outside IME composition,
previews only content/code/family drafts, and commits/restores against CURRENT
preferences on Enter/blur. The shared numeric-focus select-all hook is deferred
until focus settles. Per-axis saved-text comparison preserves unrelated drafts
when a palette, syntax or another field changes. Width accepts source radix /
exponent notation, retains legacy Full until changed, and resets to following
820. The existing list owns eight syntax choices; one exhaustive adapter sets
the shared code resolver. Source example rows use the existing workspace lexer
and sole semantic kind bridge, also used by the editor's line adapter.

Deliberate native preview bound: uncommitted numeric drafts are clamped to the
same source min/max as committing, rather than attempting an unbounded glyph
allocation. No global typography changes while typing. Invalid/empty draft
falls back to the current saved axis. The preview uses source py8/px12 and
full-row marker/tint tokens; it prepares code/ranges outside paint. Preparation
failure shows a translated unavailable message and never changes preferences.

These are preparation sources only: selected checks and real form-event tests
are still owed, and no new F5 screen, plugin catalogue or package gate is claimed.

The prior baseline's fractional native UI type steps remain scaled from their
original values (fonts.rs::ui); Paseo apply.ts:35–45 rounds its canonical
integer size ramp. Preserving the existing native fractional Card/Tiny and
line-height steps avoids changing other owners' default frame geometry in
this package. Content/code axes and preview line heights are absolute and
rounded per source. The existing lightweight classifier is retained rather
than claiming Shiki token-by-token equivalence for the preview example.

### Contributed-selection model preparation

W16's reviewed e46cd57b scalar values are reused directly. No second colour
parser, registry, installation graph, transport or runtime schema is created.
The current pure catalog borrows eligible `(PluginAddress, ThemeContribution)`
pairs from that owner, deduplicates plugin/theme/contribution keys in first
appearance order, and picks the remembered eligible host else the first.
Host/generation identity is not part of the saved theme key. A window-local
256-entry recency cache bounds host hints (Paseo uses an unbounded module Map);
eviction affects only an old navigation hint, never saved selection. The
plugin owner separately bounds 256 themes per installation/node,128
installations/node and128 retained hosts; those are not duplicated here.

Appearance retains the existing eight-value Theme API. Its optional contributed
choice is `Selected(key)` or `Remembered(key)` plus the last built-in theme.
Selecting a built-in remembers the key; selecting a plugin retains the last
built-in. Missing active key resolves System without rewriting preferences,
then recovers when an eligible contribution returns. Cycling from an active
plugin starts at Light through the EXISTING shortcut registry action. Native
serde uses a tagged optional field, rather than two loose browser-storage
fields; old default documents remain byte-compatible. Current valid font/size/
width/syntax overrides are unchanged by either selection transition.

This is pure-model preparation, not a connected plugin picker. W16's current
native Registry29fcb004 has a pending narrow gate and is not imported. Its
Installation address/themes and installed(NodeId) seam will supply the actual
eligible data; no synthetic product catalogue or fake runtime is shown. The
sole app palette adapter still needs that live projection, contributed RGBA
mapping and refresh on catalog changes. Final F5 completeness remains pending.

The two newly added endpoint acceptance assertions are now separate tests
(interface21 and code9), as the coordinator requested. The inherited cases
retain only the two approved refused-input corrections (22 and8); every other
head case remains unchanged. Source: storage.ts:62–73.

### Terminal font seam

C2 approved the existing `terminal_grid::face` helper/import-only correction:
measurement and drawing both read one configured code FontStack snapshot,
including its ordered fallbacks. The default is unchanged. No grid/row/input/
cursor rendering is duplicated, and C2's Rc row-cache changes are untouched.
FontStack already supplies typed Eq/PartialEq identity for family+fallbacks;
C2 owns adding that identity to RowsKey and its quiet-grid regression. This
helper correction alone does not prove retained terminal-frame invalidation.
