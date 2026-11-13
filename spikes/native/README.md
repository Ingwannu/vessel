# Vessel's native app (milestone 12)

The first builds of the native app ADR 0035 decided on: one window with
the machines in the node book and their sessions, the board's lanes, a
session's conversation with its composer and permission answers, and
pairing with a node from a pasted link. It is a client of its nodes as
the `vessel` CLI is: the same node book, token files and device
credentials, reached through `vessel-client` (direct, and a node's relay
through the secure channel; SSH routes come later).

This is a workspace of its own, outside Vessel's: Vessel's gate does not
build it, and its tree (GPUI's) stays out of the main lock file. Its own
gate is `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
and `cargo test`, with the main workspace's lints copied.

## Crates

| Crate | Owns | Must not |
| --- | --- | --- |
| `view` (`vessel-app-view`) | what a screen means: the board's lanes and phrases, a conversation folded from its records, the permissions waiting | use any UI toolkit, do I/O |
| `link` (`vessel-app-link`) | the nodes: the book, connecting, the session list, a session's stream, prompts, answers, pairing; a tokio runtime of its own, commands in and events out on bounded channels | use any UI toolkit |
| `kit` (`vessel-app-kit`) | Vessel's kit: the design system's tokens in both themes, the type scale with the faces embedded, a button, a text field, Markdown | know about nodes or sessions |
| `app` (`vessel-app`) | the window, the screens, the binary | name `gpui-component`; screens import from `kit` |

```
app ──> kit ──> GPUI (gpui-pre 0.3.10) + gpui-component 0.8.1 (text field, Markdown)
 │
 ├──> link ──> vessel-client, vessel-proto
 └──> view ──> vessel-proto
```

Data flow: `link` lists every book node's sessions every two seconds (the
page's cadence) and sends a `Fleet`; opening a session follows its log
from the start and sends its records; `view` folds them; the screens draw
the result. A prompt, an answer or a pairing is a `Command` to `link`.

## Why GPUI, and which

ADR 0035: the app is native, drawn as Zed draws its own; Zed's framework
is a reference and is not taken as it is. The kit is ours so the layer
under it can change. Underneath for now: the `gpui-pre` snapshot 0.3.10
(Zed's tree, republished weekly), with `gpui-component` 0.8.1, the newest
pair the community mobile layer also pins. Every problem met with it is
recorded in `docs/plans/backend-m12-spike.md` (question 9).

The toolchain is Rust 1.98.1 (`rust-toolchain.toml`), not Vessel's 1.93:
the snapshot uses `std::hint::cold_path`, stable since 1.95.

## Build and run

```bash
cd spikes/native
(cd ../../web && npm ci)          # Pretendard comes from the page's packages
scripts/fonts.sh                  # the faces into assets/fonts (not committed)
cargo run -p vessel-app
```

Linux needs the X11 and Wayland development libraries GPUI links
(`libxkbcommon-dev`, `libwayland-dev`, `libxcb*-dev`, `libfontconfig-dev`)
and, to run, a Vulkan driver (Mesa's is enough).

- `VESSEL_CONFIG_DIR`: another configuration directory (its
  `vessel/nodes.toml` is the book), as the CLI reads it.
- `VESSEL_APP_THEME=light|dark`: pin the scheme (default: the system's).
- `VESSEL_APP_SIZE=1280x800`: the window's size.
- `VESSEL_BUILD_LABEL` (at build time): the label in the title and the
  sidebar's foot, `dev` without it.

## Trying it against fake nodes

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

## A release

`spikes/release/build.sh` (the release track's) builds every artifact of
a label from this workspace, Linux included, signs the set and replaces
it in the distribution directory at once. Nothing here writes there.
