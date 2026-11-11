# ADR 0020: One UI in a browser, a desktop app and a phone app

Status: proposed, 2026-11-11. The owner asked for the product to run as a
desktop app, in a browser and on a phone, "as Paseo does". This ADR
records what the references do, proposes how Vessel does it, and waits for
the owner's word before a shell is built. Milestone 9 only prepares for
it.

## Context

Today the page runs in one place: a browser on the machine that runs
`vessel ui` (ADR 0017). `vessel ui` binds loopback, holds the node book
and every credential, and bridges the page to the nodes over a websocket.
The page holds nothing and reaches nothing else.

A phone is another machine. It cannot open a loopback page, and ADR 0017
ruled out the two shortcuts on purpose: a node serving the page to the
network, and a hosted page given credentials.

What Vessel already has for a second machine (ADRs 0009 and 0010): a relay
that only forwards ciphertext, a Noise channel end to end, pairing by a
single-use link or QR code, and a revocable credential per device. All of
it is in `vessel-client`, in Rust, and the CLI on a second machine already
uses it.

What the two references do (read from their source):

- **Paseo.** One Expo / React Native component tree for the phone, the web
  (`react-native-web`) and the desktop (Electron loading the web build).
  Where a platform differs there are two files (`.native.tsx` beside the
  web one): the terminal is a grid drawn natively on the phone and
  xterm.js on the web. A phone reaches a daemon through a relay with a
  Noise channel, paired by a link. Wire types are hand-written schemas.
- **agent-orchestrator.** A desktop app (Electron, React DOM, Tailwind)
  and a separate phone app (Expo / React Native) with screens of its own.
  No UI code is shared: the shared view package is DOM only and a test
  keeps it so. The phone mirrors the wire types by hand and draws the
  terminal with xterm.js in a WebView.

## Options

1. **An Expo / React Native rewrite, as Paseo did.** One tree for every
   surface. The page built in milestones 7 to 9 (React DOM, CSS, xterm.js)
   is rewritten in React Native primitives; the terminal becomes two
   implementations; the look of agent-orchestrator, which is DOM and CSS,
   is re-approximated in another layout system.
2. **Two UIs, as agent-orchestrator did.** Keep the web UI for desktop and
   browser, write a second, native UI for the phone. Every screen exists
   twice from then on.
3. **A progressive web app.** The page installed from a browser. It needs
   something on the network to serve it and to hold credentials, which is
   what ADR 0017 refused.
4. **One web UI in three shells.** The browser keeps `vessel ui`. The
   desktop and phone apps are Tauri 2 shells: a Rust core and the system's
   webview showing the same page. The core links `vessel-client` and the
   bridge and is a paired device of its own.

## Decision (proposed)

Option 4.

- **One page.** `web/` is the only UI. It gets one seam, the transport: a
  websocket to `vessel ui` in a browser, Tauri's IPC in an app. Both carry
  the same typed envelope (`web/src/wire/bridge.ts`), so nothing above the
  transport knows which it is.
- **The bridge becomes a library both hosts use.** `vessel-web` already
  separates the bridge (one connection per node, calls forwarded, streams
  followed with resume) from where its nodes come from (`NodeSource`).
  `vessel ui` supplies the CLI's book; an app supplies its own book and
  device credentials.
- **An app is a device.** It pairs with a node the way `vessel nodes add
  --pair` does and holds its credential in the platform's keystore. A
  phone reaches nodes through the relay with the Noise channel that
  already exists. No node serves a page, and no page holds a credential:
  ADR 0017's line holds, with the app's Rust core where `vessel ui` was.
- **The phone's layout is the page's narrow layout**: the sidebar as a
  drawer, the board's lanes stacked or swiped, the terminal full screen.
- **Types stay generated.** The app's IPC uses the same Rust types the
  websocket does, so the TypeScript the page sees is still generated from
  them and checked in CI.

## Why

- **It is the most Rust and the least second UI.** The owner's rule for
  this port is Rust where possible and strict TypeScript where a UI needs
  it. Option 4 adds Rust (the shells' cores, reusing `vessel-client`) and
  no second UI. Option 1 replaces the UI; option 2 doubles it.
- **The look survives.** The owner wants the product to resemble
  agent-orchestrator, whose UI is DOM and CSS. A webview shows that
  exactly. React Native would re-approximate it, on every screen, forever.
- **The hard part is already written.** Paseo needed a relay, a Noise
  channel and pairing for its phone app. Vessel has them, in Rust, which a
  Tauri core links directly; an Expo app would need them again in
  TypeScript or a native module.
- **The cost is honest.** A webview UI on a phone is not a native one:
  scrolling, the keyboard and the terminal need care, and Paseo drew its
  phone terminal natively for that reason. If the webview terminal is not
  good enough on a phone, the terminal alone can become a native view
  inside the same shell, which is the one place both references diverge
  per platform anyway.

## Consequences

- Milestone 9 keeps the page's transport behind one module and gives the
  page a narrow layout that works; it builds no shell.
- A later milestone adds the shells: a crate for the app core, the Tauri
  project, the transport's second implementation, pairing in the app,
  signing and store packaging. iOS builds need macOS; CI can provide it.
- ADR 0017's "the UI cannot be reached from another machine" stays true of
  `vessel ui`. An app on another machine is a different host of the same
  bridge, not a way into `vessel ui`.
- If the owner chooses option 1 instead, milestones 7 to 9's page is the
  specification for the rewrite, and the wire types, the board's rules
  (`data/board.ts`) and the tests carry over; the components and the CSS
  do not.
