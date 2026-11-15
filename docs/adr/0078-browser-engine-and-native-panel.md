# ADR 0078: A Chromium engine and a native view of the same tabs

Status: contract accepted; implementation and runtime qualification pending.
No engine, new call, or visible entry is enabled by this document.

## Purpose and intent

Give a person a browser panel in the native app and give their agents the
same twenty-two browser tools as Paseo. Within General or a particular Preview binding, a sign-in must belong to
the page both are driving. A separate front-end engine with another cookie
store would be misleading; Preview isolation is an explicit security boundary.
The browser tools default off. The panel must never display a test fixture
as a page the engine actually loaded.

## Existing implementation and constraints

Paseo 0.15.4 uses an Electron webview on the desktop, a persistent shared
partition, resident background views, and a daemon broker. The reference
inventory with source lines is [the package plan](../plans/wp-browser.md).
Non-Electron web and mobile clients do not host its browser. Its desktop
executor uses CDP, trusted input, a page accessibility snapshot, and scoped
upload paths. Its default-off switch warns about access to signed-in tabs.

Vessel's native client uses GPUI, with pure models in view, node I/O in link,
controls in kit and screens in app. No browser exists on the package's app
base. The current node head also has no registered browser call. The earlier
ADR 0069 chose a node-owned browser but left the engine to a later decision.
Its initial security return was corrected at the document level: destination
classification, credentials, test-only doubles, deadlines, and bypass traffic.
A document pass is not evidence that an engine enforces those controls.
This ADR completes that choice; it does not silently replace its public-only
network policy or its warning about text in session logs.

The previously drafted BrowserCommand/BrowserFailure value types and absent
engine are reusable candidates, not an integrated service. They must be
reviewed against the current protocol before adoption. No parallel RPC or
second setting is introduced to avoid that integration.

## Alternatives considered

| Engine | Linux | Windows | Cost and reason |
| --- | --- | --- | --- |
| System webview in a child window, through wry | WebKitGTK; GTK event loop; raw child integration is X11-only; Wayland requires a GTK container | WebView2, a Chromium engine with a CDP API | Native composition and accessibility are attractive; Linux would need a second automation implementation and coordination with GPUI's event loop. |
| One headless Chromium, controlled through CDP, with its frames drawn in GPUI | Same command implementation; no embedded GTK loop | Same CDP implementation; child lifetime through a Windows job object | Large optional runtime; pixels require explicit input and accessibility work. One profile and executor. Chosen. |
| System webview for people and headless Chromium for agents | Two engines | Two engines | Rejected: a person can sign into the wrong engine; cookie synchronisation is not a safe substitute for one profile. |
| Chromium Embedded Framework | Native embedding work | Native embedding work | Additional large bundled runtime and platform integration, not needed to render the CDP surface. Not adopted. |

WebView2 does expose CDP; the reason to reject the first option is Linux's
WebKit backend and event-loop integration, not a claim that WebView2 lacks
CDP. The wry vendor documentation describes these platform limits:
<https://docs.rs/wry/0.57.2/wry/>. No wry dependency is added.

## Choice

Use **node-owned Chromium in unified headless mode**, one implementation
with two profile classes: persistent General and isolated session-bound
Preview. Both agents and the person operate the same CDP target within its
class. The Preview capability below is the only private-origin exception. The GPUI pane is a view of those targets,
not a system webview. A node needs no desktop app connected for automation.
That is an explicit difference from Paseo's desktop-host broker.

The engine is an optional, explicitly configured, absolute executable path
to an operator-installed Chromium distribution on Linux or Windows. No PATH
search, executable from a workspace, automatic download, automatic update,
attachment to the person's existing Chrome, or reuse of their Chrome profile.
The runtime's version and file size are recorded during setup without
recording a credential. Unconfigured, incompatible or failed engines are a
typed NoEngine/Unsupported/Launch refusal. Never start with --no-sandbox.
A root process is refused rather than disabling Chromium's sandbox.

No new third-party crate is selected: use the workspace's existing tokio,
tokio-tungstenite, serde, bytes, url, base64 and HTTP components for a narrow,
typed CDP subset. Reuse vessel-proc for supervised lifetime. The standard
library does not implement Chromium, TLS or websocket framing. Chromium is
not bundled: its binary bytes add zero to Vessel's installer, its installed
size depends on the distribution and is not yet measured. Chromium's root
license is BSD-3-Clause; a distribution includes separately licensed
third-party components. The runtime's actual notices must be inspected and
retained before a distribution is certified or redistributed. This ADR
permits no redistribution or installer until that inspection and size record.
Source: <https://chromium.googlesource.com/chromium/src/+/main/LICENSE>.

Unified headless mode creates no visible browser windows on either platform:
<https://developer.chrome.com/docs/automation-and-testing/headless>.
A configured binary is not considered supported merely because it accepts
unknown flags. Readiness requires the versioned runtime qualification below.

## Process and security boundaries

The node owns a browser actor and its policy proxy, one bounded command queue,
owned child process trees, profile locks, CDP connections and bounded event decoders.
A Preview capability gets a separate ephemeral profile and policy proxy; it
cannot share Chromium storage/network context with General or another session.
No raw CDP method, websocket endpoint or arbitrary browser launch argument is
part of the public API. The node sends typed commands after authorization.
The browser process has Chromium's normal sandbox and no Node/Electron bridge.
This is not an OS sandbox for an agent that already has a general shell.

Launch uses vessel-proc: a process group on Unix, a kill-on-close job object
on Windows. Environment is cleared; only platform runtime necessities and
private home/temp locations are supplied. No node token, harness environment,
SSH agent, device key, inherited proxy configuration or real user's profile.
CDP binds an ephemeral loopback port, never a wildcard or public port. Its
random target endpoint is private control information, never returned or
logged. The policy proxy refuses that endpoint and all node-local addresses.
Local hostile processes running as the same OS account remain in the trust
boundary; CDP is not authenticated against that account. It must not be
published through a reverse proxy, port forwarding or service discovery.

