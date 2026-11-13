# Vessel's native app (milestone 12)

The native app ADR 0035 decided on and ADR 0036 makes the one experience
on every surface: the same UI crate on the desktop (Linux, Windows,
macOS) and on a phone (Android through the community mobile layer; the
host project is `android/`, track M's), with the page's look, words and
behaviour. It is a client of its nodes as the `vessel` CLI is: the same
node book, token files and device credentials, reached through
`vessel-client` (directly, or through a node's relay over the secure
channel; SSH routes come later).

This is a workspace of its own, outside Vessel's: Vessel's gate does not
build it, and its tree (GPUI's) stays out of the main lock file.

## Crates, and what may depend on what

| Crate | Owns | Must not |
| --- | --- | --- |
| `view` (`vessel-app-view`) | what a screen means: the board's lanes and phrases, a conversation folded from its records, the permissions waiting; **the words** (`view/src/text/`, the legacy area tables; `view/locales/`, the staged Fluent catalog of ADR 0039) | use any UI toolkit; do I/O |
| `link` (`vessel-app-link`) | the nodes: the book, connecting, the session list, a session's stream, prompts, answers, pairing; the app's remembered choices (`prefs.rs`); a tokio runtime of its own, commands in and events out on bounded channels | use any UI toolkit |
| `kit` (`vessel-app-kit`) | Vessel's kit: the design system's tokens in both themes, the type scale with the faces embedded, the controls (button, choice, text field, Markdown), the window's chrome (title bar, window controls, resize edges), its glyphs | know about nodes or sessions |
| `app` (`vessel-app`) | the window, the routes, the areas (screens), the binary | name `gpui-component` or `gpui-platform` outside `main.rs`; hold a sentence of its own (words come from `view::Text`) |

```
app ──> kit ──> GPUI (gpui-pre 0.3.10) + gpui-component 0.8.1 (text field, Markdown only)
 │
 ├──> link ──> vessel-client, vessel-proto
 └──> view ──> vessel-proto
```

Nothing of GPUI's reaches `view` or `link` (ADR 0035, 0036): the browser
track compiles them to WebAssembly, and the layer under the kit can be
replaced without touching them.

Data flow: `link` lists every book node's sessions every two seconds (the
page's cadence) and sends a `Fleet`; opening a session follows its log
from the start and sends its records, which the shell folds in batches
into `view::Conversation`; the areas draw the result. Everything a person
does that reaches a node is a `link::Command`.

## The app's layout

```
app/src/
  main.rs          the window (the kit's chrome), language, theme
  shell.rs         the window's state: route, fleet, prefs, areas; link events dispatched; the frame
  routes.rs        THE registry: `Route`, what draws each, what arriving does, the sidebar's list
  chrome.rs        what is in the title bar (the mark and label; the update's status at the right)
  update.rs        the updater (track R's crate) on the shell: its state followed, check now,
                   automatic or only when asked, restart into a new build, "Updated to …" once
  areas/
    mod.rs         one `mod` line and one field of `Areas` per area
    common.rs      pieces several areas draw (a frame's title, its foot, a session's colour)
    draft/         the new chat: Paseo's composer card, its pickers (pickers.rs), the
                   step of Paseo's ladder the card's width allows (density.rs), the
                   features as controls or as Paseo's features sheet (features.rs)
    session.rs     a session: the conversation and the composer in Paseo's one chat
                   column (at most 820 wide, 16 from the edges), permissions, stop
    about.rs       About as Paseo's settings section: the version, the app's updates
                   (Paseo's words), automatic updates (a switch), the log file; and the
                   note shown once after an update
    pair.rs        adding a machine: Paseo's paste-link modal, then its host
                   confirmation, over the new chat
    board.rs, archive.rs, sidebar.rs
```

### Adding a screen (an area)

1. Its meaning in `view` (a module of its own, ported from the page's
   `web/src/data/...`, with the page's test cases as its tests: same
   inputs, same outputs; fixture files used by path, not copied).
2. Its node calls in `link`: a `Command` variant, an `Event` variant
   when it answers, the call in `State::handle`.
3. Its words in `view/src/text/<area>.rs`, a file of its own: a struct
   `<Area>Words` with one field per sentence, an `EN` and a `KO`
   constant of it (a sentence missing from one language does not
   compile), sentences with values as methods matching on the language,
   and `impl Lang { pub const fn <area>(self) -> &'static <Area>Words }`;
   one `mod` line in `view/src/text/mod.rs`. `Lang` is the one switch
   (the system's language or `VESSEL_APP_LANG`); the shell holds it
   (`shell.t` is the lead's `Text`; `shell.t.lang` is the switch for any
   area's words: `shell.t.lang.conversation()`).
4. Its screen in `app/src/areas/<area>.rs` or `app/src/areas/<area>/`:
   a state type (fields: `TextField`s, choices), an `impl Shell` block
   for what a person does there (methods on the shell, so a click reaches
   the link, the fleet and other areas), `pub(crate) fn render(shell,
   tokens, …) -> Div`, and `arrive` if arriving focuses or settles
   something.
5. Register it: one `mod` line and one field in `areas/mod.rs`, one
   variant and one arm in each match of `routes.rs`; `Route::NAV` if the
   sidebar lists it. An event from the link for it: one arm in
   `Shell::on_event` calling the area's method.

The Devices page keeps invitation text outside text fields: only an
explicit reveal projects it for up to thirty seconds or until expiry,
whichever comes first. Copy, focus loss and navigation mask it. Its pure
state and stale-response tests live in `view/src/devices/`.

### Adding a kit control

One file per control in `kit/src/` (`widgets.rs` holds the smallest),
drawn from `Tokens` and `Type` only, exported from `kit/src/lib.rs`. A
control another track could need (a menu, a dialog, a sheet, tabs, a
tooltip, a virtual list) is the lead's to write: ask through the
coordinator first, so two tracks do not write two.

A settings screen (About now, the settings track's sections later) is
built from `kit::settings`: Paseo's section with its small title, the
card, a row with its title and hint at the left and its control at the
right, and Paseo's switch; buttons are `kit::button` (accent) and
`kit::outline_button` (Paseo's outline).

The kit's `qr.rs` draws a supplied module matrix on a white tile in both
themes, with a four-module quiet zone. `view::devices::qr` encodes with
`qrcode` 0.14 (MIT OR Apache-2.0, default features off); neither layer
writes a matrix or a link. QR and text share the invitation reveal gate.

`kit::button_with_kind` is the filled entry: typed primary, secondary and
destructive kinds plus enabled/disabled state. Destructive fill/border use
`neutral.destructive`, text its separate `destructive_foreground`; hover
keeps the palette, pressed is 85% and disabled 50%. Disabled callers attach
no handler. The existing bool `button` delegates and stays until the lead
retires it after moving its callers.

`kit::breakpoint::TOUCH_TARGET` is the shared minimum touch target;
screens take gaps and insets from `kit::theme::spacing`.

`kit::assets::Glyph::CalendarClock` is the shared scheduling mark: an
original calendar and clock on the kit's 24-unit, 1.5-stroke line grid.
Navigation and empty states use this one registered vector.


Shared action glyphs also include `Trash2`, `ArchiveRestore`, `Minus`,
`GitBranchPlus`, `Commit` and `Undo2`; existing `Pencil`/`Archive` stay.
Each is registered once in `assets.rs` on the same 24-unit line grid.

`kit/badge.rs` supplies Paseo's StatusBadge for every area: typed success,
warning, error and muted variants; small and extra-small sizes; optional
leading content; semantic status tokens separate from board lane colours.
Labels remain whole and explanations belong below their row.

`kit/alert.rs` supplies Paseo's five notice variants and four density sizes,
with optional title, wrapping description and an indented action. It uses
existing semantic/neutral colours and palette blue 300 / amber 500 in
`theme/status.rs`; a notice has no focus or click behaviour. Pure logic
returns `view::alert::Variant`, mapped once by `app::badge::alert_variant`.
Its glyphs are the kit's own embedded drawings, as in `assets.rs`.

`kit/dialog.rs` keeps a fitting dialog at its natural size. When its
content exceeds the card's existing height limit, only the body scrolls;
the heading and footer actions retain their size and remain reachable.

`TextField::form_multiline` uses the same input state and form shell as
the kit's existing fields: Enter starts a line, Mod+Enter submits, and
after twelve lines the editor scrolls. Its marked-text regression also
exercises the composer; platform input-method support remains shared.

### Rules every screen follows

`kit::theme::spacing` contains Paseo's complete spacing scale, with its
keys in the constant names (`S0_5`, `S1`, …) and values in logical points.
Its exact-value test keeps every step comparable with `styles/theme.ts`.

- Words through `view/src/text/` only; English and Korean together.
- Colours, type, spacing and radii from the kit's tokens, never a literal
  in a screen. Pretendard is behind every face (the kit sets it); no
  generic family anywhere.
- An error belongs beside the control it is about; the frame's foot is
  for what belongs to no control.
- Every screen works from 360 points wide (the phone); targets are 44
  points on touch; nothing depends on hover (ADR 0036).
- Two widths, both named in the kit, each with its reason:
  - `kit::breakpoint::PHONE` (768): below it the sidebar is a sheet from
    the title bar's menu button, and the title bar's statuses are icons.
    ADR 0036's width, the page's too, so the sidebar folds at the same
    size on every surface.
  - `kit::picker::COMPACT` (600, Paseo's compact width,
    `constants/layout.ts`): below it controls are touch-sized (44), a
    picker's list and a dialog are sheets from the bottom, and the
    composer's card is tighter. Paseo's own width, so its controls change
    where Paseo's do.
  Between 600 and 768 the sidebar is a sheet and the controls are still
  pointer-sized: a narrow desktop window, where a mouse is still likely.
  `kit::breakpoint::INSPECTOR` (1280) folds the inspector, when there is
  one.
- No library's default look anywhere (the owner, on 0.0.0.0.3): fields
  are the kit's (`TextField`: a form field, or the composer's bare text
  in `composer_card`), pickers the kit's (`pill` and `combobox`),
  buttons the kit's; carets and marks are the kit's vector glyphs
  (`assets::Glyph`), never characters. Controls take their colours from
  `tokens.neutral` (Paseo's scale); the accent is the brand's.

## Why GPUI, and which

ADR 0035: the app is native, drawn as Zed draws its own; Zed's framework
is a reference and is not taken as it is. The kit is ours so the layer
under it can change. Underneath for now: the `gpui-pre` snapshot 0.3.10
(Zed's tree, republished weekly), with `gpui-component` 0.8.1, the newest
pair the community mobile layer also pins. Every problem met with it is
in `docs/plans/backend-m12-spike.md` (question 9). The toolchain is Rust
1.98.1 (`rust-toolchain.toml`), not Vessel's 1.93: the snapshot uses
`std::hint::cold_path`, stable since 1.95.

## Build, run, gate

```bash
cd spikes/native
(cd ../../web && npm ci)          # Pretendard comes from the page's packages
scripts/fonts.sh                  # the faces into assets/fonts (not committed)
cargo run -p vessel-app
```

A `.cargo/config.toml` beside this file (untracked, one per worktree)
sends build output to a directory of your own on the data disk with six
jobs; one build at a time.

Linux needs the X11 and Wayland development libraries GPUI links
(`libxkbcommon-dev`, `libwayland-dev`, `libxcb*-dev`, `libfontconfig-dev`)
and, to run, a Vulkan driver (Mesa's is enough).

The gate of this workspace (the main workspace's lints are copied into
its manifest), run from `spikes/native` with its pinned toolchain
(`rust-toolchain.toml`), the whole workspace, never a `-p` subset; every
track runs exactly this before it reports a slice:

```bash
cd spikes/native
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Environment:

- `VESSEL_CONFIG_DIR`: another configuration directory (its
  `vessel/nodes.toml` is the book; `vessel/app-prefs.json` the app's
  remembered choices), as the CLI reads it.
- `VESSEL_APP_LANG=ko|en`: the language (default: the system's).
- `VESSEL_APP_THEME=light|dark`: the scheme (default: the system's).
- `VESSEL_APP_SIZE=1280x800`: the window's size.
- `VESSEL_APP_TITLEBAR=system`: the system's title bar and frame instead
  of the app's own (the way out if the app's bar misbehaves on a
  platform).
- `VESSEL_APP_NO_UPDATE` (any value): no updates for this run.
- `VESSEL_APP_CONVERSATION=full`: conversations as the "Full" display
  setting shows them (every tool call a row of its own; an agent's log
  as one folded row) instead of collapsing tool calls, until the app
  reads that setting.
- `VESSEL_APP_SCROLL_BENCH=<entries>`: once the session shown holds that
  many entries, it scrolls itself for 600 frames and logs the median,
  p95 and longest frame.
- `VESSEL_BUILD_LABEL` (at build time): the label in the title bar and
  the sidebar's foot, `dev` without it. With `VESSEL_UPDATE_FEED` and
  `VESSEL_UPDATE_KEY` too (track R's `build.sh` sets them), the build
  updates itself; without them it never does.

## Trying it against fake nodes, and capturing a screen

```bash
# in the repository root: the node, the CLI and the fakes
cargo build -p vessel-cli -p vessel-node -p vessel-testkit --bins
# here: two nodes with fake agents and a book naming them
scripts/fleet.sh start /tmp/fleet <the main workspace's target>/debug
VESSEL_CONFIG_DIR=/tmp/fleet/config cargo run -p vessel-app
/tmp/fleet/pair.sh                # a pairing link for `laptop`
scripts/fleet.sh stop /tmp/fleet
```

The fake agent answers `plain` with an echo, `entries <n>` with `n`
entries, `wait` by waiting, `shapes` with a real conversation's shapes;
any other prompt runs a tool that asks for permission. The fleet's
Claude Code adapter offers what the real one sends
(`FAKE_ACP_OPTIONS=real`, `crates/vessel-testkit/src/bin/fake_acp/real.rs`):
sixteen models, subagents (`ocx-*`) with descriptions of several lines,
names longer than a list and Korean ones, an on/off option with no name.
Look at every picker with it before a build; `kit::picker`'s test lays
out rows of those shapes and fails if one grows past a line or runs into
the next.

Settings also has checked shapes (`scripts/settings-fixtures.py`): many
paired devices, long and Korean names, a never-seen and a revoked device;
a home bundle with instructions, skills and MCP names; a synthetic member
behind version zero with kept edits. These fixture reports do not run a
real member sync loop. The second machine has no paired devices.

Normal-build captures always mask the invitation. Revealed-state model
checks use inert, test-only invitations in `view/src/tests/devices.rs`;
no app feature can substitute a fake invitation in an installable build.

Screens are checked against the two `shapes` sessions the fleet makes on
`laptop`: a long thought streamed a few characters at a time (Korean
first, a syllable alone in a chunk), Markdown with a table, a 240-character code
line and Korean, a long path, a wide diff, a command on one long line
asking for permission with four answers, and 2,400 lines of output.
`shapes` has its permission answered (the output in the timeline),
`shapes-asking` is left asking. Every Claude Code session also opens
with the start-up log the real adapter writes before the first prompt
(ten `[session/create] …` lines, `FAKE_ACP_STARTUP_LOG`). `fleet.sh start`
fails if either `shapes` session lacks any of it (it reads them back
with `vessel log`), so a fake-acp built before these existed is caught:
rebuild it. A third session on `laptop`, `turns`, holds five turns (one
long prompt, one Korean; each tool's permission allowed): the outline of
prompts and read marks are checked against it. A fourth, `queued`,
runs a turn that waits until stopped (`wait`, which takes a steer) with
three prompts queued behind it (one long, one Korean, one short),
checked through `vessel session queue`: the queue above the composer is
checked against it. Two more, `ask questions` and `ask one`, are left
waiting on what Claude's adapter sends for its `AskUserQuestion` (three
questions, one choosing several and one in Korean, and one question):
the question card is checked against them. Three more, `permit plan`,
`permit codex` and `permit two`, are left waiting on permission requests
in the real adapters' shapes (Claude Code leaving plan mode with five
"always" answers, Codex's five answers, two at once): the permission card
is checked against them and the `shapes-asking` session's long command.

On a machine without a display: an Xvfb of your own on a free display
number, Mesa's software Vulkan, and ImageMagick's `import`:

```bash
Xvfb :140 -screen 0 1280x800x24 -nolisten tcp &
DISPLAY=:140 VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  VESSEL_CONFIG_DIR=/tmp/fleet/config VESSEL_APP_THEME=light <target>/debug/vessel-app &
sleep 15; DISPLAY=:140 xdotool mousemove 390 380 click 1   # drive it
DISPLAY=:140 import -window root shot.png
```

Pick a display nobody uses (`ls /tmp/.X11-unix`), stop what you started
by its PID. A phone-sized capture: `VESSEL_APP_SIZE=390x844` and an Xvfb
screen of that size.

## Translation engine (staged, ADR 0039)

`view::localization` sits beside the legacy tables. Its host build script
reads `view/locales/<tag>/<area>.ftl`, validates the Fluent AST, and generates
a typed API from English. The target library embeds the resources and does
no I/O. Prepare `Translator::get(locale)` before drawing: plain accessors
borrow retained fields, while messages with values are formatted when those
values change. The allocation-counting test covers warmed plain access.

`LANGUAGES` exposes each embedded tag, own name and completeness; the
checking locale `en-XA` is excluded. `resolve_language` applies an override,
a typed preference, or the system tag, with exact/base/English fallback.
The source API is English, Korean must be complete with the same variables,
and a contributed folder may be partial. Run the shared validator/report:

```bash
scripts/locale-coverage.sh
```

`view/locales/<tag>/formats.ftl` owns number separators, primary/secondary
sizes, ten digit characters, month/weekday names, whole date/time patterns
and the h12/h24 choice. Missing contributed settings/patterns use English.
The build validates settings and emits static numeric rules automatically;
adding a folder needs no separate generator command or Rust edit. Messages
call standard `NUMBER`/`DATETIME` names. NUMBER remains numeric for plural
selectors; DATETIME accepts typed Gregorian dates/local clock readings and
a caller-resolved zone label. This small implementation is not complete
Intl: no other calendars, time-zone conversion, currency/percent formatting,
significant-digit rules or automatic numbering-system discovery. Typed
decimals are finite, within the u32 count magnitude, and at most nine decimal
places. Patterns currently receive minute precision. Dynamic formatting is
performed when data changes, before painting. ICU4X is not a dependency of
this formatting layer; ADR 0039 retains its rejected measured cost.

No existing area has moved in the mechanism slice. Area owners keep their
Rust tables until the coordinator accepts the engine and its first migration;
the first migrated area will supply the contributor model. The pseudo-locale
is tested in this slice; normal-screen captures follow that migration.


### Add a language

A folder under `view/locales/<tag>/` is embedded in the released app and
listed in the chooser by its own `meta-name`. It is not a draft area.
Before adding it, the coverage report must show `0 missing` and
`complete=true` for every language being shipped, with no syntax, variable
or formatting-rule errors; the complete native gate must pass. The staged
app currently ships only English and Korean while legacy tables remain.
A report over migrated messages does not prove those legacy screens are
translated.

Prepare a candidate outside `view/locales/`, alongside copies of the
current English/Korean source folders, and inspect it without shipping it:

```bash
scripts/locale-coverage.sh --root CANDIDATE_ROOT
```

The two-message Japanese folder is solely a test fixture under
`view/tests/fixtures/locales/`. Registry, coverage and fallback tests read
that root; its partial translations never enter the released registry.
To inspect that proof, from this workspace:

```bash
scripts/locale-coverage.sh --root view/tests/fixtures/locales
```

Without `--root`, the report also reads the compiled shipped-language list.
Core validation supports partial candidates and English fallback; the
released-catalog test requires every shipped catalog to be complete.

## A release

`spikes/release/build.sh` (the release track's) builds every artifact of
a label from this workspace, signs the set and replaces it in the
distribution directory at once. Nothing here writes there.

### Official agent artwork (ADR 0048)

`kit::agent_mark` is the one embedded-image control. Its opaque `Artwork`
constants carry reviewed vendor vectors, `Size` the requested logical
size, and `Space` the surface's existing room. Unlike `glyph`, it preserves
colours. A brand minimum/clear space that does not fit produces the bot
fallback. No URL or SVG from the wire reaches this control. Source hashes,
terms and permission requirements are in `docs/design/agent-marks/README.md`.

`agent_mark::GITHUB` is shared artwork for Copilot identification and
GitHub forge pages. `forge_mark(Forge::GitHub, size, tokens)` supplies the
second typed use without a second copy. An agent mark's identity still
belongs to `view`; the kit knows no node or session. The Invertocat's agent
use requires explicit GitHub Copilot wording in the same row or tooltip
(ADR 0048). The unchanged official Gemini CLI PNG is the sole raster mark.

The artwork picker header uses its own ordinary 16-point slot rather than
its rows' common 32-point column. A long header title follows the reference's
single-line ellipsis. Disabled artwork retains its approved colour/opacity;
the words and controls are muted separately. Enter validates the current
row's availability and bounds before dispatch. The 44-point touch model
pill holds OpenCode's 32 × 36 clear-space box; the short pointer pill and
inline spots use Generic. A Settings ProviderRow needs a fitted adapter,
not an unconstrained 36-high child inside its padded content band.

`view::AgentMark::of` maps only typed node identities; `app::agent_mark`
adapts them to kit artwork once. The draft, sidebar and board call that
adapter. Added agents retain Generic even if their labels mention a brand.
Settings › About lists vendor sources, trademark terms and the Gemini
original's separate software licence. For long-name capture fixtures,
after `scripts/fleet.sh start ROOT TARGET`, run
`bash scripts/mark-fixtures.sh ROOT TARGET`; it checks the node's saved
bilingual titles and full custom-agent labels before captures.

Machine management uses `view::machine_book` for validated names, edit outcomes
and safe localized failures, and `link::machines` for the CLI's existing node
book. `BookFile` is reopened immediately before each atomic edit; the app does
not maintain a second machine list on disk. A forget ends the app's watched
stream only, leaving the remote session running. Original file errors remain
at the file boundary because a malformed book can contain credentials.

The actor resolves the book path once and reads it on the blocking pool for
startup, tick, reload, retry and edits. Syntax failures share `FileFault` with
pairing; no transient raw parser error reaches an event. Failed edits leave
clients and the session follow untouched. Browser refusals retain the original
Read/Edited response shape. Actor regressions use real framed RPC clients on
bounded in-process streams, including client-preserving rename and watched
forget; no real agent or network endpoint is contacted. Other areas' aliases
and project follows must be integrated before the Machines screen is delivered.
