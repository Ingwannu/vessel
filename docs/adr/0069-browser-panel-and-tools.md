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
engine. The double that tests the contract is not part of a product
build.

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
   a live page. WebView2 is Chromium; the review names
   `CoreWebView2.CallDevToolsProtocolMethodAsync`, and that header was
   not opened here. WKWebView and WebKitGTK headers were not opened
   here either. The tools would still be two implementations, and the
   WebKit ones would not match Paseo. That conclusion is not a line of
   Paseo.
3. **Both engines.** Chromium for agents, a system webview for the
   person.

A Chromium Embedded Framework build inside the GPUI process was not
measured and is not an option here: it is a second copy of Chromium
plus a native windowing fight, and the standard library cannot paint
HTML.

## The choice

Option 1. One browser, owned by the node, spoken to over CDP when a
real engine exists. The app is a view of those tabs, not a second
browser. The first code slice (the plan's slice 2) is the tool surface.
Until a real engine exists, a product build answers `NoEngine` for
every command and does not advertise the tools. The in-memory double
lives in test code only. No Chromium, no webview, no new crate
dependency until a later ADR names the binary.

The method's protocol number is the number given at merge. This
document does not choose one. The base this branch speaks is 20
(`crates/vessel-proto/src/node.rs`, `PROTOCOL_VERSION`). The code will
gate the new method and the new settings field on one constant,
`BROWSER_PROTOCOL`, and leave `PROTOCOL_VERSION` at the base until
merge changes that one constant.

## Why this and not the others

Paseo's fact that matters is that the person and the agent share one
profile. Option 3 breaks it. A shared cookie file between Chromium and WebKit
was not read in a vendor tree; nothing in Paseo shows one, and Paseo's
own copy says one partition (`browser-profile.ts` line 1, and the
settings sentence in `en.ts` near the `browserData` block). Two
engines would be two profiles. That is the opposite of that sentence.

Option 2 cannot implement the twenty-two tools once on every platform
Vessel ships. File upload is `DOM.setFileInputFiles`, screenshots are
`Page.captureScreenshot`, clicks are `Input.dispatchMouseEvent`, and
the snapshot is script injected into the page. Whether WebView2
accepts those methods was not read in its header. WKWebView and
WebKitGTK headers were not opened; they are not Chrome, so macOS and
Linux would be a standing difference from Paseo unless a later note
shows otherwise. WebKitGTK is LGPL-2.1, the system library, not part
of `wry`. A webview beside GPUI is also a
separate native window, not a kit control, and `view` must not do I/O.

Option 1 keeps one profile and one command set. It differs from Paseo
in where the process lives, which is forced: Vessel has no Electron.
The differences are listed below, each with why.

The contract is settled before the engine because a real browser in
the first code slice would make the tests depend on a download. The
double that answers those tests is not a product engine: a node whose
setting is on must not tell an agent that a fixture is a page.

## Advantages, costs and effects

**Advantages.** Agents can use the tools on a node that has no app
open. One implementation of each command. The profile is a directory
the node already knows how to keep private. The app stays a client.

**Costs.** The person does not get a live page in the first code
slices. A later view is pixels or a remote surface, not a native
webview, so input methods and dragging a file in from the desktop are
weaker than Paseo's until that view is designed. The node becomes able
to run a browser that holds cookies and opens connections from the
node's machine. A scheme rule does not limit where those connections
go. The setting defaults off. The browser's environment must not
receive the node's token, device credentials or harness secrets.
TCP the HTTP proxy carries is held to the destination table below.
WebRTC UDP and QUIC are not that TCP; the engine closes them with
the switches in that section. Text an agent types is kept in the
session log, as the credential section says.

**Effects.** A new crate `vessel-browser`. Slice 2 has no process and
no product double: its product engine is the absence, and every
command on it is `NoEngine`. `vessel-proc` is an edge only when a real
process is spawned. Edges, recorded in ADR 0001 and
`docs/ARCHITECTURE.md` in the commit that adds them, not before:
`vessel-browser -> proto`, and later `-> proc`; `vessel-node ->
vessel-browser`. The destination check calls
`vessel_relay::reach::is_public` (the ranges already written there).
That is an edge `vessel-browser -> relay`, or those two functions
moved to a lower crate, decided in the engine slice so the range list
is not copied. `vessel-mcp` stays `-> proto, client` and gains no
browser crate. CDP, if it is ever a Rust crate, has one owner:
`vessel-browser`. No row is added now, because the boundary check
fails a table row with no manifest.

## What a page can reach

Paseo's webview may open any `http` or `https` URL, including loopback
on the person's machine (`isAllowedBrowserWebviewUrl` is a scheme
check). That machine is the one the person is sitting at. Option 1
runs the browser on the node, which is often a server. A page's script
can then try the node's own listeners, other loopback services, private
ranges, and link-local addresses such as cloud metadata, including by a
name that resolves there after it was checked (rebinding). An `http`
scheme does not stop any of that.

