# ADR 0070: Paseo's themes as the kit's palettes

Status: proposed, 2026-11-14. The catalogue below is the contract for
the palettes, the theme id and the appearance preference. It does not
draw Settings › Appearance. No wire number: nothing here is a protocol
call, so there is nothing to number at merge. No `node.settings` key:
appearance is a preference on the client, not a node setting.

Paseo 0.15.4 is the source of the values. Paths below are that app's
(`packages/app/src/...`) unless a Vessel path is named.

## Purpose and intent

The app offers the themes a person can pick in Paseo, by Paseo's names,
order, defaults and colours, on the kit's existing token names. System
still follows the operating system, and only between Light and Dark.
The owner's type rule stays: Pretendard before any generic family, and
no fallback that looks like pixel type.

## Existing implementation and constraints

The kit already has two schemes (`spikes/native/kit/src/theme.rs`).
`Neutrals` is Paseo's light theme and Paseo's default dark, except the
accent, which is the mark's colour (`assets::BRAND`, `#1A5868`), with
white on it. The page tokens beside them (`bg_*`, `text_*`, the board's
lane colours, the button tokens) are the page's, not Paseo's. Status
signals (`theme/status.rs`) are Paseo's light and dark status text and
tints, one set per scheme. Code colours (`code.rs`) are the page's
syntax tokens, not Paseo's highlight themes.

`Scheme` is only light or dark. `scheme_of` maps a window appearance:
`Light` and `VibrantLight` to light, `Dark` and `VibrantDark` to dark
(`kit/src/lib.rs`). The preference (`view/src/prefs/appearance.rs`) is
`System` (the default), `Light` or `Dark`. The app applies it at start
and again when the window's appearance changes
(`app/src/areas/settings/mod.rs`, `apply_theme` and
`observe_window_appearance`). `VESSEL_APP_THEME=light|dark` pins the
scheme for that run and the preference is not applied over it.

The faces are Geist, then Pretendard, embedded (`kit/src/fonts.rs`).
Nothing asks the system for a face. Interface size is 12 to 20 points,
default 14; code size is 10 to 18, default 12. There is no content
size and no font-family preference.

The kit must not depend on `view`, and `view` must not depend on the
kit (`spikes/native/README.md`). A preference type and a palette id are
therefore two types; the app maps one to the other. Colours live only
in `kit/src/theme.rs` and `kit/src/theme/` (QUALITY M8). A file of
product code stays at or under 500 lines, and a block of 12 lines is
not copied per theme (M3, M5). The theme enum is closed: every match
on it is exhaustive (T4), so adding a variant touches each match.

## Alternatives considered

1. Recolour Light and Dark onto Paseo's green accent, and derive the
   page tokens of those two from the neutrals. Rejected for this
   change: those two palettes are already shipped, and the accent is
   already recorded as the mark's colour. This ADR adds palettes; it
   does not repaint the two that exist.
2. One accent, the mark's, on every theme, so Zinc, Midnight, Claude
   and Ghostty differ only by surface tint. Rejected: those four are
   identified by their accent (Zinc's near-white needs dark text;
   Claude's orange; the two blues).
3. A new token for every Paseo name the kit lacks (`surfaceDiffEmpty`,
   `accentBright`, `statusMerged`, the terminal ANSI set, the syntax
   themes). Rejected: a name the kit does not already have is listed
   here and not added.
4. Paseo's font stacks (`system-ui`, and the mono stack that ends in
   `Courier New`). Rejected: the owner's rule forbids a generic family
   and a pixel-looking fallback. The embedded faces stay the stack.
5. A colour crate. Rejected: the values are data this crate already
   owns. No new dependency.

## The choice

Seven palettes, on the tokens the kit already has. Light and Dark stay
the bytes they are today. Zinc, Midnight, Claude, Ghostty and Pure
black are new, all dark, built by one function from the tint below so
the mapping is not copied five times. System is not a palette.

