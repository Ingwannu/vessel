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
| `view` (`vessel-app-view`) | what a screen means: the board's lanes and phrases, a conversation folded from its records, the permissions waiting; **the text table** (`view/src/text.rs`, English and Korean) | use any UI toolkit; do I/O |
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
  chrome.rs        what is in the title bar
  areas/
    mod.rs         one `mod` line and one field of `Areas` per area
    common.rs      pieces several areas draw (a frame's title, its foot, a session's colour)
    draft.rs       the new chat: composer first, its tray of pickers
    session.rs     a session: the conversation, the composer, permissions, stop
    board.rs, archive.rs, sidebar.rs, pair.rs
```

### Adding a screen (an area)

1. Its meaning in `view` (a module of its own, ported from the page's
   `web/src/data/...`, with the page's test cases as its tests: same
   inputs, same outputs; fixture files used by path, not copied).
2. Its node calls in `link`: a `Command` variant, an `Event` variant
   when it answers, the call in `State::handle`.
3. Its words in `view/src/text.rs`: a field per sentence in `Text`, the
   English in `EN`, the Korean in `KO` (a field missing from one does not
   compile); sentences with values are methods matching on `self.lang`.
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

### Rules every screen follows

- Words through `view::Text` only; English and Korean together.
- Colours, type, spacing and radii from the kit's tokens, never a literal
  in a screen. Pretendard is behind every face (the kit sets it); no
  generic family anywhere.
- An error belongs beside the control it is about; the frame's foot is
  for what belongs to no control.
- Every screen works from 360 points wide (the phone); targets are 44
  points on touch; nothing depends on hover (ADR 0036; the shell's
  breakpoint and sheets are the lead's next slice).

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
its manifest):

```bash
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
- `VESSEL_BUILD_LABEL` (at build time): the label in the title bar and
  the sidebar's foot, `dev` without it.

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
entries, `wait` by waiting; any other prompt runs a tool that asks for
permission.

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