Before launching, inspect the execution account, profile parents, permissions
and exclusive lock. Unix profile directories are 0700 and files created by
Vessel are 0600; run the child with private creation permissions and verify
files Chromium created. Windows requires an owner-restricted data-directory
ACL verified under the service identity; an absent or unverifiable private
ACL is a typed launch refusal, not a claim that Unix modes apply on Windows.
No recursive permission repair of an existing arbitrary profile. Fail and
explain the configured profile issue. No service restart or live data is
required for development or tests.

Every accepted call is serialized per target. Closing/restarting the engine
invalidates target ownership and element generations before queued work can
run. Revocation/settings-off cancels admitted work at the next boundary;
an action already delivered to the page cannot be undone or blindly retried.
Timeout of a mutation is an uncertain outcome; it does not authorize replay.
Shutdown joins bounded tasks and stops only the owned child tree.

### What a page may reach

HTTP(S) traffic, including subresources, redirects, service workers, websocket
upgrades and CONNECT tunnels, uses a node-owned explicit HTTP proxy with no
DIRECT fallback. Disable Chromium's implicit loopback bypass with
--proxy-bypass-list=<-loopback>. Resolve each outbound connection in the proxy,
classify the resolved addresses using the existing vessel-relay reach policy,
and dial only the address just checked. DNS rebinding must not introduce a
second unchecked resolution. Reject ambiguous numeric hosts and userinfo.
A mixed DNS answer containing forbidden addresses is refused as a whole.

| Destination | Rule | Enforcement |
| --- | --- | --- |
| Loopback, including the node's own listeners and CDP | Refused in General; one exact service origin in a live Preview capability | Resolve-and-pin proxy; implicit bypass removed; node-minted service capability below. |
| Node's local interface addresses, even public ones | Refused | Snapshot local interface addresses before launch and refresh on changes; fail closed if discovery fails. |
| Private, CGNAT, unique-local | Refused | Existing reach classification applied at every dial. |
| Link-local and metadata, including mapped IPv4 | Refused | Existing reach classification; never a workspace-preview exception. |
| Public HTTP(S) | Allowed, subject to the same proxy | Checked TCP address, normal certificate validation; no TLS interception. |
| Any direct UDP, QUIC, WebRTC, WebTransport | Refused | Runtime switches plus qualification tests below; HTTP proxy alone is not enforcement. |
| file, data top-level, javascript navigation, external handlers | Refused | Typed URL parsing and interception of page navigations/popups; no external launcher. about:blank is an internal empty target only. |

For unified Chrome use
--webrtc-ip-handling-policy=disable_non_proxied_udp and --disable-quic.
The older --force-webrtc-ip-handling-policy spelling is not sufficient for
this engine: the real-runtime experiment below observed direct UDP with it.
Do not certify a launch merely because Chromium silently accepted the flag.
The proxy never offers UDP. WebRTC TCP must also pass the destination policy;
a loopback TCP ICE test is required. A switch being present on argv does not
prove it works: qualify actual browser traffic before marking this runtime
ready. Proxy failure closes the browser rather than falling back to direct
networking. Flags are a browser configuration boundary, not a guarantee
against a compromised Chromium network process or an arbitrary same-user
program. A runtime unable to enforce the policy remains NoEngine.

Paseo's desktop scheme guard permits the desktop's private network. Moving
that reach onto a server would expose server-local APIs and cloud credentials.
An agent's separate shell may already have unrestricted node networking; this
browser opt-in neither enables that shell nor claims to confine it.

The node's existing health route has no authentication/Origin check; /rpc
requires its bearer token, /secure requires the encrypted device handshake,
and no Origin check was found on that upgrade. The model proxy rejects any
Origin; vessel ui checks Host and Origin, and requires Origin on websocket.
Those are not substitutes for blocking the browser's network path. The page
must not obtain any of those credentials from the browser environment.

### Workspace-service Preview capability

Project dev-server preview is part of the first engine, not deferred. The
node mints a non-transferable Preview binding only for a service belonging
to the requesting workspace and opening session. It is not a private-host
allowlist and it cannot be supplied as a bearer URL/query value.

- The request identifies the opening SessionId and a detected service
  candidate. The node derives project/worktree identity from its registry,
  not caller cwd, a hostname, a process name or an arbitrary port. Agents
  are pinned to their authenticated session; a human names a live session
  in the selected workspace. A workspace with no opening session refuses
  until one is selected; it does not create an unbound capability.
- Eligible candidates come from a complete successful current port scan,
  joined with live session/terminal ownership and the workspace registry.
  A script's status/declared port alone is not proof that it owns a socket.
  Both the owner and the selected session must resolve to the same exact
  workspace, including worktree. A port from another project/worktree or
  an ended/archived owner cannot mint a binding. General browser/CDP/proxy,
  node control, model proxy and management listeners are never candidates.
- Mint an opaque internal capability id, workspace, opening session,
  service owner lifetime/generation, exact loopback IP, port and HTTP/HTTPS
  scheme. Host is a canonical loopback literal, never a DNS alias, wildcard
  interface, private subnet or metadata address. An unspecified listener
  can be dialled through loopback only after ownership has been proved.
  HTTP versus HTTPS is verified by a bounded node probe to that exact
  candidate; no automatic certificate bypass or fallback to another scheme.
  port.list currently reports TCP and cannot attest the scheme by itself.
- General browser_new_tab may recognize a loopback service URL and request
  this binding, using the agent's own session; the panel's View service
  uses the same mint path through human control. browser_navigate on a
  General tab never upgrades its network permission. It explains that a
  separate Preview tab must be opened. No extra agent tool name is added.
- The capability is actor state associated with the target, not a secret
  exposed to the page, MCP arguments or persisted layout. Every command
  rechecks session/workspace ownership. A different session in the same
  workspace cannot operate it; humans viewing it must still hold Operate
  and name its original opening session. Restoring layout never remints it.
- Its dedicated profile is ephemeral, owner-private and shared only by
  targets with that same binding. It shares no cookies, storage, HTTP auth,
  service workers or cached credentials with General, another Preview
  binding or another session. Popups inherit the original binding/profile.
  The proxy permits public destinations plus precisely the bound origin.
  ws/wss upgrades map to that same HTTP/HTTPS origin; all other private
  origins are Destination. No localhost alias, other loopback IP, new port,
  changed scheme or redirect grants a new permission. Apply this to every
  resource/frame/worker and CONNECT, not just the address bar.