### What Paseo calls a theme

`THEME_OPTIONS` (`styles/theme.ts:783-832`), in this order: `light`,
`dark`, `auto`, `zinc`, `midnight`, `claude`, `ghostty`, `pureBlack`.
Labels (`i18n/resources/en.ts:2652-2659`): Light, Dark, Zinc, Midnight,
Claude, Ghostty, Pure black, System. Korean
(`i18n/resources/ko.ts:2540-2547`) keeps Zinc, Midnight, Claude and
Ghostty in Latin and says 순수 검정 for Pure black. The swatches, which
are the picker's chips and not the accent, are `#ffffff`, `#2D8B62`,
`#808080`, `#4A6BA8`, `#D97757`, `#8caaee`, `#000000` (no swatch for
`auto`). Default preference `auto` (`hooks/use-settings/storage.ts:53`,
stored at `:142`). A stored `plugin`, or any unknown name, is read back
as `auto` (`storage.ts:220`, `.catch`). Plugin themes
(`theme.ts:834-838`, `appearance/provider.tsx:40-45`) are not in this
version: there is no plugin host. A stored plugin id falls through the
existing lenient read to System.

`auto` calls `setAdaptiveThemes(true)` and returns without `setTheme`
(`provider.tsx:50-52`). The registry is configured with
`adaptiveThemes: true` (`styles/unistyles.ts:14`). A named theme calls
`setAdaptiveThemes(false)` and `setTheme` with that option's
`unistylesName` (`provider.tsx:55-56`); `light` and `dark` are the
names on the light and default-dark options (`theme.ts:783-800`).
Which theme name the Unistyles library itself picks while adaptive
mode is on was not read in that library. What this app does is the
part that was read: System resolves only to the Light or Dark palette,
never to a variant, using the window appearance (`scheme_of`).

### Each theme's tint

Every dark tint leaves `foreground` and `ring` unset, so the builder
fills them: foreground `#fafafa`, ring `#d4d4d8` (`theme.ts:391-392`).
Only Zinc sets `accentForeground` (`#18181b`, `:494`); every other dark
accent foreground is the builder's `#ffffff` (`:415`). Light sets its
own.

Light (`theme.ts:323-345`):

| Token | Value |
| --- | --- |
| surface0 | `#ffffff` |
| surface1 | `#fafafa` |
| surface2 | `#f4f4f5` |
| surface3 | `#e4e4e7` |
| surface4 | `#d4d4d8` |
| surfaceDiffEmpty | `#f6f6f6` |
| surfaceSidebar | `#f4f4f5` |
| foreground | `#1a1a1e` |
| foregroundMuted | `#71717a` |
| foregroundExtraMuted | `#a1a1aa` |
| border | `#e4e4e7` |
| borderAccent | `#ececf1` |
| accent | `#20744A` |
| accentBright | `#239956` |
| accentForeground | `#ffffff` |
| primary | `#18181b` |
| primaryForeground | `#fafafa` |
| destructive | `#b04138` |
| terminalBlack | `#1a1a1e` |
| terminalBrightBlack | `#3f3f46` |
| ring | `#18181b` |

Dark, the default (`theme.ts:460-477`):

| Token | Value |
| --- | --- |
| surface0 | `#181B1A` |
| surface1 | `#1E2120` |
| surface2 | `#272A29` |
| surface3 | `#434645` |
| surface4 | `#595B5B` |
| surfaceDiffEmpty | `#252827` |
| surfaceSidebar | `#141716` |
| foregroundMuted | `#A1A5A4` |
| foregroundExtraMuted | `#717574` |
| border | `#252B2A` |
| borderAccent | `#2F3534` |
| accent | `#20744A` |
| accentBright | `#7ccba0` |
| destructive | `#c64f43` |
| terminalBlack | `#141716` |
| terminalBrightBlack | `#434645` |

Zinc (`theme.ts:480-498`):

