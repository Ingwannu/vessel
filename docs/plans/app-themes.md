# App themes

Paseo's eight appearance themes, the content size and the two font
families, in the native kit and in `view::prefs`. No new screen.
ADR 0070 is the contract (the values, the source lines, the mapping,
the contrast, how System resolves, the font rule).

## What Paseo does

Read from Paseo 0.15.4.

- Eight catalogue choices, order Light, Dark, System (`auto`), Zinc, Midnight,
  Claude, Ghostty, Pure black (`packages/app/src/styles/theme.ts`,
  `THEME_OPTIONS`). Default `auto`
  (`hooks/use-settings/storage.ts`). Labels in
  `i18n/resources/en.ts` and `ko.ts`.
- The actual chooser moves System first (`appearance/theme-picker.tsx:19-23`):
  System, Light, Dark, Zinc, Midnight, Claude, Ghostty, Pure black.
  The catalogue's cycling order is not the tile order.
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

- Done: slice 1, the ADR and this plan. The contrast
  floors in the ADR are the unrounded ratios truncated to two
  decimals, not the rounded table, so the later test can pass on
  Paseo's own bytes.
- Done: slice 2's kit-only palette source at `c67925e5` passes the full
  native gate: 1,043 passed, zero failed, one existing ignored capture.
  V1 passed this producer's source and A13 native gate on 2026-11-15;
  the full ADR and consumer/first-use evidence remain pending.
- Done: the consumer at `fb711444` is integrated on lead `d30804b2`,
  preserves the fixed menu colour and passes the full native gate:
  1,181 passed, zero failed, one existing ignored capture. Its normal
  local app build also passed. V1 passed consumer source/native and the bounded final visual package.
- Done: the consumer's final private-display package has 332 assigned
  original images: the eight choices in both languages/four window sizes/
  four scales, actual saved-byte restart checks, live System portal events,
  actual palette input/catalogue and the existing marks example. The
  changed regions and original-derived inspection sheets were inspected;
  representative full originals and all mark/catalogue originals were opened.
- Done: V1 passed the bounded eight-choice source/native/visual package
  through `f14b5e07`; no additional blocker was found in the inspected scope.
- Pending: the coordinator's capture review, lead integration and latest
  combined-line gate. The separate shortcut registration still has its
  owner's input-boundary correction pending and is not in this binary.
  ADR 0070 remains proposed for the remaining parity/integration obligations.
- Next: hand over the bounded disposition and retain the immutable evidence.
  No source changed after the native pass, so no unchanged whole gate or
  new capture matrix is repeated. Do not repaint Light or Dark, add an
  absent token, or claim untested OS/GPU/IME behaviour.

### Resumption, 2026-11-15

`56a7cbd3` was merged by fast-forward from the saved artwork branch,
with no conflicts. It already contains the saved optical/fitted changes
and the subsequent official vector rendering, current Gemini spark and
muted sidebar marks. Their earlier partial checks are historical evidence,
not a new gate for this head. No further vendor mark is added here.

The current task is themes only. Font families, content size, font ranges,
content width and syntax preferences described above remain future work;
this resumption does not implement them. ADR 0070 stays proposed. Its
catalogue/picker distinction above was checked against the actual chooser.

First implementation slice: a kit-only `Palette` id and five additional
palettes, with one tint-to-token builder, authored-byte and contrast tests.
`Tokens::of(Scheme)` and the existing Light/Dark constructor bodies stay
unchanged. No crate edge or dependency is added. The next slice extends the
existing stored `Theme`, its labels and the single app adapter; Appearance
continues to use the existing tiles and the real chooser's System-first order.

Validation on the development machine is limited to the noncompiling
format, size, boundary and token checks, followed by narrow compiling
checks in the short lane. The full native gate must run on the separate
build machine when its command is provided; no local whole gate is queued.
Captures will use an isolated authenticated display at scales 1, 1.25, 1.5
and 2. No capture or runtime switch is claimed. The completed source gate
is recorded below separately from those pending checks.

The independent authored-byte calculation also caught one inherited floor
wording error: Pure black's white-on-destructive is 4.745176456907, the
same pair as Zinc. The table's rounded 4.75 is retained; its test floor
is the same truncated 4.74 as Zinc. No colour or test tolerance changes.

### Palette source checkpoint

`kit/src/theme/palette.rs` owns the concrete seven-entry `Palette`,
`Palette::scheme` and `Tokens::palette`. Its private `tint.rs` contains
the five additional authored tables; one builder maps them to both
neutral and legacy page tokens. `Scheme` remains two-valued. The kit
still has no view dependency, and the app is not wired to a new palette.

Four tests beside that builder check both existing entries against their
full scheme token sets, every authored neutral before contrast is measured,
the measured contrast floors for all seven palettes, and the variants'
page mapping/shared alpha and lane colours. The contrast calculation uses
the authored sRGB bytes after asserting that the built neutral values
equal those bytes; it does not approximate bytes by converting Hsla back.