The real engine therefore does not let Chromium dial. It is started
with a proxy the node owns and with no bypass. The proxy allows only
`http` and `https`, resolves the host, and connects only to an address
`vessel_relay::reach::is_public` accepts, and only to the address it
just checked. Anything else is `Forbidden` with
`BrowserFailure::Destination`. The classification already refuses
loopback, private ranges, CGNAT, link-local (including
169.254.169.254), unique-local, site-local, multicast, reserved, and
the IPv4 forms embedded in IPv6. A numeric host that is not a canonical
address is refused the same way `numeric_host` refuses one. Chromium's
own private-network preflight is not the control: it was not verified
in the headless shell, and a literal address would not depend on it.

An HTTP proxy does not carry every socket a page can open. Two of
those paths were read:

- **WebRTC UDP.** With no policy, ICE sends UDP from the browser
  process, including to loopback, and the proxy never sees it. The
  engine passes
  `--force-webrtc-ip-handling-policy=disable_non_proxied_udp`.
  That switch overrides the preference
  (`content/public/common/content_switches.cc`, read: "Override
  WebRTC IP handling policy to mimic the behavior when WebRTC IP
  handling policy is specified in Preferences"). The value is the
  policy `disable_non_proxied_udp`: "WebRTC uses either UDP SOCKS
  proxying or will fallback to TCP proxying" (`WebRtcIPHandling.yaml`,
  read). Chromium then sets
  `enable_nonproxied_udp` false
  (`peer_connection_dependency_factory.cc`, the
  `kDisableNonProxiedUdp` arm, read). The allocator's comment says
  false "disables all UDP traffic until UDP-supporting proxy RETURN
  is available" (`port_allocator.h`, read), and the flags are
  `DISABLE_UDP`, `DISABLE_STUN` and `DISABLE_UDP_RELAY`
  (`port_allocator.cc`, read). This proxy is HTTP and speaks no UDP,
  so that return does not happen and UDP stays off. The same policy
  file's item caption says TCP uses "the public-facing interface".
  The control this ADR follows is the flag, not that caption.
  WebRTC TCP is built with the proxy-resolving socket factory
  (`services/network/p2p/socket_tcp.cc`, read), so it is the table,
  not this bypass. Whether that factory exempts loopback by default
  was not read; the launch still has no bypass.
- **QUIC (HTTP/3).** It is UDP to the origin. Whether a headless
  shell already skips QUIC when `--proxy-server` is an HTTP proxy
  was not read, so the engine does not rely on it. It passes
  `--disable-quic` (`network_switch_list.h`, `kDisableQuic`, read).
  WebTransport over HTTP/3 was not read. It needs QUIC, so the same
  switch is what this ADR relies on for that UDP. The engine slice's
  loopback UDP test checks WebRTC. It is not a QUIC server, and it
  does not by itself prove the QUIC switch was honoured.

The two switches are Chromium flags. On Windows they are the same
flags. There is no signal, no systemd unit and no launchd job. A
binary that rejects a flag fails the launch before the proxy is
treated as ready. The engine slice asserts that. The error is the
one `vessel-proc` already returns; its name was not re-read here.

| Class | Option 1 | How |
| --- | --- | --- |
| Loopback (`127.0.0.0/8`, `::1`) | Refused | Proxied TCP: the proxy, via `is_public`. WebRTC UDP and QUIC: the switches above, not the proxy. |
| The node's own ports | Refused while they sit on loopback, which is the default (`127.0.0.1:7077`, the model proxy, `vessel ui`) | The same. A node bound to a public address is a public destination; the listeners below still have to authenticate. |
| Private, CGNAT, unique-local | Refused | Proxied TCP: `is_public`. WebRTC UDP and QUIC: the switches. |
| Link-local and metadata | Refused | Proxied TCP: `is_public` (169.254.0.0/16, `fe80::/10`). WebRTC UDP and QUIC: the switches. |
| Public | `http` and `https` TCP only | The proxy. WebRTC UDP and QUIC stay off, including toward a public host. |