| Token | Value |
| --- | --- |
| surface0 | `#18181b` |
| surface1 | `#1f1f22` |
| surface2 | `#27272a` |
| surface3 | `#3f3f46` |
| surface4 | `#52525b` |
| surfaceDiffEmpty | `#242427` |
| surfaceSidebar | `#131316` |
| foregroundMuted | `#a1a1aa` |
| foregroundExtraMuted | `#71717a` |
| border | `#27272a` |
| borderAccent | `#303036` |
| accent | `#e4e4e7` |
| accentBright | `#fafafa` |
| accentForeground | `#18181b` |
| destructive | `#c44a4a` |
| terminalBlack | `#131316` |
| terminalBrightBlack | `#3f3f46` |

Midnight (`theme.ts:501-518`):

| Token | Value |
| --- | --- |
| surface0 | `#161820` |
| surface1 | `#1c1e27` |
| surface2 | `#252731` |
| surface3 | `#3c3e4c` |
| surface4 | `#535564` |
| surfaceDiffEmpty | `#222430` |
| surfaceSidebar | `#121420` |
| foregroundMuted | `#9a9db0` |
| foregroundExtraMuted | `#6b6e82` |
| border | `#242636` |
| borderAccent | `#2e3040` |
| accent | `#3b6fcf` |
| accentBright | `#7eaaeb` |
| destructive | `#c44a52` |
| terminalBlack | `#121420` |
| terminalBrightBlack | `#3c3e4c` |

Claude (`theme.ts:521-538`):

| Token | Value |
| --- | --- |
| surface0 | `#1f1f1e` |
| surface1 | `#262523` |
| surface2 | `#2f2d2b` |
| surface3 | `#4a4745` |
| surface4 | `#605d5b` |
| surfaceDiffEmpty | `#2a2826` |
| surfaceSidebar | `#1a1918` |
| foregroundMuted | `#ada9a5` |
| foregroundExtraMuted | `#78746f` |
| border | `#2c2a27` |
| borderAccent | `#36332f` |
| accent | `#d97757` |
| accentBright | `#e89a7f` |
| destructive | `#cf513e` |
| terminalBlack | `#1a1918` |
| terminalBrightBlack | `#4a4745` |

Ghostty (`theme.ts:541-558`):

| Token | Value |
| --- | --- |
| surface0 | `#282c34` |
| surface1 | `#2f333d` |
| surface2 | `#383c48` |
| surface3 | `#4a4f5e` |
| surface4 | `#5b6175` |
| surfaceDiffEmpty | `#323643` |
| surfaceSidebar | `#21252d` |
| foregroundMuted | `#c8ccd8` |
| foregroundExtraMuted | `#a0a4b2` |
| border | `#353a47` |
| borderAccent | `#3f4454` |
| accent | `#3570e0` |
| accentBright | `#b4d0fc` |
| destructive | `#c44a55` |
| terminalBlack | `#21252d` |
| terminalBrightBlack | `#4a4f5e` |

Pure black (`theme.ts:723-740`):

| Token | Value |
| --- | --- |
| surface0 | `#000000` |
| surface1 | `#0a0a0a` |
| surface2 | `#111111` |
| surface3 | `#202020` |
| surface4 | `#2d2d2d` |
| surfaceDiffEmpty | `#0c0c0c` |
| surfaceSidebar | `#000000` |
| foregroundMuted | `#a1a1aa` |
| foregroundExtraMuted | `#71717a` |
| border | `#1c1c1c` |
| borderAccent | `#242424` |
| accent | `#20744A` |
| accentBright | `#7ccba0` |
| destructive | `#c44a4a` |
| terminalBlack | `#595959` |
| terminalBrightBlack | `#8a8a8a` |