An independent source comparison matched five tints times thirteen fields
to the pinned reference, with only the declared Pure black green-to-BRAND
substitution. The four existing Light/Dark constructor bodies in
`theme.rs` remain byte-identical to `56a7cbd3`.

Fast native format, size, boundary and token checks pass: 513 files,
15 existing size exceptions, four crate edges, seven owned external crates,
two name rules, and no new token literals. The first narrow Clippy found
one large-by-value argument; the builder now borrows its baseline scale.
The changed-source narrow all-target kit Clippy passed. Filtered palette
tests also passed: four passed, zero failed, 109 filtered out, after a cold
build. The source hashes before and after both checks match `bf9b6ab8`.

A final M4 inspection reduced the mapping test from 71 lines to 43 by
pairing each actual/expected colour directly instead of repeating two
parallel arrays. An independent comparison confirms the same 23 pairs
in the same order; no expectation or product code changes. The first
remote gate, still queued before any step, was cancelled for this correction
and is not a pass. The corrected test source then passed its own narrow
and full native checks as recorded below. Captures and runtime remain
unverified; there is no consumer-ready or full-feature approval.

### Palette gate, 2026-11-15

Source commits: `bf9b6ab8` (palette producer), `c67925e5` (same test pairs,
within the function budget). The exact full-gate commit is
`c67925e544734e3aeeaa9d0ffc858ce756908d22`.

Narrow run `themes-narrow-20261115T023047Z-c67925e5` passed both selected
steps: font inputs and all-target Clippy. The full native run
`themes-20261115T023316Z-c67925e5` then passed every planned step, with
no skip counted as a pass:

| Step | Exit | Seconds |
| --- | --- | --- |
| native-fonts | 0 | 9 |
| native-fmt | 0 | 2 |
| native-quality | 0 | 1 |
| native-clippy | 0 | 49 |
| native-build | 0 | 72 |
| native-test | 0 | 54 |

The whole test total is 1,043 passed, zero failed, one ignored. The ignored
case is the pre-existing `pairing_browser_words_capture`, which requires
a private display, configuration and fake-fleet offer; it was not executed.
All four new palette tests passed in this final source. The changed kit
and app were actually compiled. All twelve font/licence input files match
the local inputs by SHA-256. The root crates and web application were not
changed and no root gate is claimed.

The run's `steps.log`, `steps.tsv`, individual logs and their hashes are
retained with its run id. The first cancelled run and earlier narrow lint
failure remain separate evidence. V1 passed the producer source and this
gate; ADR 0070 as a whole remains proposed. There is no
capture folder yet: this producer is not applied by the app, and neither
new-theme selection, same-scheme redraw, persistence, System following nor
fractional-scale artwork is certified by this gate. Those are the next
consumer slice's checks. No fleet, display, permission or ownership change,
push or release was performed for this producer.

### Consumer contract before implementation

Reuse `Theme`, `appearance.theme`, `PreferenceChange::Theme` and the
existing file store. Add the five named variants without new fields,
defaults, wire calls or a new preference type. `Theme::ALL` has the actual
System-first chooser order; `Theme::next` has the separate source catalogue
order, including System to Zinc. The serde names stay snake_case. File
loading retains its per-field lenient fallback; an explicit change still
uses the closed enum.

The single app adapter supplies fixed palettes to both the apply path and
tile pictures. System alone resolves from window appearance. Applying a
preference compares actual tokens, so a Dark-to-Zinc switch refreshes even
though the scheme stays Dark. The component layer uses the same setter.
The existing override and the split System preview stay. The retained
14-point tile label differs from Paseo's 12-point label, recorded in ADR
0070; no new tile or preview control is introduced.

The palette builder will take typed choices paired with localized labels,
with no fixed three-entry capacity or second name table. The two existing
generic palette fixtures retain their original three fake choices and
assertions through that input signature; an additional case covers the
real eight-entry theme registry. Settings owns their localized names.

The shortcut registry owns `theme.cycle` registration. Its source is
Paseo `keyboard/keyboard-shortcuts.ts:1028-1050`, with callback routing in
`hooks/use-keyboard-shortcuts.ts:157` and `_layout.tsx:388-391`. Its existing
owner has been given the typed transition contract; no second key listener
is added here. Runtime cycling remains pending until that route exists.

Validation: pure ordering/cycle/serde/fallback/name tests beside the
preference, actual preference-file round trips, adapter/System tests and
same-scheme global/component token changes. The full native gate runs
remotely. The app build and authenticated display captures run locally;
saved selection, reload, same-Dark changes and all four scales require
their own inspected evidence. None is claimed at this contract commit.