An agent's shell is a different door. It exists only where both
profiles set `agent_terminals`, which is off by default (ADR 0031),
and when it exists it is a process on the node: loopback, private,
link-local and public are all in reach. Turning browser tools on does
not turn that shell on, and the browser does not inherit the shell's
reach. An agent that has both can still use the shell to open what the
browser refuses. The proxy is not a substitute for leaving the shell
off.

What those listeners do with a page's `Origin`, read in this tree:

- `vesseld` (`crates/vessel-node/src/server.rs`) serves `GET /health`
  with no token and no `Origin` check. `GET /rpc` requires
  `Authorization: Bearer` before any frame (`auth.rs`). This server
  sends no CORS header, so a page script cannot attach that header or
  read the response. `GET /secure` takes no token; the device proves
  itself inside the channel. No `Origin` check was found on this
  upgrade.
- The model proxy refuses a request that carries any `Origin` before
  it looks at the token (`crates/vessel-node/src/models/proxy.rs`,
  `admit`: "requests from web pages are refused"). `Host` must be
  `127.0.0.1` or `localhost` at the proxy's own port.
- `vessel ui` (`crates/vessel-web/src/guard.rs`) answers only a
  loopback name with its port, else 421. An `Origin` that is present
  must be that server's own origin. A websocket with no `Origin` is
  refused. No CORS header is sent.

The proxy keeps proxied TCP inside the table. The switches keep
WebRTC UDP and QUIC from leaving beside it. The listeners are the
second line if a launch ever skipped the proxy, which the engine
slice must not allow. They do not see a UDP datagram.

## Where a credential lives

A password is typed in the app, or sent by an agent as `browser_type`,
`browser_fill` or `browser_keypress`. Those bytes are fields of
`browser.call`. On a paired connection, including one that goes through
a relay, the frame is inside the Noise channel (ADR 0009): the relay
forwards ciphertext and does not see the field. On the node's local
socket the field stays on that machine. The node then hands the text
to the page (CDP insert or a fill) and does not keep a buffer of
its own after that hand-off. The session event is another copy, and
it stays.

The browser handler does not write a second copy of that text, of an
evaluate function, of `result_json`, or of a screenshot's bytes: not
as a tracing field, and not as a file beside the profile. That is
not the only copy, and this ADR does not pretend it is.

An agent sends the text by calling `browser_type`, `browser_fill` or
`browser_keypress`. Those are MCP tool calls. The harness reports
each call with its arguments, and the node stores the session event
as the harness sent it. `ToolCall.raw_input` is "The tool's raw
input arguments, forwarded and never interpreted"
(`crates/vessel-proto/src/agent.rs`). The session log is that event
(`crates/vessel-store/src/log.rs`, the `events` table). Nothing in
this design removes the text from that event. Stripping it would
hide a tool call the node already stores for every other tool.

Paseo's Claude mapper puts the same text on the timeline item.
It takes `raw.input` onto the detail
(`packages/server/src/server/agent/providers/claude/tool-call-mapper.ts`,
read). A tool name with no built-in kind, which is where `browser_*`
falls (no `browser_` arm in that parser), is returned as
`type: "unknown"` with that input
(`tool-call-detail-parser.ts`, the final return, read).

The message search index does not read a tool call's arguments
(`crates/vessel-store/src/search/said.rs`: `ToolCall` ends a run and
is not a message). `session.export` copies the harness's files
(`crates/vessel-node/src/transfers/export.rs`, read). Those files
were not opened here, so this ADR does not say they are free of the
text.

Nothing is redacted. The `browser_tools` description, and the
descriptions of `browser_type`, `browser_fill` and
`browser_keypress`, say that text passed to those tools is kept in
the session log as the tool call's input. A password typed only in
the app, as `browser.call` and not as one of those tools, is not
such an event. The handler still does not log that field. The page
and the profile may hold what was typed. A screenshot returned to
the caller can still show a password the page painted.

The profile is one directory, `browser/` under the node's data
directory. The directory is mode `0700` and the files `0600`, the same
rule as the data directory (`crates/vessel-store/src/files.rs`): the
owner is the user the node runs as. On Windows there are no mode bits;
the directory inherits the data directory's ACL and Vessel sets no
extra ACL, as it does for the rest of that directory.