Shared by every dark theme, from the builder rather than the tint:
foreground `#fafafa`, ring `#d4d4d8`, accent foreground `#ffffff` except
Zinc, destructive foreground `#ffffff`, sidebar hover = surface1,
sidebar selected = surface2, workspace = surface1, popover = surface2,
primary = foreground, primary foreground = surface0, secondary, muted
and input = surface2, interaction highlight `rgba(255, 255, 255, 0.08)`
(`theme.ts:401-433`). Light's builder (`:270-302`): sidebar hover =
surface3, sidebar selected = surface4, workspace and popover =
surface0, destructive foreground = surface0 (`#ffffff`), interaction
highlight `rgba(0, 0, 0, 0.06)`.

### Mapping onto the kit's tokens

`Neutrals` keeps its fields. For Zinc, Midnight, Claude, Ghostty and
Pure black, each field is that theme's tint, with the shared dark
builder values above. `interaction_highlight` is the same value the
dark palette already stores (`rgba(0xffff_ff14)`, the 0.08 alpha the
dark neutrals use), not a new byte. `destructive_foreground` is white.
`accent` and `accent_foreground` are the theme's, so Zinc's text on the
accent is `#18181b`, except where the tint's accent is Paseo's green
`#20744A`. That green is Light (`:336`), the default dark (`:472`) and
Pure black (`:735`). All three use the mark's colour `BRAND`
(`#1A5868`) with white on it. The kit already records the accent as
the mark's colour, and the substitution applies wherever Paseo uses
that same green, not only on the two palettes that already ship.
Zinc, Midnight, Claude and Ghostty are not that green, so their
accents stay the tint's.

Light and Dark `Neutrals` other than that accent are not rewritten.
Paseo's `#20744A` for the three green themes is recorded above and
not applied. The mark stays `BRAND` on light and the primary
text colour on dark (`kit/src/widgets.rs`, `mark`): `#1A5868` on the
dark surface0 `#181B1A` is only 2.18:1, which is why the mark is not
drawn in the brand colour there.

Page tokens of Light and Dark stay the bytes in `Tokens::light` and
`Tokens::dark`. Page tokens of a new theme, all of which are dark:

| Kit field | Source |
| --- | --- |
| `bg_shell` | `surfaceSidebar` |
| `bg_base` | `surface0` |
| `bg_surface` | `surface1` (Paseo's workspace and the composer's card) |
| `bg_elevated` | popover, which is `surface2` |
| `bg_well` | `input`, which is `surface2` |
| `border_subtle` | `border` |
| `border_strong` | `borderAccent` |
| `text_primary` | foreground `#fafafa` |
| `text_secondary` | `foregroundMuted` |
| `text_tertiary` | `foregroundExtraMuted` |
| `accent` (the page's ink, not `neutral.accent`) | primary, which is foreground |
| `ring` | `#d4d4d8` |
| `danger` | the theme's `destructive` |
| `btn_primary_bg` | primary, `#fafafa` |
| `btn_primary_fg` | primary foreground, `surface0` |
| `btn_secondary_bg` | `surface2` |
| `btn_secondary_bg_hover` | `surface3` (the next step; Paseo has no hover colour of its own) |
| `bg_hover`, `bg_active`, `border_faint`, `text_disabled`, `st_building`, `st_validating`, `st_review`, `st_ready`, `st_needs_you` | the existing Dark page values, unchanged |

`scheme` is `Dark` for every new palette and `Light` only for Light.

Status stays `Status::of(scheme)`. Paseo spreads one dark status set
into every dark theme (`theme.ts:437-439`), so a variant does not get
its own. Code colours stay `CodeTokens::of(scheme)`, the page's. A
syntax theme is a separate preference (below) and is not folded into
the palette.

### Tokens Paseo has that the kit does not get

Listed so they are not added under a new name:

- `surfaceDiffEmpty`, `accentBright`, `success` (it aliases `accent`),
  `successForeground`, and the legacy aliases `background`, `popover`,
  `secondary`, `muted`, `input`, `accentBorder` (the mapping above uses
  them only to fill a token that already exists).
- `surfaceSidebarHover`, `surfaceSidebarSelected`, `surfaceWorkspace`:
  aliases of `surface1`/`surface2`/`surface3`/`surface4`/`surface0`, as
  the builders define. No new field.
- Diff text `#15803d` / `#b91c1c` (light, `:117-118`) and `#4ade80` /
  `#ef4444` (dark, `:122-123`). The kit's diff fields are the page's
  translucent line backgrounds, a different job.
- `statusMerged` (`#7347af` light `:151`, `#a890d5` dark `:159`) and
  the status dots other than the running dot the kit already has
  (`:206-209`, `:214-217`). The running dot stays the scheme's.
- The terminal object (`:309-319`, `:441-451`) and both ANSI sets
  (`:244-259`, `:373-388`).
- `baseColors` (`:4-108`), shadows (dark `:682-701`, light `:744-763`),
  spacing, radii, icon sizes. The kit keeps its own spacing and radii.
- Syntax themes (`packages/highlight/src/themes.ts:10-33`): `github`,
  `catppuccin`, `dracula`, `tokyo-night`, `one`, `nord`, `gruvbox`,
  `solarized`, default `one` (`storage.ts:154`). Independent of the
  app theme except that a theme with two palettes follows the
  scheme. Paseo's eight ids and the default `one` are the value to
take. This ADR does not decide to differ. The coordinator routes
that preference; until it lands, `CodeTokens` stays the page's two
schemes so a screen is not left without a colour.
- Plugin themes.

### Contrast

Method: WCAG 2.1 relative luminance on the authored sRGB hex, not on
GPUI's Hsla. A channel byte `c` (0–255) becomes `c/255`; if that is at
most 0.04045 it is divided by 12.92, otherwise
`((c+0.055)/1.055)^2.4`. `L = 0.2126 R + 0.7152 G + 0.0722 B`. The
ratio is `(L_lighter + 0.05) / (L_darker + 0.05)`. The tables are that
ratio rounded half up to two decimals, for reading. Every pair below is
opaque. A ratio of 5.897 rounds to 5.90 and would fail a floor of 5.90,
so the test's floor is the unrounded ratio truncated to two decimals.
The test measures the hex bytes the palette is built from, not Hsla.

Body (`foreground`), muted (`foregroundMuted`) and extra
(`foregroundExtraMuted`) on each kit surface. Light's foreground is
`#1a1a1e`; every dark theme's is `#fafafa`.

| Theme | Surface | Body | Muted | Extra |
| --- | --- | --- | --- | --- |
| Light | surface0 `#ffffff` | 17.35 | 4.83 | 2.56 |
| Light | surface1 `#fafafa` | 16.62 | 4.63 | 2.46 |
| Light | surface2 `#f4f4f5` | 15.78 | 4.40 | 2.33 |
| Light | surface3 `#e4e4e7` | 13.67 | 3.81 | 2.02 |
| Light | surface4 `#d4d4d8` | 11.74 | 3.27 | 1.73 |
| Light | sidebar `#f4f4f5` | 15.78 | 4.40 | 2.33 |
| Dark | surface0 `#181B1A` | 16.62 | 6.97 | 3.72 |
| Dark | surface1 `#1E2120` | 15.55 | 6.52 | 3.48 |
| Dark | surface2 `#272A29` | 13.88 | 5.82 | 3.10 |
| Dark | surface3 `#434645` | 9.14 | 3.83 | 2.04 |
| Dark | surface4 `#595B5B` | 6.55 | 2.75 | 1.46 |
| Dark | sidebar `#141716` | 17.29 | 7.25 | 3.87 |
| Zinc | surface0 `#18181b` | 16.97 | 6.91 | 3.67 |
| Zinc | surface1 `#1f1f22` | 15.75 | 6.41 | 3.40 |
| Zinc | surface2 `#27272a` | 14.27 | 5.81 | 3.08 |
| Zinc | surface3 `#3f3f46` | 10.01 | 4.07 | 2.16 |
| Zinc | surface4 `#52525b` | 7.41 | 3.02 | 1.60 |
| Zinc | sidebar `#131316` | 17.77 | 7.24 | 3.84 |
| Midnight | surface0 `#161820` | 16.97 | 6.60 | 3.52 |
| Midnight | surface1 `#1c1e27` | 15.91 | 6.19 | 3.30 |
| Midnight | surface2 `#252731` | 14.24 | 5.54 | 2.96 |
| Midnight | surface3 `#3c3e4c` | 10.13 | 3.94 | 2.10 |
| Midnight | surface4 `#535564` | 7.06 | 2.74 | 1.47 |
| Midnight | sidebar `#121420` | 17.55 | 6.82 | 3.64 |
| Claude | surface0 `#1f1f1e` | 15.80 | 7.07 | 3.56 |
| Claude | surface1 `#262523` | 14.67 | 6.56 | 3.30 |
| Claude | surface2 `#2f2d2b` | 13.14 | 5.88 | 2.96 |
| Claude | surface3 `#4a4745` | 8.83 | 3.95 | 1.99 |
| Claude | surface4 `#605d5b` | 6.26 | 2.80 | 1.41 |
| Claude | sidebar `#1a1918` | 16.82 | 7.52 | 3.78 |
| Ghostty | surface0 `#282c34` | 13.41 | 8.72 | 5.63 |
| Ghostty | surface1 `#2f333d` | 12.11 | 7.87 | 5.08 |
| Ghostty | surface2 `#383c48` | 10.54 | 6.86 | 4.43 |
| Ghostty | surface3 `#4a4f5e` | 7.82 | 5.09 | 3.29 |
| Ghostty | surface4 `#5b6175` | 5.90 | 3.84 | 2.48 |
| Ghostty | sidebar `#21252d` | 14.72 | 9.57 | 6.18 |
| Pure black | surface0 `#000000` | 20.12 | 8.19 | 4.35 |
| Pure black | surface1 `#0a0a0a` | 18.97 | 7.72 | 4.10 |
| Pure black | surface2 `#111111` | 18.09 | 7.37 | 3.91 |
| Pure black | surface3 `#202020` | 15.61 | 6.36 | 3.37 |
| Pure black | surface4 `#2d2d2d` | 13.19 | 5.37 | 2.85 |
| Pure black | sidebar `#000000` | 20.12 | 8.19 | 4.35 |

Floors the palette test requires, from these measurements. They are
Paseo's own pairs, not a correction:

- Body text on each of the six surfaces is at least 5.89 (Ghostty on
  surface4 is 5.897, the lowest; the table shows 5.90). Every theme
  clears 4.5:1.
- Muted text on surface0 is at least 4.83 (Light is 4.833).
- Muted text on the sidebar is at least 4.39 (Light is 4.397; the
  table shows 4.40). That pair is under the 4.5 normal-text floor.
  It is kept; it is not darkened.
- Extra-muted is under 4.5 on most surfaces (Light on white is 2.56).
  No contrast floor: the test checks the hex. Paseo uses it for
  placeholders and the quietest chrome.
- Accent foreground on the accent fill: Light, Dark and Pure black,
  white on `#1A5868`, at least 7.95 (7.955). The same pair with
  Paseo's `#20744A` would be 5.74 and is not what those three use.
  Zinc 13.96. Midnight at least 4.81 (4.816; the table shows 4.82).
  Claude, white on `#d97757`, at least 3.12 (3.122), which is
  large-text only and is kept (Paseo's Ghostty comment claims its
  blue can carry white; Claude's orange was not given that treatment,
  and this ADR does not invent one). Ghostty 4.62.
- Destructive foreground (white) on destructive: Light at least 5.73
  (5.739; the table shows 5.74), Dark 4.56, Zinc at least 4.74
  (4.745; the table shows 4.75), Midnight 4.72, Claude at least 4.28
  (4.286; the table shows 4.29, under 4.5, kept), Ghostty 4.71,
  Pure black at least 4.74 (4.745; the table shows 4.75). Zinc and
  Pure black share the same destructive bytes and the same test floor.

### How system light and dark are chosen

The preference default stays System, which is Paseo's `auto`.

System resolves only to the Light palette or the Dark palette, never
to a variant. The mapping stays `scheme_of`: a light or vibrant-light
window is Light, a dark or vibrant-dark window is Dark. The app keeps
following the window (`observe_window_appearance`) for as long as the
preference is System, and applies the resolved palette in that turn.
Choosing Light, Dark or any variant stops following the system, as
Paseo turns adaptive mode off. `VESSEL_APP_THEME=light|dark` still
pins the run and the preference does not override it; any other value
of that variable still means System, as the entry point already does.

### Fonts, content size, and what is refused

Empty interface and code families mean the embedded stack, not
Paseo's. Paseo's empty family becomes `DEFAULT_UI_FONT_STACK` and
`DEFAULT_MONO_FONT_STACK` (`theme.ts:637-648`, applied in
`appearance/apply.ts`). The `web` branch of that `Platform.select`,
which `isWeb` names for a browser or Electron (`platform.ts:22-23`),
is `system-ui` and a mono stack that ends in `Courier New`. The
`default` branch is `normal` and `monospace`. Neither is used here,
which is the owner's rule against a generic family and a pixel-looking
fallback. The order is the chosen family if there is one, then Geist (Geist Mono
for code), then Pretendard. Pretendard is always last. No generic
family name is ever added.

A family a person sets is at most 200 characters
(`storage.ts:74`). Paseo rejects `;{}<>`, a control character, an
unfinished quote or escape, and `()[]/*` outside quotes
(`storage.ts:520` and the loop below it), and then its schema
replaces the rejection with empty (`:231`, `.catch("")`). The
replacement is a silent drop. An explicit change here is a typed
error that names the broken rule (empty after trim is not an error:
it means the embedded stack). A preference file this build cannot
read keeps the existing lenient default for that field, so one bad
field does not reject the document. That file edge is already how
every preference is loaded; it is not how a change is applied.

Content size is Paseo's chat and Markdown size: an absolute point
size, not scaled by the interface size (`appearance/apply.ts`,
`content` is set absolutely). Desktop Paseo is the web runtime
(`constants/platform.ts`: Electron is `web`, and `isNative` is false
there), so the default is 15 (`theme.ts` `FONT_SIZE.content`) and
the range is 10 to 21 (`storage.ts:64-70`). The React Native default
of 16 is not used: one preference, one default, and this app is the
desktop. A value outside the range on an explicit change is a typed
error. Paseo clamps (`parseClampedFontSize`); clamping is the silent
drop this type does not do. A file that holds an out-of-range size
loads the default, as the other sizes already do.

Interface size is 12–20, default 14, and code size is 10–18, default
12, in the preference that already shipped. Paseo's interface range
is 10–21, default 14 on the desktop (`storage.ts:57-63`,
`FONT_SIZE.base` is 14) and 15 on React Native; its code range is
9–22, default 12 (`storage.ts:71-73`). Content width here is three
steps; Paseo's is a pixel value from 600 to 4000, empty meaning 820
(`theme.ts:650`, `storage.ts:75-77`). These are not a decision to
differ. Paseo's ranges and the pixel width are the values to take.
The coordinator routes them; until that lands, the shipped fields
stay so a stored preference still loads.

The catalogue order follows `THEME_OPTIONS`: Light, Dark, System,
Zinc, Midnight, Claude, Ghostty, Pure black. The Appearance chooser
uses System, Light, Dark, Zinc, Midnight, Claude, Ghostty, Pure black:
`appearance/theme-picker.tsx:19-23` moves `auto` to the front because
it is the default. These are separate orders; the chooser's existing
System-first order stays. Stored names stay snake_case, so Pure black is
`pure_black` in the preference file (the file is this app's, not
Paseo's `pureBlack`). Existing `system`, `light` and `dark` do not
change, and neither do the words already shipped for them (Korean
밝게, 어둡게, 시스템). New words follow Paseo: Zinc, Midnight,
Claude, Ghostty in both languages, Pure black / 순수 검정.

Applying a change redraws in the same turn (`set_scheme` already
refreshes the windows). Content size and the families go through the
same refresh. Nothing waits for a restart.

### What the closed enum forces

`Theme` is matched in the tile picture
(`app/src/areas/settings/appearance.rs`), in `apply_theme`, and in
the theme's name (`view/src/text/settings.rs`). The command
palette's test lists the three themes it is given. Adding a variant
without those arms does not compile, and a `_` arm is not allowed.
The arms are the existing picture helper and the existing name
function, plus the words. That is not a new screen. Settings ›
Appearance itself stays the screen it is.

The app has one preference-to-palette adapter. System has no fixed
palette; it resolves through the existing window appearance mapping.
The apply guard compares the actual token set, since Dark, Zinc and
every other variant share `Scheme::Dark`. Comparing only the scheme
would silently keep the previous variant. The same adapter supplies
each named tile's picture; System keeps its split picture. The existing
run override remains outside that adapter and keeps its current meaning.

The command palette retains Vessel's direct preset entries, expanded to
eight. Its generic entry builder receives each typed choice with its
localized name, removing the three-name parallel array in `PaletteWords`.
Names come from Settings' existing `theme(Theme)` API; the app does not
guess a name from an index. This also makes the existing palette labels
use the same capitalization and Korean names as Appearance.

Cycling follows the catalogue rather than the System-first chooser:
`theme.ts:884-888`, called by `app/_layout.tsx:388-391`. The pure transition
lives on the preference type. Shortcut registration is the existing
shortcut registry's responsibility: `keyboard/keyboard-shortcuts.ts:1028-1050`
binds Cmd+Alt+T on Mac and Ctrl+Alt+T elsewhere, outside the command center;
the non-Mac binding is also disabled inside a terminal. A new palette
does not create an independent key listener.

The existing tile geometry and interface label tier stay. That label is
currently the native Body tier (14/20 at the default interface size),
whereas Paseo's `theme-picker.tsx:235-238` uses its small 12-point tier.
This is a retained chooser difference, not a new typography or layout
control. New labels follow the source; the existing Korean Light/Dark
labels remain as recorded above.

## What this does not reach

No browser, no HTTP listener, no helper process, no script at start.
The faces are already embedded in the kit; a chosen family is a name
checked before it is drawn, not a file fetched. Nothing here uses a
signal, systemd or launchd, so Windows is the same path as the other
desktops: the palette is data and the window appearance is the one
GPUI already reports. The contrast test is `cfg(test)` beside the
palette; it is not in a product build.

## Why this and not the others

Paseo's theme is a tint plus one shared builder. Copying that, onto
tokens that already have names, is the whole of the feature. New
names would be a second design. Repainting Light and Dark would
change the app that already ships, for an accent the kit has already
declined. The contrast floors are the measured pairs, including the
ones under 4.5, because correcting them would no longer be Paseo's
colour.

## Advantages, costs and effects

The five themes are data. One builder means a new field cannot be
filled in four themes and forgotten in the fifth, and the compiler
requires every token. Light and Dark do not move, so current
captures stay comparable.

Costs: two ids for one idea (the kit's palette id has no System; the
preference does). Claude's filled accent and destructive, and Light's
muted text on the sidebar, stay under 4.5:1. Variant themes do not
recolour syntax, status or the board's lanes. The Appearance tiles
gain five pictures only when the preference enum grows, by the
existing tile, not by a layout pass. Syntax themes, font ranges,
content width and plugin themes remain as they are, written down
here so they are not quietly invented later.