- The entire Preview browsing context can access that one service origin,
  including a public page it navigates to. This is intentional capability
  reach, not a CORS exemption; Chromium's normal origin rules still apply.
  Public navigation does not gain access to General's cookies. A trusted
  agent that can read both profiles can deliberately copy data; profile
  isolation does not claim to prevent that authorized agent action.
- Revoke on service/terminal/session stop, archive, workspace move/removal,
  owner-generation change, scan failure/truncation/unknown ownership, or
  device revocation. Lifecycle events revoke immediately when available;
  while a binding exists, retain the scanner's interest. Define
  PREVIEW_LEASE as twice the existing scanner PERIOD, renewed only by
  a successful complete scan; expiry revokes rather than silently extending. Revalidate ownership before
  each connection and again after connect, before sending request bytes.
  An existing connection is pinned to its accepted socket, not redialled.
  Stop closes active tunnels/websockets, cancels queued actions, stops
  owned Preview children, clears its temporary profile and invalidates refs.
  A replacement service on the same port requires a fresh explicit open.
- The existing cached port.list revision and PID alone do not establish
  an owner lifetime. The implementation must add a typed owner-generation
  observation and cancellation hook to the scanner/terminal boundary.
  PID/port reuse between scans is tested. An unsupported or incomplete
  platform observation is a typed refusal; no mint from stale cached data.
  Same-OS-user hostile processes remain outside the isolation guarantee,
  as they already share the CDP/control account trust boundary.

Paseo opens service URLs through public/paseo/direct route selection and
its desktop browser's shared partition. It has no such node-minted,
session-bound network capability in the sources inspected. Isolating Preview
sign-ins, requiring an opening session, refusing unrelated private origins
and revoking on stop are deliberate security differences caused by moving
the browser onto the node. The node never creates a service or forwards a
new public port merely because someone asks to preview it.

### Credentials and retained content

A password typed by a person into the GPUI page travels through link,
browser.control, and the node to CDP. Direct paired and relayed connections
use Vessel's encrypted channel; the relay sees ciphertext. A local owner
connection has the existing local transport trust boundary. Input is never
a tracing field, request Debug dump, error, separate journal or app history.
Do not persist page frames, evaluate results or human input in layout state.
Chromium and the site may retain the value and authentication cookies.

Text in an **agent** browser_fill/browser_type/browser_keypress is a harness
tool argument. ToolCall.raw_input is persisted in the session event log.
The browser service cannot promise to redact a copy that arrived through
that separate path. Descriptions and settings help must state that text
passed to these tools stays in the session log. Snapshots, logs, screenshots
and evaluate can also expose signed-in data to the agent and its provider.
Search does not index tool arguments; session export copies harness files,
whose secret content was not inspected. No claim of export redaction.

The General node profile is shared by workspaces, sessions and authorized
devices. Preview profiles follow the separate capability lifetime above.
A workspace check confines tab ownership, not cookies: opening a new tab for
a signed-in site may expose that shared sign-in. The enable warning must say
this. A Read-only device cannot view frames or operate tabs. Profile clear is
Admin and direct-human-only, stops the engine, invalidates all tabs/refs and
clears the complete dedicated profile before reopening. Partial clear is an
error, not a success toast. No arbitrary profile path reaches a delete call.

## Protocol and agent delivery

One browser.call contains a closed BrowserCommand enum covering the table
below. The node resolves an agent's workspace from its authenticated session;
an agent cannot substitute space/project/cwd. Human calls name the workspace
and require Operate. All tab calls verify ownership before the target lookup.

The sole settings key remains browser_tools (Off by default), Admin to change.
Tools are offered at session start only when enabled, a qualified engine is
ready, and the principal has Operate. A session begun while off does not gain
the list when switched on. Calls recheck the setting and session admission;
switch-off refuses old listed calls. Engine loss is NoEngine, never a fake.
The person may use the panel with agent tools off; human control uses a
separate direct-human-only browser.control call with the same engine policy.
An agent or delegated origin cannot use that method to bypass browser_tools.

Planned additional typed methods: browser.control (human commands, including
pointer/key input, stop and element selection), browser.watch (Operate,
snapshot plus bounded frame/state subscription), browser.clear_profile
(Admin, direct human only). New methods and new settings fields remain
unregistered until the integrator supplies the common method-table/accessor
shape and a protocol assignment. BROWSER_PROTOCOL is Option<ProtocolVersion>
None until assignment; never reuse a current version or manufacture a number.
Older clients cannot send these calls, newer clients refuse an unsupported
node before sending, and an absent browser_tools field means Off. Unknown
results/failures use the existing version-aware decoding policy, not success.

Paseo's host registration/affinity is not reproduced: the engine belongs to
the selected node, so browser ids are scoped by node plus workspace. Node
reconnect refreshes its inventory; engine loss shows a refusal. Nothing may
reinterpret a browser id from another node as a local CDP target.

## Tool mapping, one by one

Arguments and defaults follow server/browser-tools/tools.ts. MCP accepts the
reference booleans and converts at the edge to domain enums. Named bounds
live beside their types, not duplicated in each tool handler.

