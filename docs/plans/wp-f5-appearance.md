# F5 Appearance package

Status: audited; skeleton next. The existing eight-choice palette package is
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
- Next: confirm the shared-control and plugin contracts, write the typed
  skeleton and narrow checks, then integrate the announced validated base.
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
