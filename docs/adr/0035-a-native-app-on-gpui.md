# ADR 0035: The app is native, one Rust UI on GPUI

Status: accepted for the desktop, 2026-11-13 (the owner's decision); the
phone and the browser are aims, each to be shown working by the spike of
milestone 12 before it is promised. Replaces the proposal of ADR 0020
(one web UI in Tauri shells), which was never accepted.

## Context

The owner asked whether a desktop app was being built, and whether it is
light and fast as Zed is and updates itself well. Asked to choose between
the web page in a Tauri shell and a native UI as Zed's, the owner chose
the native one, and asked for the phone and the web to get the same by
installing an app, as far as that can be done.

What existed when this was decided:

- The page: React, served by `vessel ui` to a browser on the same machine
  (ADR 0017), rebuilt through milestones 7 to 11 to look and behave as
  agent-orchestrator and GOAT do. It is the only screen Vessel has.
- The node and everything a client needs to reach one, in Rust:
  `vessel-client` (direct, SSH and relay routes, pairing, a device's
  credential), the wire types of `vessel-proto`. The page reaches nodes
  through a bridge only because a browser cannot link Rust.
- What a screen means (which lane a session is in, how records fold into
  a conversation, which controls an agent offers) is written twice
  already where the node decides it (Rust) and where the page does
  (TypeScript, `web/src/data`).
- No release has ever been cut and nothing updates itself.

What was looked up on 2026-11-13 (sources in "References"):

- **GPUI**, the UI framework Zed is built on: Rust, drawn on the GPU,
  Apache-2.0. Official on macOS, Linux and Windows. On crates.io as
  `gpui` 0.2.2 (2025-10); Zed's own tree is well past it.
- **Phones.** Zed's request for GPUI on iOS was closed as not planned
  (2026-07-11); a maintainer says a touch API has landed upstream and a
  simulator prototype exists on a branch, tracked outside Zed's issues.
  A community layer, `gpui-mobile`, implements GPUI's platform for iOS
  (Metal) and Android (Vulkan or GL ES, arm64) on snapshots of upstream
  published as `gpui-pre`; it calls itself experimental and says input
  method composition, accessibility and the app's lifecycle are not
  complete.
- **Browsers.** Zed's tree has a web platform for GPUI compiled to
  WebAssembly and drawn with WebGPU; nothing official is announced, and
  forks differ on whether a WebGL2 fallback exists. WebGPU is in Chrome
  and Edge since 113 and Safari since 26; Firefox on Linux and Android
  had it only in pre-release builds in August 2026.
- **Components.** `gpui-component` (Apache-2.0, on crates.io, built for a
  shipping desktop app): some sixty controls, among them a virtual list,
  a dock, inputs, menus, Markdown and a code editor.

## Options

1. **The web page in Tauri shells** (ADR 0020's proposal). One UI
   everywhere at once; drawn by the system's web view, so lighter than
   Electron and no closer to Zed than the page is.
2. **Electron**, as Paseo's and agent-orchestrator's desktop apps. The
   heaviest; nothing recommends it.
3. **A native UI on GPUI.** What Zed is: no web view, no JavaScript, the
   client linked in. Every screen is written again in Rust. Phones and
   browsers are reachable only through layers that are experimental
   today.
4. **Another Rust UI kit that already covers desktop, phone and web**
   (Slint, egui, Makepad, Dioxus). Wider reach today; none is what the
   owner named, and none has GPUI's text handling and component set for
   a tool of this kind. Kept as the answer if the spike shows GPUI cannot
   reach a phone at all.

## Decision

Option 3.

- **One UI crate, in Rust, on GPUI**, for the desktop first: macOS, Linux
  and Windows.
- **It links `vessel-client`.** The app is a client of its nodes as the
  CLI is: it reads the node book, pairs as a device, takes the direct,
  SSH and relay routes. There is no bridge, no loopback server and no
  page credential in the native app.
- **What a screen means lives in one Rust crate with no UI in it** (the
  view models: the conversation folded from records, the board, an
  agent's controls, the sidebar, drafts, find), tested as
  `web/src/data` is today. The native UI draws it. This is what makes a
  second surface cheap, whichever it turns out to be.
- **The look stays.** The design system the page reached (the tokens,
  the type, the references' layout: `docs/design/web-ui/DESIGN.md`) is
  carried into GPUI's theme, not redesigned.
- **The phone and the browser use the same UI crate where the platform
  layers hold**: Android and iOS through GPUI's mobile platform,
  browsers through its WebAssembly one. Each is taken up only after the
  spike shows a real screen of Vessel's running there; until then, and
  wherever it does not hold, the page (`vessel ui`) remains the answer
  for a browser, and a phone uses the page.
- **The page stays** as the browser's client and is not removed. Work in
  flight on it is finished; screens not yet drawn anywhere (pull
  requests, shells and scripts, schedules, adding agents) are drawn in
  the native app first.
- **Updates** are the app's own: it checks for a release, verifies a
  signature, replaces itself and restarts, as Zed does. This needs
  releases to exist; the release pipeline is part of the milestone.

## Why this and not the shells

The owner's measure is Zed: start at once, idle at nothing, scroll a
long conversation without a hitch, small on disk. The page in any shell
is a web view and a JavaScript heap; milestone 11 measured it (a
20,000-entry conversation shown in 2.8 s, 13 MiB of heap, 335 KB to
load) and made it good for a page, which is not the same thing. The
node already is what was asked (21 MiB at rest); the screen was the
part that was not Rust.

## Costs

- **Every screen is written a second time.** Five milestones of page are
  not reused as code; their design, their behaviour, their tests' cases
  and their view logic (ported to Rust) are.
- **Two UIs exist until the native one covers a browser**, if it ever
  does. A feature is drawn twice or is missing from one.
- **The phone is not promised.** The mobile layer is a community
  project on snapshots of an upstream that has declined to plan it.
  Input method composition is incomplete there, which for this project's
  owner means Korean text may not be typeable on a phone until it is
  fixed, upstream or by us.
- **iOS cannot be built or tried on the machines this project is built
  on** (it needs macOS and Xcode, and a developer account to reach a
  phone). Windows and macOS desktop builds are confirmed only by CI.
- **GPUI moves fast and its released crate lags its tree.** The version
  Vessel builds on is pinned, and moving it is work each time.
- **A browser cannot run it without WebGPU.**

## Open, decided by the spike or by the owner

- Which GPUI Vessel builds on (the released crate, the snapshots the
  mobile and web layers need, or Zed's tree), and that nothing it pulls
  in carries a licence Vessel cannot link (ADR 0014).
- Whether Android, iOS and the browser hold (the spike).
- Where releases are published and with what key they are signed; code
  signing on Windows and notarisation on macOS; an Apple developer
  account for iOS (the owner).
- Whether CI may build Windows, macOS and iOS, which needs branches
  pushed (the owner; `AGENTS.md` section 11 forbids pushing today).

## References

- GPUI on crates.io: `https://crates.io/crates/gpui`
- Zed's issue on GPUI for iOS: `https://github.com/zed-industries/zed/issues/43206`
- The community mobile layer: `https://github.com/longbridge/gpui-mobile`
- The component library: `https://github.com/longbridge/gpui-component`
