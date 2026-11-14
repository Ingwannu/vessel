# Plan: the browser panel and the browser tools (ADR 0069)

Paseo keeps browser tabs inside a workspace and lets an agent drive
them with `browser_*` tools, only after someone turns the tools on.
Vessel has no browser. ADR 0069 chooses one browser owned by the node,
and this plan is the inventory and the slice order. No code until the
ADR is accepted.

A count of twenty-one is short of the source. It lists
twenty-two commands (`BROWSER_AUTOMATION_COMMAND_NAMES` in
`packages/protocol/src/browser-automation/rpc-schemas.ts`, lines 21 to
44). The catalogue lists the same twenty-two
(`packages/protocol/src/paseo-tool-presentation/catalog.ts`, lines 378
to 399). This plan follows the source.

## Resume point

- **Done:** the inventory below, the three designs, ADR 0069
  (proposed). The protocol number is the number given at merge
  (`BROWSER_PROTOCOL`). This plan does not choose one. No code, no
  crate, no dependency, no `ARCHITECTURE.md` row (a row without a
  manifest fails the boundary check). The returned review and the
  shared-document rules are in the ADR: who a page can reach, where a
  credential lives, a test double that is not in the product, WebView2
  apart from WebKit, `Scheme` and `NoHistory`, the 15 second deadline,
  and the tool list fixed at session start. The second return is also
  in the ADR: WebRTC UDP and QUIC are closed by launch switches
  because the HTTP proxy does not carry them, and text an agent types
  stays in the session log as the tool call's input. The setting and
  the three typing tools say so. Nothing is redacted.
- **Not read, so not asserted:** the CDP sequence in `service.ts` past
  the lines this plan cites; the pane render past the presets (lines
  131 to 146), the shortcut function (lines 413 to 421) and the toolbar
  labels; the element-selector body; the keyboard guest preload body;
  snapshot fingerprint fields; the WebView2, WKWebView and WebKitGTK
  headers. Chromium's session code that decides QUIC when only a
  proxy is set. Whether the proxy-resolving TCP factory exempts
  loopback unless the launch says otherwise. WebTransport's source.
  The bytes inside a harness transcript.
- **Next:** slice 2, the tool surface. Accepted for that slice only.
  The engine (which Chromium, how it reaches the node) waits for the
  owner. Merge sets `BROWSER_PROTOCOL`. No Chromium download or
  process in slice 2.

## Existing asset check

| Asset | What it is | Decision |
| --- | --- | --- |
| `vessel-mcp` tools | One protocol call per tool, through the agent principal (ADR 0013). | Extend with `browser_*` tools that call `browser.call`. Do not put a browser engine here: the crate may depend only on `proto` and `client`. |
| `vessel-web` | The control page (ADR 0017). | Not a page an agent drives. Do not extend it into the workspace browser. |
| `vessel-proto` settings | One object. This ADR adds one key, `browser_tools`, absent meaning off. Another track's key is that track's ADR, added at merge, and is not listed here. | Extend with that one key. |
| `vessel-proc` | Spawning in a process group, and on Windows a kill-on-close job object (`docs/ARCHITECTURE.md`, read). | Not used in slice 2. A later engine spawns through it. No signal, systemd unit or launchd job. |
| Native `view` / `link` / `kit` / `app` | The app is a client. `view` does no I/O (ADR 0035). | The panel, when it exists, is a later slice: meaning in `view`, the call in `link`, controls from `kit`, screen in `app`. Not slice 2. |
| `chromiumoxide`, `headless_chrome`, `fantoccini`, `wry` | Clients or a system-webview host. Sizes and licences in ADR 0069. | Not adopted. The standard library cannot speak CDP or paint HTML. A dependency waits for the engine slice and its own ADR paragraph. |

Nothing in the tree is a browser. A new crate `vessel-browser` is
required for the engine because `vessel-node` already owns policy and
must not also own CDP, and `vessel-proto` cannot do I/O. Slice 2 adds
the crate with the product engine absent. The in-memory double is
`cfg(test)` only, or a feature a release build cannot enable.

## What Paseo does

Read from Paseo 0.15.4 on 2026-11-14. Paths are inside that tree.

### Where the browser runs, and who owns the profile

The desktop app owns it. The daemon never opens a window.

- Partition `persist:paseo-browser`
  (`packages/desktop/src/features/browser-profile.ts`, line 1). One
  session for every tab in the app, not one per workspace and not one
  per tab. Legacy partitions
  `persist:paseo-browser-<browserId>` are still cleared, at most 1,000
  ids, with a comment to remove that after 2027-01-15 (lines 3 to 4
  and 69 to 81).