Who shares it is a stated difference. Paseo's partition is one desktop
app, used by the person at that machine. Vessel's profile is the
node's: every workspace, every session, and every device or agent that
can make `browser.call` (a grant that includes `operate`, and a session
that was shown the tools) shares the cookies. A device with only
`read` cannot call the method. An agent's grant is its creator's,
shrunk to at most `read` and `operate`, never `admin`
(`crates/vessel-node/src/agents/mod.rs`).

## Contract of the first code slice

Slice 2, after this ADR is accepted. Not built here.

### Wire (number given at merge)

One method, not twenty-two. The command is a closed enum, matched
exhaustively. Paseo already has one execute request; a method per tool
would restate the list (QUALITY M5).

- `browser.call` / `browser.call` / `"browser.call"` `[Operate, since
  = BROWSER_PROTOCOL]`: `BrowserCall` => `BrowserResult`. The constant
  is the number given at merge, not a number chosen here.
- `NodeSettings.browser_tools`: a two-variant enum `Off` (default) and
  `On`. `#[serde(default)]`, omitted on the wire when `Off`, so a node
  of protocol 20 and a saved `settings.json` from protocol 20 both read
  as off. Same pattern as `drivers` (protocol 18).
- `SetNodeSettings.browser_tools`: `Option`, absent means unchanged.
  Needs `admin`, as every other setting change does. `node.settings`
  stays `read`.

**Deadline.** Every command has the broker's deadline: 15 seconds
(`BrowserToolsBroker`, default). `browser_wait` is the exception Paseo
already makes: when the caller sets `timeoutMs`, the deadline is that
value plus one second, and `timeoutMs` is at most 30 seconds, so the
deadline is at most 31 seconds. Past the deadline the answer is
`TimedOut`. The test does not sleep; it passes a deadline the test
chooses and a double that never answers.

**Older and newer, both ways.** The comparison is with a peer whose
protocol is below `BROWSER_PROTOCOL`, whatever number merge assigns.
This ADR adds one settings key, `browser_tools`. It does not list any
other key. `drivers` is already in `settings.rs`; a key another track
adds is that track's ADR, added at merge, and is not classified here.

- A client below `BROWSER_PROTOCOL` never sends `browser.call`
  (`Call::since`). A node at that constant, asked by such a client,
  does not run it. A node below the constant has no method; the client
  refuses before sending.
- A client below the constant decoding `node.settings` from a newer
  node ignores `browser_tools` (serde's default on this struct: an
  unknown field is not an error). It cannot turn the tools on.
- A client at the constant reading an older node sees the field absent
  and treats it as off. It does not send `browser.call`.

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
| `browser_denied` | `Forbidden` | `Scheme` | `navigate` to a URL that is not `http` or `https`. Paseo answers this inside the host (`service.ts` `executeNavigate`, "Browser navigation only supports http and https URLs."). The wire type already makes that URL unrepresentable, so the edge answer is `Invalid`; `Scheme` is what the engine answers if a navigation still reaches it, including one the page itself starts. Paseo's `will-navigate` guard drops those with no tool error; Vessel answers `Scheme` instead of dropping one. |
| `browser_denied` | `Forbidden` | `NoHistory` | `back` or `forward` when that stack is empty (`service.ts`, "There is nothing to go back to." / "There is nothing to go forward to."). |
| `browser_denied` | `Forbidden` | `Destination` | The proxy refused the address (the table above). Paseo has no such refusal. Stated difference. |
| `browser_no_host` | `Unavailable` | `NoEngine` | The setting is on and this build has no real engine. Every command, not a fixture. |
| `browser_tab_not_found` | `NotFound` | `UnknownTab` | No such tab in this workspace. |
| `browser_tab_closed` | `Conflict` | `Closed` | The tab was closed. |
| `browser_stale_ref` | `Conflict` | `StaleRef` | The ref is not from the tab's latest snapshot. |
| `browser_timeout` | `Unavailable` | `TimedOut` | The deadline above. |
| `browser_unsupported` | `Unsupported` | `Unsupported` | A command this engine does not implement. The test double can be built with a subset; the product absence answers `NoEngine`, not this. |
| `screenshot_no_frame` | `Unavailable` | `NoFrame` | The real engine, when the tab has not painted. The test double may return it. A product build with no engine does not pretend to paint. |
| `browser_unknown_error` | `Internal` | `Internal` | A bug. Not used for a bad argument. |
| (validation) | `InvalidRequest` | `Invalid` | A bad id, URL, ref, size, wait or path. |