### Consumer source and integration checkpoint

The eight-entry preference, canonical Settings names, named palette-entry
input, sole app adapter and actual-token guard are written. Tests cover
chooser/cycle order, ids, lenient compatibility, all eight labels, the
existing atomic preference store, System resolution, dark-to-dark updates
and component-layer colours. The two generic palette test fixtures keep
their original choices, labels and assertions through the new named input.
Fast format, size, boundary and token checks passed before integration;
the compiling checks and inspected runtime evidence are recorded below.

The fixed menu success colour is imported from the existing token-only
source of `ef91b954`: `theme::palette::GREEN_500`, unchanged in name, type
and value. There is one definition in the now-public palette module;
`Palette` keeps its original re-export and builder. The tint tables stay
private. No second renderer or duplicate fixed-colour literal is added,
and the colour is independent of a named palette's accent. The typed menu
itself is not imported or redrawn by this work.

Integration moves onto the current lead before the consumer gate. The
unmerged theme history will be preserved under a backup reference and its
net theme change submitted without the intermediate `wip:` subject in the
lead's merge range. Earlier source/gate hashes remain historical evidence,
not a pass for this changed base or the consumer. Both document texts and
the lead's existing production behaviour are preserved on conflict.


### Consumer native gate, 2026-11-15

The integrated producer/consumer is `4f754b4b`, followed by the isolated
real-registry test fix `fb711444338140ffa5a9a51ce7476daa424a5a9d`. The
unmerged history is preserved under the integration safety reference;
the submitted range begins at lead `d30804b2` and contains no `wip:` subject.
Both the lead's host-rename README paragraph and the palette paragraph
were retained. The fixed `GREEN_500` compatibility described above is
part of this exact gated source.

Full A13 run `themes-20261115T035513Z-fb711444` passed all six planned
native steps without a skip: fonts, format, quality, Clippy, build and
test. Their exit codes are all zero. The ten raw test summaries total
1,181 passed, zero failed and one ignored. The ignored test is still
`pairing_browser_words_capture`, not a runtime pass. The changed view,
kit, link and app were actually compiled. Evidence is retained at
`/tmp/work/gk-k-themes/consumer-native-fb711444`, with independent totals
and log SHA-256 values in `consumer-gate-evidence.json` beside it.

The first integrated narrow check failed because the new real-theme test
called an old fake-theme-specific helper. The correction uses the real
entry iterator directly; the old fixtures, public API and expectations
are unchanged. The corrected narrow run
`themes-narrow-20261115T034800Z-fb711444` and the full run above both passed.
No root/web change or root gate is claimed.

A normal local app build of this same source also passed. Its binary
SHA-256 is `83b8c195ed54705d80f04d8ce1945acf21be33c72f6a43e802509c49852639d4`.
Authenticated private-display captures are in progress against an isolated
loopback fake fleet. The first calibration used the wrong scratch HOME,
so its tilde-relative credentials could not resolve; that image is
excluded. Correcting only the capture environment restored both fake
connections. Initial stale frames taken before a click painted are also
excluded. Final captures must wait for the saved preference and the actual
painted colour, and be inspected before any UI claim.

This gate does not certify live selection, restart, System following,
fractional-scale marks or the separate shortcut registration. S2 owns the
latter through the existing registry. ADR 0070 and the complete feature
remain proposed pending those scoped checks and V1 review. No new tile,
font/content/syntax setting, production service, ownership change, push or
release was introduced.


### Consumer source disposition

V1 independently passed the source and native gate for exact `fb711444`
on 2026-11-15. The review checked all 727 pinned source inputs, the four
unchanged legacy constructor bodies against both baselines, the accepted
tint bytes, the one public fixed menu colour, preference compatibility,
the sole app adapter and the actual-token/component refresh path.
The retained `steps.log` SHA-256 is
`50667e2ef01b1045aa8486b8324b3a7535cae90688c5232325b01deb987c67a6`.

That verdict explicitly excludes screen approval, live saving/restart,
System events, same-dark visual changes, palette input, fractional marks,
real OS/GPU behaviour and general accessibility approval. The shortcut
registration is a separate source range, not part of this binary.
The final runtime package and its inspected evidence are still pending.


### Final consumer runtime package, 2026-11-15

The runtime source and both local binaries remain pinned to `fb711444`.
Later commits change this plan only. There is one combined image/source/
gate assignment in the scratch evidence root, `theme-final-manifest.json`,
with a companion SHA-256 file, exact physical dimensions, scale, language,
UID and SHA-256 for every assigned original. Its evidence references pin
the source inputs, twelve matching font/licence inputs, native gate,
inspection records and process cleanup. No screenshot or credential file
is committed.

