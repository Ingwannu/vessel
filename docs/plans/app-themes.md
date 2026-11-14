# App themes

Paseo's eight appearance themes, the content size and the two font
families, in the native kit and in `view::prefs`. No new screen.
ADR 0070 is the contract (the values, the source lines, the mapping,
the contrast, how System resolves, the font rule).

## What Paseo does

Read from Paseo 0.15.4.

- Eight choices, order Light, Dark, System (`auto`), Zinc, Midnight,
  Claude, Ghostty, Pure black (`packages/app/src/styles/theme.ts`,
  `THEME_OPTIONS`). Default `auto`
  (`hooks/use-settings/storage.ts`). Labels in
  `i18n/resources/en.ts` and `ko.ts`.
- `auto` follows the operating system between the themes named
  `light` and `dark` only (`appearance/provider.tsx`,
  `styles/unistyles.ts`). A named theme does not follow the system.
  Variants are all dark tints of one builder. Plugin themes exist;
  this app has no plugin host (ADR 0070).
- Each tint's hex, and every derived token, is in ADR 0070 with its
  line. A token Paseo has and the kit lacks is listed there and is
  not added.
- Interface family and code family: empty means the platform stack,
  at most 200 characters, with a sanitiser that rejects CSS-breaking
  characters (`storage.ts`). This app does not use that stack (ADR
  0070): Pretendard stays last, and `Courier New` is not a fallback.
- Interface size 10–21, default 14 on desktop (Electron is the web
  runtime) and 15 on React Native. Content size 10–21, default 15
  on desktop and 16 on React Native, absolute, not scaled by the
  interface size (`appearance/apply.ts`). Code size 9–22, default
  12. Content width 600–4000, empty means 820. Syntax theme one of
  eight, default `one` (`packages/highlight/src/themes.ts`).
- An out-of-range size is clamped. An illegal family becomes empty.
  Both are silent. An explicit change here is a typed refusal
  instead (ADR 0070). The preference file stays lenient, as every
  other preference already is.

## What Vessel already has

- `kit/src/theme.rs`: Light and Dark. Neutrals are Paseo's light and
  default dark; the accent is the mark (`#1A5868`), on purpose.
  Page tokens are the page's. Not rewritten (ADR 0070).
- `kit/src/theme/status.rs`: Paseo's status text and tints, per
  scheme. One dark set covers every dark theme, as Paseo's builder
  does. Not duplicated per variant.
- `kit/src/code.rs`: the page's syntax colours, per scheme. Paseo's
  eight syntax themes (default `one`) are the value to take; the
  coordinator routes that preference. Until then the page's two
  schemes stay, so code is not left uncoloured.
- `kit/src/fonts.rs`: Geist, then Pretendard, embedded. Interface
  and code sizes scale the type steps. No content size, no chosen
  family.
- `view/src/prefs/appearance.rs`: `System` | `Light` | `Dark`,
  interface 12–20 default 14, code 10–18 default 12, content width
  in three steps. `Theme` is matched by the Appearance tiles, by
  `apply_theme`, and by the theme's name.
- System is applied at start and on `observe_window_appearance`.
  `VESSEL_APP_THEME` pins a run.

## Reuse

The palettes extend `Tokens` and `Neutrals`. They are not a new
crate and not a new token vocabulary. The five new themes are one
builder over a tint table, because five copies of the same struct
would be the same block five times. Light and Dark stay the
functions they are, so their bytes cannot drift.

No external palette crate: the values are the ones already cited,
and the standard library can hold a table of them.

Decision: modify the kit and `view::prefs`. Do not adopt a library.
Do not start a second theme type in the app.

## Slices

1. ADR 0070 and this plan. Docs only.
2. The palette id and the five palettes in the kit, Light and Dark
   unchanged. A test that every id builds every token, and that the
   contrast floors in ADR 0070 hold, measured from the hex. No
   screen, no preference change.
3. The preference: theme id (System plus the seven), content size
   10–21 default 15, interface family and code family as validated
   types. Paseo's desktop defaults. Applied in the same turn.
   Exhaustive matches gain arms (the existing picture helper and
   the existing name function); that is the compile, not a new
   layout. Words for the five: Paseo's, including 순수 검정.
   Interface range (Paseo 10–21), code range (9–22), content width
   (600–4000, empty means 820) and the eight syntax themes are
   Paseo's values to take. This track does not change the shipped
   fields. The coordinator routes them.
4. A kit example that draws one screenful of each palette, for
   captures on this track's own display, at the four scales. No
   hex outside the kit.

## Resume point

- Done: slice 1, the ADR and this plan. No code. The contrast
  floors in the ADR are the unrounded ratios truncated to two
  decimals, not the rounded table, so the later test can pass on
  Paseo's own bytes.
- Half-done: nothing.
- Next: slice 2, the kit palettes and the contrast test, behind the
  token names ADR 0070 maps. Do not repaint Light or Dark. Do not
  add a token the ADR lists as absent.