| Paseo MCP tool | Engine operation and outcome |
| --- | --- |
| browser_list_tabs | Workspace-filtered actor inventory; no workspace is NoWorkspace. |
| browser_new_tab | New CDP target, optional URL default https://example.com; background in layout, responsive size initially 1280x800 while unshown. |
| browser_snapshot | CDP accessibility/DOM data projected to readable YAML with @eN refs; stable within the snapshot generation, invalid after navigation or replacement. |
| browser_click | Resolve latest ref, await visible/enabled/stable/hit-testable state, dispatch trusted mouse events; left/single defaults, modifiers retained. |
| browser_fill | Resolve editable ref, focus and replace content through page input operations; empty value clears. |
| browser_wait | Exactly one text or URL predicate; default 5 seconds, explicit maximum 30 seconds; bounded DOM/navigation observer wait. |
| browser_type | Optional ref then trusted text input into focus; input text remains a tool argument. |
| browser_keypress | Optional ref, parsed key/chord, trusted key down/up; unsupported key is typed Invalid. |
| browser_navigate | Parse tool URL (scheme-less becomes http), proxy-check destination, navigate; clear refs. |
| browser_back | Read history and navigate to prior entry; empty stack is NoHistory(Back). |
| browser_forward | Read history and navigate to next entry; empty stack is NoHistory(Forward). |
| browser_reload | Reload current target and clear refs. |
| browser_screenshot | PNG, viewport by default or full page; no painted frame is NoFrame; MCP image plus metadata with no duplicate base64. |
| browser_upload | Resolve workspace files into private staged regular-file copies, then DOM.setFileInputFiles; no arbitrary host path or symlink escape. |
| browser_select | Resolve select control, choose value, dispatch input/change; absent option is typed Invalid. |
| browser_hover | Latest ref, actionability and trusted mouse move. |
| browser_drag | Validate both refs first; trusted mouse movement from source to target with held button. |
| browser_logs | Bounded console/network history, default 50, requested count 1..200; typed records, no tracing copy. |
| browser_evaluate | Run supplied page function, optional element argument; typed CDP edge, bounded opaque result JSON; never a fixture echo. |
| browser_scroll | Optional ref, finite deltas, trusted wheel event. |
| browser_resize | Positive bounded CSS dimensions; update device metrics, frame and retained viewport together. |
| browser_close_tab | Close target and all watches; layout closure event, stale calls return Closed/UnknownTab. |

Default command deadline is 15 seconds. Explicit wait uses its timeout plus
one second, at most 31 seconds. Actionability and first painted screenshot
have 5-second deadlines inside the outer deadline. Keep Paseo's 80,000 text
cap, 79,000 evaluate preview, 2,000 evaluate error cap, 200 log entries and
20 dialogs per command. Alert is accepted, confirm/prompt/beforeunload are
dismissed; summaries report handled dialogs. A swallowed popup or dialog
must not be reported as successful navigation.

Reuse ADR 0069's typed failures: Disabled, NoWorkspace, Scheme, NoHistory,
Destination, NoEngine, UnknownTab, Closed, StaleRef, TimedOut, Unsupported,
NoFrame, Internal, Invalid; fixed safe messages. Add typed Busy (Unavailable)
for bounded admission, Launch (Unavailable) with a retained internal source,
and ProfilePermissions (Forbidden) for an unsafe profile. None of those
messages may expose a command payload or an arbitrary filesystem error. Preserve their mapping to
Paseo's ten error codes in the MCP summary. An uncertain mutation timeout
must additionally warn against automatic replay. UnknownTab does not leak
whether the tab exists in another workspace.

## Native panel and module structure

The panel goes through the workspace pane registration contract. Its target
is Browser(node, space, browser id), label is title then hostname then URL,
subtitle/tooltip URL, loading status Running, globe fallback. No independent
window manager or browser route is added. A hidden pane retains its target;
closing a tab closes its target, splitting/moving does not create a second
engine. Layout persistence stores identifiers/viewport only, not credentials,
frames, page text or authenticated URL query/fragment values.

view/browser owns URL preparation, toolbar state, navigation eligibility,
viewport presets, pending actions, element refs and errors. link/browser owns
all node calls, bounded frame decode, finite waits and reconnection. app's
browser area uses kit controls to draw a toolbar and the latest frame; it
translates pane-relative device pixels into typed CSS coordinates before
input. Selection and preedit use committed text; system IME candidate windows
remain GPUI's. An unimplemented input mode has a visible refusal; there is
no generic keylogger or page bridge into app state.

Use the reference's fourteen viewport choices, Cmd/Ctrl+L URL focus,
Cmd/Ctrl+R reload, browser-new-tab shortcut and pane focus semantics. App
shortcuts win over page input. Fixed frames center/clip; compact toolbar
moves secondary actions into a menu. Loading/error/closed states draw words
in English and Korean, prepared outside paint. Native text uses kit fonts.

Frames are bounded PNG images from the same target. A watch holds at most
one unsent latest frame; newer paint replaces it, discrete state/close events
must not be dropped. Start/stop frame production with watch visibility, keep
background targets alive for tools. Maximum frame rate 10/s, maximum decoded
image 8 MiB, viewport sides at most 8192, at most 32 targets and 64 queued
commands per engine; saturation is Busy, not an unbounded queue. Preview is
not advertised as smooth video or native page accessibility. CDP screencast
is experimental; capability-probe it and return Unsupported when missing,
not a static image that appears interactive.

Element annotate selects the engine's element and returns metadata plus a
bounded element screenshot to the existing composer attachment path.
Screenshot-element copies through the shared clipboard path. No new
attachment schema is invented; if the composer contract cannot represent it,
that control stays hidden pending the shared contract. DevTools is a visible
typed Unsupported for this first engine: never open an unauthenticated CDP
URL in the system browser. Public popups create another target in the same
workspace after policy checks, never switch workspace or launch externally.

The local-interface inventory is not implemented by std. Reuse an existing
platform facility only if it provides an authenticated complete snapshot;
otherwise this part stops for a dependency amendment with license and size,
rather than running a workspace-supplied shell command or silently allowing
self-addresses. This is an unresolved implementation prerequisite.

Node modules: vessel-browser/{actor,process,cdp,proxy,targets,actions,snapshot,
frames,uploads}; vessel-node/service/browser owns guard/session projection;
vessel-mcp/tools/browser owns only tool schema/decoding/formatting. Proto
owns all wire types; client owns typed calls. Add manifest edges only with
the code: browser -> proto, proc, relay (reuse reach), node -> browser;
MCP remains client + proto. Native crates retain their existing edges.
Update ADR 0001 and architecture tables when those edges are added.

## Why this choice and its trade-offs

It extends the previously reviewed one-profile policy and uses the same CDP
semantics for Linux and Windows. A child system webview adds a separate Linux
engine and windowing loop; using two engines gives misleading signed-in
state. The cost is remote frame latency, optional browser installation,
server-side credential custody, weaker native accessibility/IME/media and
a restricted service-origin capability for private previews. Those are explicit costs, not evidence
of completed parity. No browser panel is enabled before frames and input
work against the real qualified engine.

## Differences from Paseo and reasons

1. Node engine instead of desktop broker: one implementation/profile across
   native platforms, works headless, carries the server credential/reach costs
   documented above. No most-recent-desktop routing.
