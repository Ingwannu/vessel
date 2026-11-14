# ADR 0069: One browser, owned by the node

Status: proposed, 2026-11-14. Documents only. No browser code is written
until this ADR is accepted. The inventory, the line references and the
slice order are `docs/plans/browser.md`.

## Purpose and intent

Paseo gives a workspace a browser the person can see, and gives agents
twenty-two `browser_*` tools that drive those same tabs, including the
sign-ins the tabs share. Vessel has neither. The aim is that behaviour:
one profile, the person's tabs and the agent's tools on that same
profile, the tools off until someone turns them on, and a refusal that
names what was wrong.

The native app is GPUI. It cannot host the Electron webview Paseo uses.
This ADR chooses where the one browser lives, and stops before any
engine, so the tool contract can be reviewed on a fake.

## Existing implementation and constraints

**Paseo** (0.15.4, read 2026-11-14; the plan names every file and line).
The browser is an Electron `<webview>` in the desktop app, one session
partition `persist:paseo-browser` for the whole app. Workspace tabs
share cookies and site data; the settings copy says so. A browser with
no desktop app does not exist: the web client shows "Browser is
desktop-only", and the daemon answers `browser_no_host` when no desktop
has registered. Tools are omitted from the agent's MCP server unless
`daemon.browserTools.enabled` is true, and that defaults to false. The
warning on the switch says agents then control tabs, including
logged-in state.

The daemon does not run the browser. It brokers
`browser.automation.execute.request` to the most recently registered
desktop, or to the desktop that owns that tab. The page is sandboxed
(`nodeIntegration` false, `contextIsolation`, `sandbox`, `webSecurity`,
no nested webview). Navigation is http, https or `about:blank`.
`browser_evaluate` runs the agent's JavaScript in the page. Uploads
must sit inside the agent's working directory. Dialogs are not shown:
alert is accepted, confirm, prompt and beforeunload are dismissed.

The accessibility snapshot is YAML built in the page, adapted in
Paseo's tree from Playwright's injector (Apache-2.0, Microsoft). Vessel
does not copy that script, nor any other Paseo source.

**Vessel.** No browser tools, no CDP client, no page engine. `vessel-web`
is the control page (ADR 0017), not a page an agent drives. The native
app is a client of nodes (ADR 0035): `view` has no I/O, `link` has no
toolkit, `kit` has no application types, `app` draws. `vessel-mcp` is a
client of the node (ADR 0001, ADR 0013): a tool is one protocol call
through the agent's principal. `vessel-proto` does no I/O. The node's
settings are one object, read whole and changed field by field, so a
new field stays absent for an older peer (protocol 12, `settings.rs`).
This branch speaks protocol 20 (`PROTOCOL_VERSION` in
`crates/vessel-proto/src/node.rs`).

## Alternatives considered

1. **The node drives a headless Chromium over CDP.** Agents get the
   tools with no app open. The app later shows frames of those same
   tabs, or a remote view, and sends input back through the same call.
   One profile, in the node's data directory.
2. **A system webview beside the GPUI window, per platform.** `wry`
   0.57.2 (Apache-2.0 OR MIT, crate archive 222,390 bytes on crates.io,
   2026-11-14) draws WebView2, WKWebView or WebKitGTK. The person sees
   a live page. The agent tools would have to be implemented three
   times, because those engines do not share Chrome's CDP.
3. **Both engines.** Chromium for agents, a system webview for the
   person.

A Chromium Embedded Framework build inside the GPUI process was not
measured and is not an option here: it is a second copy of Chromium
plus a native windowing fight, and the standard library cannot paint
HTML.

## The choice