- The webview's `partition` attribute is that session
  (`packages/app/src/desktop/browser/resident-webviews.ts`, line 307).
  Attach is refused unless the partition is exactly that and the URL
  is allowed (`packages/desktop/src/features/browser-webviews/index.ts`,
  lines 42 to 45).
- Clear data is a settings row, "Cookies and site data", copy: "Browser
  tabs share sign-ins and site data across Paseo." Confirming clears
  cookies, filesystem, indexdb, localstorage, serviceworkers,
  cachestorage, shadercache, the HTTP cache and the auth cache, then
  reloads open guests
  (`browser-profile.ts` lines 6 to 14 and 99 to 122;
  `packages/app/src/desktop/browser/settings/browser-data-section.tsx`;
  English strings in `packages/app/src/i18n/resources/en.ts`, the
  `browserData` block near line 2514).
- Tab records (url, title, viewport, favicon) are persisted in the
  app's storage under `workspace-browser-store`
  (`packages/app/src/desktop/browser/store/index.ts`, lines 72 to 85).
  Cookies are not in that record. They are in the Electron session.
- The web client has no webview. It shows "Browser is desktop-only" /
  "Open this workspace in Electron to use the built-in browser."
  (`packages/app/src/desktop/browser/pane/index.web.tsx`; strings at
  `en.ts` lines 575 to 580).

### How a tool reaches the page

1. The agent's MCP server registers the tools only when
   `browserToolsEnabled` is true
   (`packages/server/src/server/agent/tools/paseo-tools.ts`, lines
   1358 to 1359). The flag defaults to false
   (`packages/server/src/server/config.ts`, line 531;
   `packages/server/src/server/browser-tools/policy.ts`, lines 16 to
   20). The settings card warns: "Allow agents to access and control
   Paseo browser tabs, including logged-in browser state. Only enable
   this for agents you trust."
   (`packages/app/src/screens/settings/browser-tools-config.ts`, lines
   3 to 5).
2. `list` and `new` refuse, before the broker, when the caller has no
   workspace id. The message tells the agent to start from a workspace
   (`packages/server/src/server/browser-tools/tools.ts`, lines 28 to
   29 and 746 to 759). Other tools do not check that themselves; the
   desktop filters by workspace.