2. General is public-only; Preview permits one node-minted workspace service
   origin with session/lifetime checks and a separate ephemeral profile.
   No WebRTC/QUIC. This enables dev-server preview without broad server reach.
3. Dedicated operator-installed Chromium, no Electron-bundled runtime: no
   unreviewed runtime download or silently inherited user profile.
4. One General node-wide profile plus per-capability Preview profiles instead
   of one desktop partition: prevent a private dev server inheriting general
   sign-ins; warning distinguishes General sharing from Preview isolation.
5. GPUI frames/input instead of webview: no native page accessibility tree,
   extension UI, audio/video guarantee or native DevTools in this package.
   Typed refusal and hidden unfinished controls preserve honesty.
6. Added caps on viewport, images, targets, queues and frame rate: bound node
   and transport memory; tool defaults and reference caps otherwise retained.
7. Strong staged upload containment instead of lexical path containment:
   avoid symlink changes while an asynchronous browser reads a file.
8. Safe URL persistence, no query/fragment/userinfo in retained layout: avoid
   keeping authentication-bearing URLs in the app's preferences.
9. Human commands separate from default-off agent calls: panel use need not
   grant an agent control of signed-in tabs. Direct-human authorization is
   enforced at the server, not inferred from caller-supplied fields.
10. No production fake: absence is NoEngine and hidden tools, never a 1x1
    screenshot or evaluate echo. Test doubles compile only in tests.
11. Text-retention warning on typing tools: session event/raw_input is a
    separate credential copy; no claim that the browser removed that copy.
12. Protocol number assigned at merge and pending methods refused: no new
    call sent to a peer that cannot decode it.

## Verification and release conditions

Tests must fail with absent ownership checks, destination pinning, tool gate,
input routing, or generation invalidation. They cover all twenty-two tools,
old/new protocol directions, switch-off after start, cross-workspace ids,
agent attempts at human-only calls, stale frames/refs, empty history, close
while waiting, proxy failure, disconnect, screenshot-before-paint, symlink
upload and complete clear. Preview tests cover foreign workspace/session/port/scheme, missing
opening session, stale/truncated scan, PID/port reuse, redirect/subresource
escape, service stop with an active websocket/CONNECT, and cookie/storage
isolation in both directions. Boundaries are tested at each named cap.

Real-engine qualification uses only disposable fixtures, an isolated profile
and no real sign-in. Test actual proxy routing through redirects, rebinding,
subresources, websocket and service worker traffic. Bind IPv4 and IPv6 TCP
and UDP canary listeners and verify pages cannot reach them, including ICE
UDP and TCP. A real QUIC/WebTransport fixture is separate: an ICE UDP test
alone does not qualify QUIC. Inspect actual sockets/counters and successful
public fixture control traffic so a broken test cannot pass by doing nothing.
Run this under the actual non-root service identity, verify child files,
shutdown/relaunch/new-file permissions, and independently on Windows before
claiming Windows engine support. Unsupported/unqualified platforms refuse.

A fake CDP peer tests the typed protocol but proves no Chromium sandbox or
flag behavior. A green Rust gate is not the qualification gate. The native
capture matrix proves toolbar/layout/input only for the engine actually
used. Read every capture. Finish one whole native gate and the root gate for
node changes; submit engine/security commits and the completed panel to their
assigned reviewers. Until those implementation proofs exist, the engine remains unsupported and
the feature stays hidden.


## Unified Chrome WebRTC correction from real-runtime evidence

The initial contract's force-prefixed switch came from a content/headless
shell implementation. This ADR chooses unified Chrome, which has a distinct
command-line preference mapping. At Chrome tag 149.0.7827.55:

- chrome/common/chrome_switches.cc:703-705 defines
  kWebRtcIPHandlingPolicy as webrtc-ip-handling-policy.
- chrome/browser/prefs/chrome_command_line_pref_store.cc:55 maps that Chrome
  switch to prefs::kWebRTCIPHandlingPolicy.
- chrome/common/pref_names.h:1230-1232 names the preference
  webrtc.ip_handling_policy; chrome/browser/renderer_preferences_util.cc:
  158-159 reads that preference into renderer preferences.

Primary source revision:
<https://chromium.googlesource.com/chromium/src/+/refs/tags/149.0.7827.55/chrome/browser/prefs/chrome_command_line_pref_store.cc>.
The source was downloaded and read; a symbol's presence in an unrelated
shell implementation is not proof that the chosen executable applies it.

Actual diagnostic runtime: Chrome for Testing 149.0.7827.55, binary SHA-256
2d18db9d8608b052b6a552ee00ec1e830f93692e928b65ecc67d693bd33fe801,
278568152 binary bytes. Its root-owned installation and parents are mode
0755; the worker executes as UID/GID 1000 with a fresh mode-0700 profile,
minimal environment and sandbox enabled. Initial startup failed without a
usable sandbox; only the operator's dedicated sandbox-capable installation
was used afterward. No --no-sandbox experiment or system modification is
part of this package. Captured credits identify a Google Chrome for Testing
distribution; Chromium's root license alone does not describe that bundle.
Its credits/notices were captured for inspection, not approved for shipment.

The force-prefixed switch run loaded a synthetic public-origin fixture and
captured a real PNG, but the loopback UDP canary received six packets and
ICE gathering hit its deadline. That is a failed configuration, not a
qualified runtime. A separate Preferences-file experiment never loaded the
page, so its zero UDP count is inconclusive and is not counted as a pass.

With the Chrome switch, the public-origin fixture loaded, ICE gathering
completed, a real PNG was captured, and the IPv4 direct TCP and UDP canaries
both received zero packets after independent successful positive-control
connections/datagrams. The profile, proxy, sandbox and binary were otherwise
isolated in a fresh run. That is scoped diagnostic evidence for the switch
correction. The proxy was a synthetic fixture that served a test origin and
refused other destinations; it was not the product's resolve/classify/pin
proxy. It proves no production proxy, IPv6, QUIC/WebTransport or Windows
behavior, and does not allow a visible supported engine.

Qualification is bound to binary hash, platform, launcher/sandbox policy,
profile policy and exact flags. A changed executable at the same path,
changed preferences, policy override or launch options invalidates it.
After the product path exists, repeat its real network/credential/lifetime
suite with that exact path/configuration. Keep typed NoEngine until then.