Paseo's `retryable` boolean is not a wire field. `Unavailable` is the
retryable class; the others are not. Stated difference, under that
heading.

### Types (no bare strings across a module)

Parsed once at the MCP and RPC edge.

- `BrowserId`: Paseo's two shapes, UUID version 4 or
  `<13 or more digits>-<hex>` (`rpc-schemas.ts` lines 14 to 16). The
  test double mints a UUID version 4. The product absence mints
  nothing.
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
  difference, so a page cannot make the node hold an unbounded image.
  The test double may return a fixed 1×1 PNG. A product build must not.

`browser_evaluate`'s result is a string the node does not parse
(`result_json`), never a `serde_json::Value`. The test double does not
execute the function. It returns a deterministic JSON string that
echoes the function's length and whether a ref was passed, with
`truncated: false`. A product build with no engine returns `NoEngine`
instead, and a test asserts that it does.

### Tools, and when the list is decided

The MCP names are Paseo's. Arguments match the plan's table. Each tool
is one `browser.call` on the session's own node. Paseo registers them
only when the agent starts and the setting is on (`paseo-tools.ts`
lines 1358 to 1359, read). ADR 0065 (added at merge) filters Vessel's
catalogue at that same moment: a tool left out is absent from
`list_tools`, and a call to it is a refusal that names the tool, not a
success and not a silent skip. Browser tools are outside ADR 0065's
catalogue until this ADR's code lands; they are not a denylist entry
before they exist.

At session start the twenty-two names are included only when all three
hold: `browser_tools` is `On`, the product engine is a real one, and
the principal's grant includes `operate`. Otherwise they are absent.

- Started while the setting was off: the names stay absent for that
  session even if an admin turns the setting on later. A `browser.call`
  is `Disabled`.
- Started while they were included, and the setting is turned off
  later: the list the agent already holds is not rebuilt (Paseo does
  not rebuild it either; the registration is in the start path, and no
  second registration was read). The call is `Disabled`.
- Setting on, no real engine: the names are absent. A call is
  `NoEngine`, unless the setting is off, which wins and is `Disabled`.
- No `operate`: the existing scope guard answers `Forbidden` before
  the browser runs. The names are not shown.

When the names are shown, the tools are marked open-world.
`browser_evaluate`, `browser_navigate`, `browser_new_tab`,
`browser_upload` and `browser_click` are also marked destructive.
Paseo does not set those MCP flags; stated difference, so a client
that heeds the flag asks before running them.

`list` and `new` require the session's workspace, else `NoWorkspace`.
Every other command requires a tab of that same workspace, else
`UnknownTab`. An agent with no workspace can call none of them. The
node token and a device with `operate` may name a workspace; without
one, `list` and `new` are `NoWorkspace` too. Tabs are not global.

The test double, and only the test double, keeps tabs in memory per
workspace: url, title, history for back and forward, the last
snapshot's refs, a viewport, console and network rings of the
requested limit, and closed versus open. It does not open a socket,
read a cookie, or touch the disk except the upload check, which
resolves the paths against the session's workspace root and refuses
one that escapes (`Invalid`), without reading the file. `new` does
not change which tab it calls active (Paseo opens with intent
`background`, `handler.ts` lines 361 to 364, read). Omitted URL is
`https://example.com` (`handler.ts` line 349, read). `back` on a new
tab and `forward` with an empty stack are `NoHistory`. A navigation
the double is given that is not `http` or `https` is `Scheme`.

It is compiled only under `cfg(test)`, or under a feature the release
build cannot enable. The product engine in slice 2 is the absence.

### What slice 2 tests

On the product engine, with the setting on: the twenty-two names are
absent from `list_tools`, and every `browser.call` is `NoEngine`, not
a fixture page, not a 1×1 image, and not an evaluate echo. With the
setting off: the names are absent and the call is `Disabled`. A
session started while off stays `Disabled` after the setting is turned
on. A session started while on, with a test engine injected, gets
`Disabled` after the setting is turned off.

