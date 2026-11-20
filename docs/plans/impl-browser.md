# Workspace browser implementation

TRACKER 509–513 / catalogue F-G76–80. The reference behavior is documented in
[the browser survey and plan](browser.md). The runtime and wire choices are in
[ADR 0078](../adr/0078-browser-engine-and-native-panel.md)
and [ADR 0119](../adr/0119-general-browser-activation-independent-of-preview.md).

| Row | Packet | Evidence required |
| --- | --- | --- |
| 512 | Linux General runtime | Supervised approved sandboxed runtime under the service identity, private profile and debugger pipe, real target lifecycle, destination policy, bounded frame watch and input. |
| 509 | Native browser pane | Open through workspace actions, navigate and use device presets, retain hidden tabs, handle guest tab requests, clear General browser data and observe reload and failure. Preserve the desktop-only page boundary. |
| 510 | Element attachments | Selection and screenshot tied to the observed document. Attach and send through the shared composer, copy through the shared clipboard, cover cancellation and stale selection. |
| 511 | Browser tools and host setting | All 22 operations through the authenticated node, observed outputs and typed refusals, default-off and session-start gating, current setting recheck, and no discovery before qualification. |
| 513 | Service Preview | View service passes a verified service lease, observes revocation and uses an isolated profile. Depends on the scripts/services producer. |

Reuse the staged Chrome DevTools Protocol driver, `vessel-proc`, the shared
workspace pane registry and image path, and the single browser tool catalogue.
This keeps target and profile ownership in one place. The app consumes typed
state, bounded PNG frames and input mutations through the existing node link.
The node owns the public-destination policy, and Preview connects only through
its separate checked lease. A second embedded engine would duplicate profile
and target ownership, and the Rust standard library cannot paint HTML.

The first receiving packet carries A10 test-staged runtime components and
their required values. There is no registered browser method or supported
engine at this point. Keep wire registration pending for integration. Verify
source with remote format, lint and Rust tests, then run the immutable browser
test export against approved local fixtures under the service identity. Check
private permissions, target/process closure and relaunch. Native packets also
require full native and applicable page checks plus wide and compact captures.
Windows qualification remains pending.

Recovered historical results: run `wp-browser-ts-20261116T025409Z-c1283993`
reported TS freshness with 5 proto and 2 web checks. Run
`wp-browser-wait-tests-20261116T032059Z-218afa24` reported 77 browser unit tests
passed and 2 ignored, including both real-candidate fixtures. These results
predate this receiving tree.

The candidate fixture exercises trusted pointer and committed text, viewport
input generation, and bounded viewport/full-page capture. It also probes the
selected target's compositor using start, first-frame acknowledgement and stop.
A later watch needs a separate bounded frame path so frames cannot occupy the
essential state queue. The protocol is experimental and must be verified
against the approved runtime. [Chromium DevTools Page protocol](https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/pdl/domains/Page.pdl).

The local candidate network fixture serves its positive page in memory. Its
front proxy forwards only two held loopback destinations to the actual policy
proxy and checks the 403 responses, while both TCP listeners check for direct
connections. This isolates the refusal test from external DNS and websites.
The full policy and profile-reuse gates remain separate.

The first real candidate run on `9c6b81ae` reached trusted input, stale-frame
refusal, viewport capture and clean owned-process closure. The exact fixture
failed its full-page width assertion (375 CSS pixels observed vs 390 expected)
because a vertical scrollbar reduced the content width. The fixture now sets
its scrollbar width explicitly and retains both dimensions in the assertion.
The next run on `46e915a3` reached compositor start/ack/stop and both loopback
refusal checks. The content height was 1221 against the fixture's expected
1200 because the heading's default top margin collapsed outside the body.
The fixture defines the body's formatting context and height, and the final
assertion reports all three capture sizes together. No profile lock was released
for reuse in either run.

Each page owns one rate-limited latest-frame slot. Visible subscribers share
the checked PNG via reference counting. Navigation, resize, uncertain delivery
and close clear the previous context's pixels. The eventual node watch keeps
descriptor and terminal notices separate so a lagging frame consumer can skip
intermediate pixels while still detecting state changes.