## Transport allocation and client fetch boundary

[Decision Log]
- Purpose: a small compressed screenshot must not cause an unbounded raster allocation,
  and a favicon must not make the native application fetch a page-selected address.
- Existing constraints: the transport PNG cap is 8 MiB; CSS viewport side cap is 8192.
  Compressed size and CSS size do not bound physical RGBA memory.
- Alternatives: trust CDP image dimensions, or fetch a favicon URL in the app.
- Choice: validate PNG header geometry before retaining transport bytes and independently
  cap one RGBA surface at 32 MiB. The native codec must enforce its own allocation
  limit and fully validate the image before display. Header validation is not paint proof.
  The icon is a bounded PNG obtained in the browser policy context, or a globe;
  no remote favicon URL is passed to the frontend image loader.
- Reason and impact: the separate pixel budget supports the 2560x1440 preset while
  refusing oversized/full-page captures. This is a deliberate limit beyond Paseo's
  screenshot behavior. A refused screenshot is a typed error, never a fabricated image.
  Page text is capped at 80000 UTF-8 bytes with an explicit truncation marker;
  target inventories at 32 and each log ring at 200. Excess collection entries are
  refused before their payload is decoded rather than silently truncated.

Additional diagnostic evidence, still not product qualification: the isolated IPv6
HTTP/WS/STUN/TURN canaries also had successful independent TCP/UDP controls, completed
ICE and zero direct TCP/UDP observations with the corrected Chrome switch. A separate
local aioquic 1.8.0 H3 fixture observed a real WebTransport CONNECT and handshake in a
positive direct/QUIC-enabled control. The proxy plus disable-quic configuration observed
zero datagrams/handshakes/CONNECTs and a browser handshake failure. Two configuration
settings differ, so this tests that combined diagnostic configuration, not single-flag
causality. aioquic is scratch tooling only, not a product dependency. These runs do not
prove the future product proxy, profile isolation, Preview revocation or Windows behavior.

## Proposed C5 self-contained HTML document surface (review before implementation)

Status: proposed extension only. No document method, profile, supported renderer
or capability is enabled by this amendment. General and service Preview keep
all their existing rules, including first-engine Preview qualification.

[Decision Log]
- Purpose: W13 owns Target::File, its root identity, source buffer and mode.
  A file's HTML preview should reuse the verified rendering pipeline without
  acquiring General sign-ins, service Preview reach or an arbitrary file URL.
- Reference: Paseo app file-panels/html-preview.web.tsx:5-17,26-36 uses srcDoc,
  sandbox allow-scripts (no same-origin/popup/top-navigation tokens), and
  no-referrer. html-preview-csp.ts:15-27 permits inline/eval/blob scripts,
  inline styles, data/blob image/media and data fonts; default/connect/form/
  base/frame/object sources are denied. Lines48-54 supply the doctype and CSP
  before the verbatim body and remove one leading BOM. Sibling workspace
  resources are not granted. This is separate from Markdown relative resources.
- Caveat verified in that source: CSP does not prevent the frame navigating
  itself; navigate-to is not enforced. Native navigation interception is a
  mitigation, not a blanket browser guarantee. Vessel must not describe CSP
  alone as a network/navigation barrier on a remote node.
- Alternatives: render in General (would share credentials); point Preview at
  a made-up origin (would invent reach); file/data navigation (would bypass the
  root contract); a second unqualified renderer (would duplicate security work).
- Choice: a distinct ephemeral document context, below the same qualified
  engine owner and resource budgets, with no shared cookies/auth/storage/cache,
  no public/private network destination and no browser-tool addressability.
  Its public identity is a document handle, never a general BrowserId or CDP id.
- Consequence: canvas accessibility/IME/clipboard/export limitations are the
  first headless engine's stated limitations, not hidden iframe equivalence.
  When this context is unqualified, W13 retains Source and an explicit
  unsupported-preview result. No OS-open or General-profile fallback.

### Ownership and data flow

W13 supplies a self-contained valid UTF-8 snapshot of at most 1 MiB, bound to its
exact root/file/buffer revision and current resident operation ticket. Its
checked read/snapshot API is still pending; this ADR does not invent its types.
The app-side W12 ticket is a UI correlation/fencing token, never node authority.
The node rechecks the authenticated direct human, current node/workspace/file
read policy and snapshot/revision receipt under W13's contract. No cwd, guessed
path, caller URL or renderer title is used as ownership. No relative resource
reads are added, and no document invocation grants a wider file root.

Active HTML runs JavaScript on the node rather than in the user's local iframe.
The initial remote implementation requires direct human Operate as well as the
file owner's read permission; Agent/Delegated callers cannot enter it. Read-only
devices retain Source. This is a deliberate difference from client-local Paseo
HTML rendering, because node-side active content is server execution even when
its document is read-only. Agent browser_tools/session-start offering remains
irrelevant to this human file-viewing path. No new MCP browser tool is added.

A trusted blank root contains a sandboxed opaque srcdoc child, with only the
reference script allowance and no-referrer. The static doctype/CSP prologue is
installed before untrusted markup; no search for a user-provided head/doctype.
Untrusted markup is transferred as typed data, never interpolated into an
unescaped executable bootstrap. Snapshot chunks may be transferred within CDP's
control budget and joined only before installing the complete document. Source,
frames and page exceptions are redacted from Debug and never persisted in layout,
node diagnostics or a session tool log. The C5 owner keeps title/source mode.

### Required enforcement and refusal

The document profile uses a deny-all proxy, which refuses HTTP(S), websocket
and CONNECT before DNS/dialing. It has no General public allowance and no
service exception. The exact runtime/configuration must also prove no direct
TCP/UDP/WebRTC/QUIC/WebTransport bypass. File/debugging/external-handler schemes,
new windows, downloads and host clipboard/media/permission prompts are refused.
No sibling file/resource provider or invented URL authority is supplied.