Option 1. One browser, owned by the node, spoken to over CDP when a
real engine exists. The app is a view of those tabs, not a second
browser. The first code slice (the plan's slice 2) is the tool surface
and an in-memory fake. No Chromium, no webview, no new crate dependency
until a later ADR names the binary.

Protocol **21 is provisional**. It is the next number after this
branch's 20. Another track may take it. The node integrator assigns
the final number before the code slice merges.

## Why this and not the others

Paseo's fact that matters is that the person and the agent share one
profile. Option 3 breaks it: Chromium and WebKit do not share a cookie
jar, so a sign-in in the panel would not be a sign-in for the tools, or
the reverse. That is the opposite of the settings sentence Paseo shows.

Option 2 cannot implement the twenty-two tools once. File upload is
`DOM.setFileInputFiles`, screenshots are `Page.captureScreenshot`,
clicks are `Input.dispatchMouseEvent`, and the snapshot is script
injected into the page. WebKitGTK (LGPL-2.1, the system library, not
part of `wry`) and WKWebView do not offer that surface. Per-OS
behaviour would be a standing difference from Paseo, which this version
does not accept. A webview beside GPUI is also a separate native
window, not a kit control, and `view` must not do I/O.

Option 1 keeps one profile and one command set. It differs from Paseo
in where the process lives, which is forced: Vessel has no Electron.
The differences are listed below, each with why.

The fake comes before the engine because the contract (ids, limits,
refusals, the setting's default) is the part review can settle without
a 100 MB binary, and because a real browser in the first slice would
make the tests depend on a download.

## Advantages, costs and effects

**Advantages.** Agents can use the tools on a node that has no app
open. One implementation of each command. The profile is a directory
the node already knows how to keep private. The app stays a client.

**Costs.** The person does not get a live page in the first code
slices. A later view is pixels or a remote surface, not a native
webview, so input methods and dragging a file in from the desktop are
weaker than Paseo's until that view is designed. The node becomes able
to run a browser that can reach the network (SSRF) and hold cookies.
That is why the setting defaults off, why navigation stays http(s), and
why the browser's environment must not receive the node's token, device
credentials or harness secrets.

**Effects.** A new crate `vessel-browser` (no I/O in slice 2 beyond the
fake's memory; `vessel-proc` only when a real process is spawned).
Edges, recorded in ADR 0001 and `docs/ARCHITECTURE.md` in the commit
that adds them, not before: `vessel-browser -> proto`, and later
`-> proc`; `vessel-node -> vessel-browser`. `vessel-mcp` stays
`-> proto, client` and gains no browser crate. CDP, if it is ever a
Rust crate, has one owner: `vessel-browser`. No row is added now,
because the boundary check fails a table row with no manifest.

## Contract of the first code slice

Slice 2, after this ADR is accepted. Not built here.

### Wire (provisional protocol 21)

One method, not twenty-two. The command is a closed enum, matched
exhaustively. Paseo already has one execute request; a method per tool
would restate the list (QUALITY M5).

- `browser.call` / `browser.call` / `"browser.call"` `[Operate, 21]`:
  `BrowserCall` => `BrowserResult`.
- `NodeSettings.browser_tools`: a two-variant enum `Off` (default) and
  `On`. `#[serde(default)]`, omitted on the wire when `Off`, so a node
  of protocol 20 and a saved `settings.json` from protocol 20 both read
  as off. Same pattern as `drivers` (protocol 18).
- `SetNodeSettings.browser_tools`: `Option`, absent means unchanged.
  Needs `admin`, as every other setting change does. `node.settings`
  stays `read`.

**Older and newer, both ways.**

- A client on 20 never sends `browser.call` (`Call::since`). A node on
  21 asked by that client does not run it. A node on 20 has no method;
  the client refuses before sending.
- A client on 20 decoding `node.settings` from a node on 21 ignores the
  new field (serde's default: unknown fields are not an error on this
  struct). It cannot turn the tools on, and the tools stay off unless
  a newer client set them.
- A client on 21 reading a node on 20 sees the field absent and treats
  it as off. It does not send `browser.call`.

No new `ErrorCode`. Browser failures use the codes below, plus
`ErrorDetail::Browser { reason: BrowserFailure }` so a client can match
without reading the message (T3). `ErrorDetail` already has
`#[serde(other)] Unknown`: a peer that does not know `browser` keeps
the `ErrorCode` and the message and sees `Unknown`. A message never
contains a cookie, a token, a page's HTML, or an evaluated result.

| Paseo code | `ErrorCode` | `BrowserFailure` | When |
| --- | --- | --- | --- |
| `browser_disabled` | `Unsupported` | `Disabled` | The setting is off. |
| `browser_denied` | `Forbidden` | `NoWorkspace` | `list` or `new` without a workspace. |
| `browser_no_host` | `Unavailable` | `NoEngine` | Slice 2 has an engine (the fake), so this is reserved for a later real engine that is not running. The fake does not return it. |
| `browser_tab_not_found` | `NotFound` | `UnknownTab` | No such tab in this workspace. |
| `browser_tab_closed` | `Conflict` | `Closed` | The tab was closed. |
| `browser_stale_ref` | `Conflict` | `StaleRef` | The ref is not from the tab's latest snapshot. |
| `browser_timeout` | `Unavailable` | `TimedOut` | The fake does not time out. Reserved. |
| `browser_unsupported` | `Unsupported` | `Unsupported` | A command the fake refuses. Slice 2's fake implements every command, so tests that want this use a second fake that advertises a subset. |
| `screenshot_no_frame` | `Unavailable` | `NoFrame` | The fake's screenshot is a fixture, so this is reserved. |
| `browser_unknown_error` | `Internal` | `Internal` | A bug. Not used for a bad argument. |
| (validation) | `InvalidRequest` | `Invalid` | A bad id, URL, ref, size, wait or path. |

Paseo's `retryable` boolean is not a wire field. `Unavailable` is the
retryable class; the others are not. Stated difference, under that
heading.

### Types (no bare strings across a module)

Parsed once at the MCP and RPC edge.

- `BrowserId`: Paseo's two shapes, UUID version 4 or
  `<13 or more digits>-<hex>`. The fake mints a UUID version 4.
- `ElementRef`: `@e` and a decimal, as Paseo (`^@e\d+$`).
- `HttpUrl`: `http` or `https` only. A tool URL with no scheme and no
  whitespace becomes `http://` plus the text, as
  `normalizeHttpUrlInput` does. `javascript:`, `file:`, `data:` and
  embedded whitespace are `Invalid`.
- `BrowserCommand`: one variant per tool in the plan's table. No
  `_` arm.
- `ClickButton`: `Left` (default), `Right`, `Middle`.
- `ClickCount`: `Single` (default), `Double`. Not a `bool`.
- `KeyModifier`: `Alt`, `Control`, `Meta`, `Shift`.
- `ShotExtent`: `Viewport` (default), `Page`. Not a `bool`.
- `WaitFor`: exactly one of `Text` or `Url`, both non-empty.
  `timeout` defaults to 5 seconds and is at most 30 seconds, as Paseo.
- `LogLimit`: 1 to 200, default 50.
- `CssDelta`: the scroll's x and y, finite. Non-finite is `Invalid`.
- `ViewportSize`: each side at least 1. **Cap 8,192.** Paseo has no cap;
  stated difference.
- `SnapshotText`, `EvaluateJson`: at most 80,000 bytes, with `truncated`
  set when the cap binds. Paseo's preview cut for evaluate is 79,000
  before the cap marker; slice 2 uses one cap of 80,000 and `truncated`,
  and records the 79,000 figure as Paseo's so a later engine can match
  it if review wants the same cut.
- Screenshot bytes: at most 8 MiB decoded. Paseo has no cap; stated
  difference. The fake returns a fixed 1×1 PNG fixture, well under it.

`browser_evaluate`'s result is a string the node does not parse
(`result_json`), never a `serde_json::Value`. The fake does not execute
the function. It returns a deterministic JSON string that echoes the
function's length and whether a ref was passed, with `truncated: false`.

### Tools

The MCP names are Paseo's. Arguments match the plan's table. Each tool
is one `browser.call` on the session's own node. Hidden while
`browser_tools` is off (Paseo does not register them); a call that
arrives anyway is `Disabled`, not a dropped success. When on, the tools
are marked open-world. `browser_evaluate`, `browser_navigate`,
`browser_new_tab`, `browser_upload` and `browser_click` are also marked
destructive: they change a page or run its script. Paseo does not set
those MCP flags; stated difference, so a client that heeds the flag
asks before running them.

`list` and `new` require the session's workspace, else `NoWorkspace`.
Every other command requires a tab of that same workspace, else
`UnknownTab`. An agent with no workspace can call none of them. The
node token and a device with `operate` may name a workspace; without
one, `list` and `new` are `NoWorkspace` too. Tabs are not global.

The fake keeps tabs in memory per workspace: url, title, history for
back and forward, the last snapshot's refs, a viewport, console and
network rings of the requested limit, and closed versus open. It does
not open a socket, read a cookie, or touch the disk except the upload
check, which resolves the paths against the session's workspace root
and refuses one that escapes (`Invalid`), without reading the file.
`new` does not change which tab the fake calls active (Paseo opens in
the background). Omitted URL is `https://example.com`.

### What slice 2 tests

The rules beside the code, on the fake only: the setting's default and
both directions of protocol 21, each command's happy path and the
refusals in the table, a ref that survives until the next snapshot and
dies after it, an upload path outside the workspace, a non-http URL, a
viewport of 0 and of 8,193, a wait with both text and url, and the MCP
list empty while the setting is off. No sleep. No real browser. No
network.

## Stated differences from Paseo

1. **The browser process lives on the node, not in the desktop app.**
   GPUI cannot host Paseo's webview, and a system webview cannot
   implement the tools once. Consequence: tools work with no app
   connected. Paseo returns `browser_no_host` in that case. Vessel has
   no such host. `NoEngine` is only for a real engine that failed to
   start, which slice 2 does not have.
2. **Tools stay in the MCP catalogue's implementation but are not
   advertised while the setting is off,** and a direct `browser.call`
   is still refused with `Disabled`. Paseo only does the first half.
   The refusal is so a caller that guesses the method cannot succeed
   unseen.
3. **`retryable` is not a field.** `Unavailable` means retry; the other
   codes do not.
4. **`bool` arguments on the wire are enums** (`ClickCount`,
   `ShotExtent`). The MCP edge accepts Paseo's booleans and converts
   them once.
5. **Viewport sides above 8,192, and screenshots above 8 MiB, are
   refused.** Paseo accepts any positive size and any PNG string.
6. **The fake does not execute JavaScript or paint.** Its evaluate and
   screenshot results are fixtures. A real engine must not keep the
   fixtures.
7. **Destructive and open-world flags** on the tools named above. Paseo
   does not set them.
8. **The aria snapshot will be written for Vessel, not copied** from
   Paseo or from Playwright. The observable contract (YAML, the caps in
   the plan, refs `@eN`, stale after a navigation) is what must match.

## What this does not decide

The Chromium binary, its licence file, and who downloads it. Measured
only as a size reference, not a choice: Chrome for Testing stable
157.0.8103.78 `chrome-headless-shell` zip, Content-Length on
2026-11-14, linux64 127,965,683 bytes, mac-arm64 104,822,985, win64
126,047,001. The NOTICE inside the zip was not read. `chromiumoxide`
0.9.1 and `headless_chrome` 1.0.22 are clients, not browsers (MIT OR
Apache-2.0, 102,338 bytes; MIT, 250,258 bytes). `fantoccini` 0.22.1
speaks WebDriver, not CDP (MIT OR Apache-2.0, 84,785 bytes). None is
adopted.

The app's panel (toolbar, device presets, element annotate, clear
data) is specified in the plan and is not slice 2. Clearing data will
be `admin` when a profile exists. DevTools, popups and the keyboard
preload have no slice yet.