| Assigned evidence | Original images | What it verifies |
| --- | ---: | --- |
| Appearance choices | 256 | Every choice selected, stored enum and actual background RGB; English/Korean, 1280×800/390×800/800×600/1024×700, scales 1/1.25/1.5/2 |
| Fresh-process restores | 24 | All eight choices saved through the UI, exact preference bytes unchanged after reopening, nonblank paint and fake bridge loaded; selected tile captured without clicking it again |
| Private System portal | 20 | At all four scales, System light→dark through the real GPUI portal event, fixed Midnight across the opposite event, then System light again |
| Command palette input | 22 | Five new themes searched and chosen with Enter in English at 1280 and Korean at 390; real saved enum, painted colour and selected tile |
| Command palette catalogue | 2 | All eight localized entries visible, System first and Pure black last, current choice checked |
| Existing mark example | 8 | Five existing official assets at 14/16/20 points on off-grid origins, both schemes and all four scales |

The complete Appearance matrix was already produced before the review
policy changed. The final input/catalogue checks use the two relevant
language/window cases; they do not add another matrix for the unchanged
popup layout. Inspection used all 32 eight-state theme-grid sheets, the
restart/System/input sheets, representative full originals, and every
mark and catalogue original. No overlapping new tile label or clipped
chosen outline was found in those regions. Lower sections retain the
existing vertical scroll; this is not a claim that every section fits in
one viewport.

Excluded attempts remain separate. Eight original mark shots were black
first frames. The earlier restart set also included a black Pure black
startup frame: its expected background alone cannot prove the UI has
painted. Final restart checks require nonblank UI and the fake bridge,
and compare the actual saved file bytes across processes. An initial
palette set had stale empty-query frames even though its subsequent
selection actions succeeded; final query images wait for a changed,
stable frame and were read directly. Diagnostic OCR mistakes `Zinc` for
`Zine` and the Korean prefix for Latin letters, so it is not an automatic
semantic pass. The Korean Pure black search uses the one-character
prefix 순, with the full localized command label visible; no IME
composition test is claimed. An OCR timeout in the catalogue harness is
retained as a failed harness attempt, not a product or native-gate result.

Limits: the System test uses an isolated desktop-portal fixture and
software rendering, not an installed desktop or hardware GPU. No Mac,
general accessibility, 12-point mark or complete app-surface certification
is claimed. The four legacy constructor bodies are byte-identical to the
baselines; an entire baseline-screen pixel comparison was not performed.
The existing Body14 tile label versus Paseo's small12, Korean Light/Dark
names and Pure black brand-accent substitution remain the stated ADR
differences. No new vendor artwork is introduced.

The separate shortcut registration is not merged into this source. Its
owner is correcting the host's CharacterInput/AltGr boundary before the
matcher; the current terminal-context seam remains the lead's named false
stub until an actual terminal API exists. Neither the candidate's gate
nor these screenshots certify that input route. Its integration and the
complete parity/ADR verdict remain external review obligations.

All runtime work ran as UID 1000. Preferences and display authority files
were private; no production ownership or permission changed. The two
fake-node roots and their recorded descendants (22 processes total) were
stopped only after matching PID, start time, UID and executable. All 206
recorded capture/private-bus identities were checked; no owned survivor
remained. No real agent, preview node, push or release was used. Final
source/native pass and final visual approval remain distinct.


### Bounded final review disposition, 2026-11-15

V1 passed the bounded native eight-choice theme package for
`d30804b2..f14b5e07`. Executable source remains exact `fb711444`; only this
plan changed afterward. The existing six-step native result, 1,181 passed,
zero failed and one ignored, is retained without an unchanged rerun.

The reviewer independently matched the combined manifest SHA-256
`e17dfd02ce78e0ccbe848c9cd15b1267b13edc8e8cb0e8bbc975e9d64ea750de`,
all 332 original image hashes/dimensions, nine proof files and the 32 grid
sheet hashes. The reviewer personally opened 43 risk-selected originals
and 13 eight-state derived grid sheets, not all 332 originals. Their
inspection covered the eight-choice appearance and borders, both
languages/window cases and four scales, all eight restored selections
including nonblank Dark/Pure black startups, the private System/fixed
transition, all ten final queries, both catalogues and all eight mark
originals. No new blocker was found. This closes this package's pending
visual scope; the audit is retained with the review evidence.

The verdict does not include the separate `theme.cycle` input fix,
AltGr/IME, a real terminal context, installed desktop OS or hardware GPU,
general AA/accessibility, complete baseline-frame equality, or the latest
combined line. The mark proof remains five known assets at 14/16/20, not
12 or every product surface. Excluded failed capture attempts remain
excluded. Runtime/cleanup scripts and events are author execution
evidence; the reviewer did not independently execute live cleanup or
portal transitions. Coordinator capture review, lead integration and the
combined gate remain separate obligations. No new source, gate or capture
matrix is needed solely to record this disposition.