Only the initial opaque document and its same-document fragment movement may
be presented. Every other attempted navigation invalidates the document owner
and closes/refuses the surface. Output is fenced by the expected frame/loader
identity before and after capture; a new document can never be published under
an old file revision. Such output fencing is not by itself proof that an unsafe
navigation was prevented. Qualification must independently prove that forbidden
schemes cannot read host files or invoke external handlers, and that navigation
cannot produce any outbound traffic. If headless Chromium cannot establish the
required guard for the actual runtime, return Unsupported for this surface;
never rely on an unenforced CSP directive or widen another profile's authority.

One document consumes one of the existing total target slots; initially at most
two document contexts may exist on a node. The 1 MiB source cap, encoded PNG 8 MiB,
RGBA 32 MiB and owned task/queue limits are independent. Hidden/closed/replaced
panes stop frame production; 5s paint/action waits and a wedged-script deadline
end the owned process group/job rather than yielding an old frame as success.
Close, changed buffer/revision, workspace/session move/archive, lost connection
lease or ticket replacement cancels the context, invalidates output and deletes
its private ephemeral profile. Cleanup uncertainty is an explicit failure;
no authority or worker budget is released merely because its caller timed out.

### Qualification owed

Test the exact launched binary/hash, sandbox, flags, profile and policy, as the
actual service UID/GID. Cases include malformed/oversized snapshots, BOM and
hostile HTML prologues, inline/eval/blob behavior, opaque storage/parent denial,
script/meta-refresh/form/anchor navigation, fragment movement, nested frames,
popups and file/debugging/external schemes. Use only synthetic file/network
canaries with independent positive controls, never real host secrets. Verify
no host-file read/publication, no private/public network path across IPv4/IPv6
TCP/UDP and QUIC/WebTransport, no credentials from General/Preview, process and
profile cleanup, late-reply/ticket rejection, saturation and platform refusal.
Header/codec fixtures, source review and a Rust gate alone qualify none of this.
The file owner must receive one checked renderer/API hash before adopting it.

## Proposed Linux child-pipe debugger transport (review before code)

Status: proposed transport refinement; the engine remains NoEngine. The already
staged loopback WebSocket adapter is not independently qualified by this change.

[Decision Log]
- Purpose: bind debugger traffic directly to the exact supervised child without
  a local TCP listener, endpoint discovery, bearer URL or socket-owner scan.
- Existing constraints: the current WebSocket model needs independently proven
  child/endpoint ownership. The existing process lifetime observer alone does
  not attest a listening socket, and legacy port.list is not such proof.
- Alternatives: wait for a complete socket-generation attester, inherit extra
  descriptors through new platform process APIs, or use Chrome's documented
  remote-debugging-pipe with the existing supervised piped child.
- Choice: first qualify Linux with anonymous child pipes. Retain the typed CDP
  command/event vocabulary and bounded multiplexing, not a second executor.
- Reason: the parent's private pipe handles already identify the child transport;
  no unrelated same-port service can replace it. This also removes the debugger
  listener from the page's network destination space. It does not replace the
  service-origin attestation needed by Preview.
- Tradeoffs: Linux needs one fixed noninteractive shell launcher to duplicate
  stdio onto the engine's expected descriptors and set a child-only umask. The
  shell path/content is part of qualification. Windows handle/job/ACL integration
  is separate and cannot inherit Linux proof. Missing proof remains NoEngine.

Chromium 149's DevTools pipe reader/writer use NUL-terminated JSON messages by
default. Its public DevToolsAgentHost declares fd3 for incoming control and fd4
for outgoing replies, and its startup helper checks these descriptors before
opening unrelated files. Primary sources, pinned to the allowed runtime's tag:

- https://chromium.googlesource.com/chromium/src/+/refs/tags/149.0.7827.55/content/public/browser/devtools_agent_host.h
- https://chromium.googlesource.com/chromium/src/+/refs/tags/149.0.7827.55/content/browser/devtools/devtools_pipe_handler.cc
- https://chromium.googlesource.com/chromium/src/+/refs/tags/149.0.7827.55/components/devtools/devtools_pipe/devtools_pipe.cc

The only shell source is a fixed implementation-owned literal:
`umask 077; exec 3<&0 4>&1; exec "$@"`. The shell's first positional argument is
an inert label; the executable and each fixed/typed flag are separate positional
arguments. No URL, page text, uploaded path, credential or caller-supplied shell
source is interpolated. vessel-proc still creates and owns the process group;
exec retains that leader identity. Only the operator-approved absolute Chrome
path and verified fixed shell are eligible, with the Chromium sandbox enabled.
Environment remains cleared, cwd/home/cache/config/temp remain private node-owned
locations, and the real user's profile and inherited proxies/keyrings stay absent.
No parent-process/global umask or host configuration is changed.

The launcher consumes the actual child's stdin/stdout handles and starts Chrome
with remote-debugging-pipe, never a debugging TCP port. Stderr is separately
bounded/drained without retaining page/runtime diagnostic text. Unexpected stdout
or malformed/oversized/unterminated control messages fail the engine; they never
trigger a TCP or unauthenticated-listener fallback. The existing CDP transport
message, command, pending, event and write/outer/shutdown bounds remain the single
owners for the pipe adapter. Read cancellation must preserve already-read bytes;
EOF settles pending calls with the same delivery uncertainty as socket loss.
Closing the pipes precedes supervised group cleanup and private-profile release.

Qualification must launch the real approved binary as the actual service user,
prove Browser.getVersion and a real page/PNG over these exact child pipes, verify
absence of debugger TCP listeners, and exercise malformed/fragmented/oversized
frames, blocked reader/writer, child death, cancellation and descendant cleanup.
The runtime, shell, flags, profile/environment policy and framing implementation
must all match the qualification fingerprint. Existing WebSocket diagnostic
results cannot qualify this changed launcher. IPv4/IPv6 TCP/UDP/ICE, QUIC and
WebTransport, credential isolation and complete Preview revocation remain owed.
There is no generic public pipe constructor, raw CDP tool or new MCP permission.
The whole supported feature remains hidden until the complete first-engine gate,
including Preview authority, passes; this proposal is not an exception.

## Proposed independently implemented DOM snapshot (review before implementation)

[Decision Log]
- Purpose: preserve the observed snapshot output, element names, state attributes,
  form fingerprints and actionability required by the behavior specification.
- Existing assets: Vessel's sealed CDP requests, bounded values and monotonic
  reference owner can be extended. Paseo's snapshot inputs, output shape and edge
  cases are behavioral references only; its source is not an implementation asset.