On the test double only: each command's happy path, `NoHistory`,
`Scheme`, a ref that survives until the next snapshot and dies after
it, an upload path outside the workspace, a viewport of 0 and of
8,193, a wait with both text and url, and a double that never answers,
which is `TimedOut` at the deadline the test passes. `Destination` is
tested with the proxy, in the engine slice, not by pretending the
product answered it. That same slice binds a UDP socket on
`127.0.0.1`, has the page gather ICE toward it, waits until gathering
finishes, and asserts the socket received nothing. Slice 2 has no
real browser and does not run that. No sleep. No real browser in
slice 2. No network. The
settings field round-trips as absent when off, against a peer below
`BROWSER_PROTOCOL`, with the constant set to a number in that test
only.

## Stated differences from Paseo

1. **The browser process lives on the node, not in the desktop app.**
   GPUI cannot host Paseo's webview, and WKWebView and WebKitGTK cannot
   implement the tools. Consequence, once a real engine exists: tools
   work with no app connected. Paseo returns `browser_no_host` in that
   case (`broker.ts` `noBrowserHostFailure`, read). Vessel has no such
   host. Until the engine exists the product answer is `NoEngine`, not
   a page.
2. **The tool list is fixed at session start, and a later call is still
   refused.** Paseo only omits the registration when the agent starts
   (`paseo-tools.ts` 1358 to 1359). Vessel also answers `Disabled` if
   the setting is off at the call, including after it was on at start,
   so a guessed call cannot succeed unseen. Reason: a typed refusal
   rather than a silent success.
3. **`retryable` is not a field.** `Unavailable` means retry; the other
   codes do not. Reason: the wire already has that class (`ErrorCode`),
   and a second boolean would not say more than the code.
4. **`bool` arguments on the wire are enums** (`ClickCount`,
   `ShotExtent`). The MCP edge accepts Paseo's booleans and converts
   them once. Reason: a bare `bool` does not say which meaning it has
   (T9).
5. **Viewport sides above 8,192, and screenshots above 8 MiB, are
   refused.** Paseo accepts any positive size (`resident-webviews.ts`
   `rememberBrowserWebviewSize`, read: round to at least 1, no maximum)
   and any PNG string (`rpc-schemas.ts` `dataBase64` is `min(1)` only,
   read). Reason: the node must not hold an unbounded image or
   viewport.
6. **Non-public destinations are refused** by the proxy
   (`Destination`), and WebRTC UDP and QUIC are turned off at launch
   because an HTTP proxy does not carry them. Paseo's scheme check
   allows the destinations. Files under `packages` whose names contain
   `browser` do not mention these switches (searched). Other Paseo
   files were not searched for them. Reason: the browser runs on the
   node, not on the person's desktop. The table is above.
7. **One profile for the whole node**, not for one desktop the person
   is sitting at. Reason: there is no Electron session. Owner, mode and
   who shares it are above.
8. **The test double does not execute JavaScript or paint, and it is
   not in the product build.** A product build with no engine answers
   `NoEngine`. Reason: a fixture must not be reported as a page.
9. **Destructive and open-world flags** on the tools named above. Paseo
   does not set them (not read as annotations on those registrations;
   the registrations in `tools.ts` that were read set title,
   description and `inputSchema` only). Reason: a client that heeds the
   flag can ask before a tool changes a page.
10. **The aria snapshot will be written for Vessel, not copied** from
    Paseo or from Playwright. The observable contract (YAML, the caps in
    the plan, refs `@eN`, stale after a navigation) is what must match.
    Reason: the script in Paseo's tree is adapted from Playwright and
    is under that licence; Vessel does not copy it.
11. **`Scheme` is answered, not dropped,** when a navigation that is not
    `http` or `https` reaches the engine. Paseo's `will-navigate` guard
    calls `preventDefault` and returns no tool error
    (`browser-webviews/index.ts` `preventUnsafeBrowserWebviewNavigation`,
    read). Reason: a typed refusal rather than a silent drop.
12. **The process is not signalled.** When the engine exists it is a
    child through `vessel-proc`: a process group on Unix, a
    kill-on-close job object on Windows (the `vessel-proc` row of
    `docs/ARCHITECTURE.md`, read). No systemd unit and no launchd job.
    Windows is that job object, not a refusal.
13. **Typing tools warn that their text stays in the session log.**
    Nothing is stripped from `raw_input`. Paseo's mapper puts an
    unknown tool's input on the timeline item (cited above). The
    disk write of that item was not re-read. The switch warning
    already in this ADR is about logged-in state, not this sentence.
    Reason: the node must not say that no copy remains.

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
