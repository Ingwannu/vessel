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
| `view` (`vessel-app-view`) | what a screen means: the board's lanes and phrases, a conversation folded from its records, the permissions waiting; **the words** (`view/src/text/`, English and Korean, one file per area) | use any UI toolkit; do I/O |
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

### Rules every screen follows

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
the question card is checked against them.

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

## A release

`spikes/release/build.sh` (the release track's) builds every artifact of
a label from this workspace, signs the set and replaces it in the
distribution directory at once. Nothing here writes there.