- Alternatives: translate Chromium native AX roles/names/states, or write our own
  bounded DOM traversal, labeling and fingerprint implementation from the behavior
  description, using standard browser DOM APIs and typed CDP requests.
- Choice: independently implement the DOM traversal and typed graph producer in
  Vessel, with Rust decoding, rendering and reference ownership. No reference
  JavaScript or other reference code/assets may be copied, vendored or adapted;
  license notices do not provide an exception to this project rule.
- Reason: a DOM-based implementation can represent the required form fingerprints
  and same-origin iframe behavior directly, without adding an AX translation.
  Behavior fixtures are independently authored from input/output requirements.
- Tradeoffs: independent labeling and edge-case coverage require dedicated tests;
  a private isolated realm and script result parser remain necessary. These tests
  do not replace actual engine, frame, actionability or isolation qualification.

The earlier reuse/vendor proposal was returned in V3 row206 and withdrawn under
its coordinator ruling. No script or vendor asset was copied into either tree.
This replacement proposes our own implementation only; it grants no engine or
runtime readiness and does not reopen the project reference-code boundary.

The script executes only in a node-created isolated world of the admitted
page/frame. No universal-access grant, host object, node filesystem, credential
or CDP handle is exposed. User browser_evaluate runs separately in the ordinary
page context; it cannot select this realm. Script source is implementation-owned.
Arguments/configuration are serialized typed JSON, never string-concatenated page
source or a caller-supplied function. The raw result is decoded immediately into
a bounded tagged graph/fingerprints; unknown/malformed shapes refuse and clear
host refs, never return an empty successful snapshot or evaluate echo.

Runtime script and Rust bounds have one named owner each. The existing1500 node,
500 reference and80000 rendered-text ceilings remain; final rendering counts
UTF-8 bytes, as already recorded. A new32-level graph depth ceiling protects the
nested JSON decoder and stack; deeper branches truncate visibly, a deliberate
difference from Paseo's node-count-only depth. Iteration stops at the same node
budget without eagerly copying an unbounded DOM child list. Bindings use exact
BigInt/decimal sequence values from the node's monotonic ref namespace rather
than reusing @e1 after a snapshot/navigation. The host accepts only the expected
current names/fingerprints from this target/realm and invalidates before a new
snapshot or failed traversal. Generation/world loss clears the entire binding.

Rust retains at most the existing current-ref cap and renders complete escaped
lines plus the reference truncation marker within the UTF-8 cap. Counts report
actual retained/rendered work, not a claim that the full DOM/frames fit. The
same-origin iframe limitations are preserved and stated; no new cross-origin
frame authority follows from the private world. Active DOM script/input stays
below existing Operate/session-start/tool policy and profile/destination checks.
The default-off tools warning still applies to signed-in DOM content and agent
input/results retained by the harness. Direct human input gains no extra logs.

Qualification requires real same/cross-origin iframe fixtures, hostile DOM/global
prototype changes, realm destruction/navigation, replaced elements/fingerprints,
deep/wide/long-Unicode documents, script timeout/cancellation and stale refs from
older snapshots. A static script/graph fixture is not a supported page or proof
of process/network/Preview isolation. No engine or tools become visible here.


## Proposed persisted browser-tools setting (review before implementation)

[Decision Log]
- Purpose: a person must explicitly enable agent browser tools; a missing,
  downgraded or unknown saved choice must never grant page/credential access.
- Existing asset: node.settings/node.set_settings and the settings.json owner
  already provide field-wise atomic persistence, private files and preservation
  of unreadable/later-build data. BrowserTools is already a default-Off enum.
- Alternatives: an independent browser config file/RPC, or extend that existing
  settings object and reuse its compatibility/admission rules.
- Choice: add browser_tools to NodeSettings (default/omitted Off) and optional
  browser_tools to SetNodeSettings; keep the one BROWSER_PROTOCOL=None constant.
  The existing settings/file owner is extended; no second writer or new method.
- Reason: one atomic owner prevents concurrent settings changes from losing this
  choice and preserves future values consistently with all other node settings.
- Impact: this is storage/admission plumbing only. It qualifies no engine,
  creates no session-start tool lease and offers no browser tool by itself.

A change naming this field requires its assigned browser protocol on every
local/forward/relay/deliver hop, including Off. While unassigned, the request is
refused before side effects rather than an older peer silently ignoring it.
Replies project the field to omitted Off for peers without that feature. Older
files read as Off. Unknown saved choices are effectively Off and remain subject
to settings.json's existing later-build preservation/backup rules. An unknown
requested choice is Invalid and performs no write; only explicit supported On
can enable the saved preference. Changes omitting the field preserve it.

Read uses existing settings Read authority. Any mutation naming browser_tools
additionally requires a direct human Owner or Device with Admin, including
turning Off. Agent and Delegated principals cannot use the generic settings
method to enable, disable or delegate this feature, even with other Admin scopes.
The existing guard and typed field protocol check still run before this check.
No warning text, origin, file path or credential is accepted as an authority token.

The native enable flow must show the already required warning about agent access
to signed-in content, General cookies shared across workspaces/devices, and agent
tool input/results retained in harness history. The user explicitly changes the
setting after that warning; an app read/restore cannot write On. Until the real
qualified backend and reviewed UI flow exist, the visible supported feature stays
NoEngine/hidden. Human control/watch remains separately authorized and is not
turned into an agent tool by this preference.

Engine qualification, shared ToolPolicy, Operate, local authenticated agent scope,
workspace/Preview ownership and a session-start offered-tool lease remain ANDed
with the live saved preference on every call. Turning On does not retroactively
add a tool lease to an existing provider process. Turning Off refuses subsequent
and not-yet-dispatched agent calls; already delivered effects remain uncertain,
not undone or silently replayed. It does not clear credentials or close separately
authorized human controls. Actual lease/revocation wiring is still owed.

Qualification includes absent/unknown/On restart persistence, concurrent unrelated
updates, failed-write atomicity, later-build backup and private service-account
files; direct/delegated/agent authority; pending/old-peer and nested-envelope
refusals before writes; and no session offer/engine claim from a saved On alone.
