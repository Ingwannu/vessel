# ADR 0078: A Chromium engine and a native view of the same tabs

Status: proposed. No engine, new call, or visible entry is enabled by this document.

## Purpose and intent

Give a person a browser panel in the native app and give their agents the
same twenty-two browser tools as Paseo. A sign-in must belong to the page
both are driving; a second browser with a second cookie store is not parity.
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

Use one **node-owned Chromium in unified headless mode**. Both agents and the
person operate the same CDP target. The GPUI pane is a view of those targets,
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
a child process tree, a profile lock, CDP connection and bounded event decoder.
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
| Loopback, including the node's own listeners and CDP | Refused | Resolve-and-pin proxy; implicit bypass removed. |
| Node's local interface addresses, even public ones | Refused | Snapshot local interface addresses before launch and refresh on changes; fail closed if discovery fails. |
| Private, CGNAT, unique-local | Refused | Existing reach classification applied at every dial. |
| Link-local and metadata, including mapped IPv4 | Refused | Existing reach classification; never a workspace-preview exception. |
| Public HTTP(S) | Allowed, subject to the same proxy | Checked TCP address, normal certificate validation; no TLS interception. |
| Any direct UDP, QUIC, WebRTC, WebTransport | Refused | Runtime switches plus qualification tests below; HTTP proxy alone is not enforcement. |
| file, data top-level, javascript navigation, external handlers | Refused | Typed URL parsing and interception of page navigations/popups; no external launcher. about:blank is an internal empty target only. |

Use --force-webrtc-ip-handling-policy=disable_non_proxied_udp and --disable-quic.
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

**Service preview consequence:** only a publicly reachable service URL can
open in this first engine. A loopback/private service URL is Destination,
with an explanation in the panel and an explicit external-open action. Do
not invent a public URL, tunnel a private origin through CONNECT, or silently
open it externally. Full local-service preview parity requires a separately
reviewed scoped service-origin capability; no global private-network toggle
is added here. This is a material feature difference, not completed parity.

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

One node profile is shared by workspaces, sessions and authorized devices.
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
restricted private service preview. Those are explicit costs, not evidence
of completed parity. No browser panel is enabled before frames and input
work against the real qualified engine.

## Differences from Paseo and reasons

1. Node engine instead of desktop broker: one implementation/profile across
   native platforms, works headless, carries the server credential/reach costs
   documented above. No most-recent-desktop routing.
2. Public destinations only, and no WebRTC/QUIC: prevent server-local reach;
   local service preview is refused pending a scoped reviewed capability.
3. Dedicated operator-installed Chromium, no Electron-bundled runtime: no
   unreviewed runtime download or silently inherited user profile.
4. One node-wide profile instead of one desktop partition: required by the
   engine location; warning names all workspaces/devices sharing sign-ins.
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
upload and complete clear. Boundaries are tested at each named cap.

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
assigned reviewers. Until those proofs exist, status stays proposed and the
feature stays hidden.