The runtime inventory generates a browser ID for a successfully created
target, binds it to the admitted workspace, caps resident targets and retains
the target on an uncertain close. The live fixture checks foreign-workspace
refusal and removes the binding only after the actual target acknowledges close.

The immutable Linux fixture on `41682f25` passed in 6.75 s under UID/GID 1000.
Remote format, strict browser Clippy and the test export passed. The live test
observed a trusted click, committed text, stale-input refusal, viewport and
compositor PNGs of 390×844, and a 390×1200 full-page PNG. Both local canaries
received a positive control and zero browser connections after the actual
policy proxy refused their requests. The browser and private pipe closed, and
the retained profile still refused a second owner. The audit found no exact
profile process after closure and no group or other access across all 247
created entries. Product routing, profile reuse, per-renderer sandbox
observation and release qualification are pending.

The follow-up workspace and frame slice at `0e86fb5c8` passed remote format,
strict browser Clippy, 80 ordinary browser tests (2 ignored) and the separate
live candidate fixture. [Resume and exact gate receipts](impl-browser-resume.md).

### P1 actor ownership

The browser actor owns the verified child, its debugger receiver and the
workspace-bound inventory. Reuse the existing delivery latch to distinguish a
cancelled queued command from an uncertain command that has already started.
A bounded command queue limits pending work while the owner serializes target
mutations. Shutdown runs inside the actor after cancellation, so losing a
client waiter cannot drop the retained child or profile. A caller receives a
typed frame subscription, never the debugger connection. The actor stays
staged until the node admission path and release qualification are ready.

Discrete load and end notices have their own bounded channel. A visible frame
replaces the previous frame, while a lagged notice reader receives an explicit
gap from the broadcast receiver. An acknowledged close publishes a terminal
notice and drops the frame producer. Actor shutdown settles each watch before
closing the debugger and owned process.

The compositor sends a small paint signal so an active pane can update even
when loading state stays settled. The actor acknowledges that signal and
schedules a real viewport capture at the next permitted instant. It captures
both document stamps around the screenshot instead of using the small signal
image as the display frame. This keeps the CDP event queue bounded and lets
the node drop intermediate paints while retaining the last scheduled update.

The Linux relaunch fixture reopens a private profile only after the prior
child's acknowledged close and a fresh bounded observation of its original
process group and exact profile marker. A separate launch must return a
fixture-only persistent cookie through the loopback test proxy. This checks
reuse of a real Chromium profile under the service identity. The recovery
policy for a live node with an uncertain exit still needs integration.

The selected fixture exported from `1b27514d4` passed in 13.03 s as UID/GID
1000. It observed a real watched frame, foreign-workspace and stale-input
refusals, terminal target closure, an actual refused local attachment, a
held profile lock and the persisted cookie after relaunch. The audit found
306 owned private entries, no download file or partial download and no
remaining exact-profile process. The renderer sandbox sampler returned no
observation. This verifies the staged component against the approved runtime.

The extended fixture sends both direct and redirected requests through the
actual policy proxy to held IPv4 and IPv6 canary listeners. A controlled
loopback origin also runs embedded fetches and a real service worker. The
worker must retrieve a positive control before reporting the separate canary
responses. Distinct routes prevent an earlier navigation from satisfying a
redirect or background request. The fixture front door maps only observed
policy refusals to its expected marker. The live run is pending. These cases
leave real DNS rebinding, websocket and UDP/QUIC/WebTransport qualification
open.

### Download follow-up

| Owner | Scope | Reference and current difference |
| --- | --- | --- |
| D3 follow-up | Download/save from the browser pane | `desktop/src/main.ts:193,774` mounts the guest context menu, and `desktop/src/window/window-manager.ts:367–370` invokes Electron's download for **Save Image As…**. The browser partition uses Electron's normal download routine. The first Vessel General release denies downloads under the launch policy. A later packet needs a reviewed app-owned destination, cancel/error behavior and local fixture tests. |