3. `BrowserToolsBroker` (default timeout 15 seconds, line 46) sends
   `browser.automation.execute.request`. `list_tabs` is asked of every
   connected desktop and the tabs are concatenated (lines 209 to 237).
   `new_tab` goes to the most recently registered desktop (lines 276
   to 283). Any other command goes to the desktop that last returned
   that `browserId`. If that desktop disconnected, the error is
   retryable `browser_no_host` ("The app hosting browser tab …
   disconnected."). If it is the only desktop, the command is sent
   there anyway. If several desktops are connected and the id is
   unknown, `browser_tab_not_found` (lines 284 to 336).
4. The desktop registers as host kind `"desktop app"` with every
   command name
   (`packages/app/src/desktop/browser/automation/handler.ts`, lines 77
   to 78). `resize`, `new_tab` and `close_tab` are handled in the app
   before the CDP service. The service's own `resize` returns
   `browser_unsupported` ("browser_resize is handled by the app
   runtime.", `service.ts` lines 505 to 506).
5. `new_tab` opens a workspace tab with intent `background` (the
   person's view does not switch), default URL `https://example.com`,
   and waits up to 5 seconds for the webview to register (handler
   lines 349 to 391).

A command the desktop did not advertise is `browser_unsupported`.

### The twenty-two tools

Schemas: `rpc-schemas.ts`. Registration and the text the model sees:
`tools.ts`. Results below are the success payload's fields. On success
the tool also returns a one-line text summary (the plan does not
restate every sentence; they are `summarizeBrowserSuccess` and the
helpers under it, `tools.ts` lines 849 to 1036). On failure the text
is the mapped sentence (lines 1037 to 1054) plus any dialogs. Dialogs,
when present, are appended as "Handled browser dialogs: …".

`browserId` matches a UUID version 4 or `<13+ digits>-<hex>`
(`rpc-schemas.ts` lines 14 to 28). The app mints `randomUUID()`, or
`Date.now()-<hex>` if that is missing (`store/index.ts` lines 35 to
43). A ref matches `^@e\d+$`.

| Tool | Arguments | Result |
| --- | --- | --- |
| `browser_list_tabs` | none | `tabs[]`: `browserId`, optional `workspaceId`, `url`, `title`, `isActive` default false, `isLoading` default false, optional `canGoBack`, `canGoForward` |
| `browser_new_tab` | optional `url` | `browserId`, `workspaceId`, `url` |
| `browser_close_tab` | `browserId` | `browserId` |
| `browser_navigate` | `url`, `browserId` | `browserId`, `url` |
| `browser_back` | `browserId` | `browserId` |
| `browser_forward` | `browserId` | `browserId` |
| `browser_reload` | `browserId` | `browserId` |
| `browser_snapshot` | `browserId` | `browserId`, optional `workspaceId`, `url`, `title`, `format` = `aria-yaml`, `snapshot`, `truncated`, `stats` (`nodeCount`, `refCount`, `textLength`, optional `iframeCount`, `maxDepth`) |
| `browser_screenshot` | `browserId`, `fullPage` default false | `mimeType` = `image/png`, `dataBase64`, `width`, `height`. The MCP image block carries the bytes. `structuredContent` omits `dataBase64` (`tools.ts` lines 810 to 818 and 832 to 844). |
| `browser_click` | `ref`, `browserId`, `button` default `left`, `doubleClick` default false, `modifiers` default `[]` (`left`/`right`/`middle`; `Alt`/`Control`/`Meta`/`Shift`) | `browserId`, `ref`, optional `x`, `y` |
| `browser_hover` | `ref`, `browserId` | `browserId`, `ref`, optional `x`, `y` |
| `browser_drag` | `sourceRef`, `targetRef`, `browserId` | those ids, optional source and target `x`/`y` |
| `browser_fill` | `ref`, `value` (any string, including empty), `browserId` | `browserId`, `ref` |
| `browser_select` | `ref`, `value`, `browserId` | `browserId`, `ref`, `value` |
| `browser_type` | `text`, optional `ref`, `browserId` | `browserId`, optional `ref`, `x`, `y`. No ref means the focused element. |
| `browser_keypress` | `key` (at least one character), optional `ref`, `browserId` | `browserId`, `key`, optional `ref`, `x`, `y` |
| `browser_upload` | `ref`, `filePaths` (at least one non-empty string), `browserId` | `browserId`, `ref`, `filePaths` |
| `browser_wait` | exactly one of `text` or `url` (non-empty), optional `timeoutMs` (positive int, at most 30,000), `browserId` | `browserId`, `matched` = `text` or `url` |
| `browser_scroll` | `browserId`, optional `ref`, `deltaX`, `deltaY` (CSS pixels) | those, plus optional `x`, `y` |
| `browser_resize` | `browserId`, `width`, `height` (positive ints) | `browserId`, `width`, `height` after rounding to at least 1 (`resident-webviews.ts` lines 197 to 214). No maximum. |
| `browser_logs` | optional `maxEntries` (1 to 200, default 50), `browserId` | `console[]` (`level`, `message`, optional `source`, `line`, `timestamp`) and `network[]` (`url`, optional `method`, `status`, `type`, `startTime`, `duration`, optional `transferSize`) |
| `browser_evaluate` | `function` (at least one character), optional `ref`, `browserId` | `browserId`, `resultJson`, `truncated`. With a ref, the element is the function's first argument (`tools.ts` lines 582 to 590). |

URL rules for the tools (`tools.ts` lines 26 to 27 and 724 to 744):
empty is rejected; an explicit `http:` or `https:` URL must parse; any
other explicit scheme is rejected; whitespace is rejected; anything
else is prefixed with `http://` and must then parse as http. The
panel's address bar is a different function (`store/state.ts` lines
90 to 104): empty becomes `https://example.com`; a localhost or an IP
literal becomes `http://`; a scheme is kept; `//` becomes `https:`;
anything else becomes `https://`. Navigation of the webview then
allows only `http:`, `https:` and `about:blank`
(`window-open.ts` lines 74 to 86). Both rules stay. Slice 2
implements the tool rule only.

`browser_wait`: the host polls every 25 ms, default 5 seconds
(`service.ts` lines 68 to 69 and 1002). When the tool is given
`timeoutMs`, the broker's timeout is that plus 1 second so the broker
outlives the page (`tools.ts` lines 220 to 236).

### Limits read from the implementation

| Limit | Value | Where |
| --- | --- | --- |
| Broker timeout | 15 s, retryable `browser_timeout` | `broker.ts` line 46 |
| Wait default / max | 5 s / 30 s | `service.ts` line 68; schema |
| Actionability timeout | 5 s, then `browser_timeout` or `browser_stale_ref` | `actionability.ts` line 22. The check's script body was not read line by line. |
| Screenshot capture | 5 s, retry every 200 ms; no frame → `screenshot_no_frame` "The tab has not painted yet. Retry the screenshot." | `service.ts` lines 70 to 72 |
| Snapshot in the page | 1,500 nodes, 500 refs, 80,000 text | `aria-snapshot-script.ts` lines 7 to 9. The script says it is adapted from Playwright's injector, Apache-2.0. Do not copy it. |
| Snapshot text returned | 80,000, then a truncation line | `snapshot-engine.ts` lines 67 to 68 |
| Evaluate JSON | 80,000; preview cut 79,000; error text 2,000 | `service.ts` lines 74 to 76 |
| Logs | `maxEntries` default 50, max 200. The performance script keeps the last 200 resource and navigation entries. `method` is set only for a navigation, to `GET`. `status` is not filled by that script. | `service.ts` lines 809 to 820 and 1623 to 1636 |
| Dialogs per command | 20. Alert accepted. Confirm, prompt and beforeunload dismissed. | `dialog-handling.ts` lines 3 and 17 to 20 |
| Window-open queue | 20 URLs per guest | `window-open.ts` line 14 |
| Resident viewport when parked | 1,280 × 800 | `resident-webviews.ts` lines 12 to 13 |
| New-tab registration wait | 5 s | handler, default in `waitForBrowserRegistration` |

Refs are bound to a fingerprint from the latest snapshot and resolve
through `window.__PASEO_BROWSER_AUTOMATION__`. A miss is
`browser_stale_ref` (`snapshot-engine.ts` line 357). A navigation
makes old refs stale. The exact fingerprint fields were not copied
into this plan; the engine slice reads them before implementing a
real snapshot.

Upload (`service.ts` lines 1423 to 1461): needs the request's `cwd`;
each path is resolved against it; a path outside that directory is
refused ("browser_upload only accepts files inside the agent
workspace."); the files are set with `DOM.setFileInputFiles`. No file
bytes are returned.

Typing uses the webview's `insertText`, not CDP `Input.insertText`,
because CDP would type into the Paseo window (`service.ts` lines 41
to 48). A node-owned Chromium has no embedder focus, so that
workaround does not carry over. Clicks, hover, drag and scroll use
`Input.dispatchMouseEvent` (`trusted-input.ts`). The modifier bit
masks and the key-code aliases were read only at the top of that
file (lines 14 to 36).

### The panel

A browser tab is a workspace panel (`panel.tsx`). The label is the
page title, or the hostname, or the URL. The icon is the favicon, or
a globe. A loading tab is in the running status bucket.

The Electron toolbar (`pane/index.electron.tsx`, labels from `en.ts`
lines 581 to 612): back, forward, stop or refresh, the URL field,
device size, devtools, annotate element, screenshot element. Ctrl or
Meta, and not Alt or Shift, with `l` focuses the URL and with `r`
reloads (lines 413 to 421). Device presets, centred, not translated
names (lines 131 to 146): responsive (fill), iPhone SE 375×667,
iPhone 14 390×844, iPhone 14 Pro Max 430×932, Pixel 7 412×915,
Galaxy S20 360×800, iPad Mini 768×1024, iPad Air 820×1180, iPad Pro
11" 834×1194, iPad Pro 12.9" 1024×1366, Surface Pro 912×1368, Laptop
1366×768, Desktop 1080p 1920×1080, Desktop 1440p 2560×1440.

Annotate and screenshot-element copy a selection onto the clipboard
as an attachment for the agent. That is not a `browser_*` tool. The
selector's polling and the annotation script were not read.

`window.open`: a scripted popup (window features, or a named target
that keeps `window.opener`) and any POST stay real popups, because a
workspace tab would turn a POST into a GET (`window-open.ts` lines
87 to 114). Anything else http(s) becomes a workspace tab. Other
schemes are denied.

### Page security, as attached

`will-attach-webview` (`packages/desktop/src/main.ts` lines 743 to
762): refuse a guest that is not the Paseo partition and an allowed
URL; then force `nodeIntegration` false, `nodeIntegrationInWorker`
false, `contextIsolation` true, `sandbox` true, `webSecurity` true,
`webviewTag` false, `allowRunningInsecureContent` false; delete any
preload the guest asked for and set the keyboard preload.
`nodeIntegrationInSubFrames` is true so that preload runs in iframes;
the comment says node integration stays off. The preload body was
not read.

The guest can reach the network as the desktop user's machine, read
and write the shared partition (so any tab can see another tab's
cookies for the sites it loads), and run whatever
`browser_evaluate` is given. It cannot integrate Node, and it cannot
attach a nested webview. The tools are off until the setting is on,
because of the shared cookies.

Error codes (`rpc-schemas.ts` lines 3 to 12): `browser_disabled`,
`browser_no_host`, `browser_tab_not_found`, `browser_tab_closed`,
`browser_timeout`, `screenshot_no_frame`, `browser_denied`,
`browser_unsupported`, `browser_stale_ref`, `browser_unknown_error`.
Each has a message and `retryable` (default false). Retryable in the
broker: no host, stranded tab, timeout.

## Designs

### Node-owned Chromium over CDP

The node spawns one Chromium (headless shell or a Chromium the
operator names), one user-data directory under the node's data
directory, one profile for every workspace, matching Paseo's one
partition. Agents call `browser.call` with no app open. The app, in
a later slice, subscribes to frames or asks for a screenshot of the
same tabs and sends input back through `browser.call`.

Security is the ADR's destination table and credential section, not
a scheme check. The proxy refuses anything `is_public` refuses, for
TCP it carries. The engine also passes
`--force-webrtc-ip-handling-policy=disable_non_proxied_udp` and
`--disable-quic`, because WebRTC UDP and QUIC do not go through that
proxy. The browser is not given the node token, a device credential
or a harness secret. Cookies sit in `browser/` under the data
directory (`0700` / `0600`), one profile for every session on the
node. Text an agent types is kept in the session log with the tool
call. The setting stays off by default, and turning it on says so.
With no real engine the product answers `NoEngine` and does not
advertise the tools.

Cannot: sit inside a GPUI element; match Electron's input-method and
file-drag behaviour until a view is designed; ship inside the Rust
crate (the zip sizes are in the ADR). DevTools, extensions and
`window.open` popups are not in slice 2.

### System webview beside GPUI

`wry` on WebView2, WKWebView and WebKitGTK. The person gets a live
page. WebView2 can be asked Chrome's CDP methods (the SDK header was
not opened). WKWebView and WebKitGTK cannot (their headers were not
opened either), so Linux and macOS would not match Paseo. WebKitGTK
is LGPL-2.1 as a system library. The window would be a native child
beside GPUI, which `kit` does not own and `view` must not drive.

Cannot: one implementation of the twenty-two tools on Linux and
macOS; share a cookie jar with a Chromium the node runs.

### Both

Two engines, two profiles. Rejected in ADR 0069. The sentence Paseo
shows the person is that tabs share sign-ins. A webview the person
logs into and a Chromium the agent drives do not.

"Both" as one Chromium plus a view of it is option 1, not a second
browser.

## Slices

### Slice 1 (this commit)

ADR 0069 and this plan. No code. The gate's compile and test steps
are not run: this slice changes no Rust, and a build is out of
scope until the ADR is accepted.

### Slice 2 (first code, after acceptance)

The contract in ADR 0069: `browser.call` gated on
`BROWSER_PROTOCOL` (the number given at merge), one settings key
`browser_tools` default off, a product engine that is absent, the
twenty-two names decided at session start, and tests on a `cfg(test)`
double plus a product test that expects `NoEngine`. No binary, no
network, no app screen. Update ADR 0001, `docs/ARCHITECTURE.md` and
`docs/PROTOCOL.md` in that slice, when the edges exist. Regenerate
`web/src/wire/` because the wire changes.

### Later, not scheduled here

A real engine, only after its binary's NOTICE is read and a
dependency paragraph names the licence and the size. That launch
passes the two switches in the ADR, and its test is the loopback UDP
listener: gathering finishes, and the socket got nothing. The app's
panel comes after that, against the list above (toolbar, presets,
shared profile, clear data, background new tab). Element annotate and devtools stay
out until their own slice, because they are not `browser_*` tools.

## Review notes for slice 2

- Do not copy `aria-snapshot-script.ts` or Playwright's injector.
- Do not log a URL's userinfo, a cookie, or `result_json` at info.
  Do not strip `browser_type`, `browser_fill` or `browser_keypress`
  out of the session event. Their descriptions say the text stays.
- Upload checks the path before any engine sees it.
- The test double's evaluate fixture must be obviously not an
  execution, so nobody "fixes" a test by running the string. That
  double is not in the product binary.
