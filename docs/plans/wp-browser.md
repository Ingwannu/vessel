# Browser package: reference, integration and evidence

Status: ADR proposed; no product implementation or qualification claim.
Decision: **modify/reuse** the earlier browser value/policy work and the
existing MCP/client/proc/native layers; **new** engine/CDP/proxy modules are
necessary because none of those layers renders or drives a page. Do not
introduce wry, an Electron host, a second cookie jar or a general CDP tunnel.
The architecture/security contract is ADR 0078.

## Paseo reference, read before implementation

Paths below are relative to Paseo 0.15.4 packages/; no source/assets copied.

| Behavior | File and lines |
| --- | --- |
| Browser tab target, title/hostname fallback, URL subtitle, loading bucket | app/src/desktop/browser/panel.tsx:11-98 |
| BrowserRecord, viewport and URL normalization, transient reset on persistence | app/src/desktop/browser/store/state.ts:1-30, 97-125, 145-228 |
| Persistent shared partition and clear all storage/cache/auth | desktop/src/features/browser-profile.ts:1, 108-137 |
| Resident default 1280x800, re-register on attach | app/src/desktop/browser/resident-webviews.ts:9-12, 64-83 |
| Fourteen device choices | app/src/desktop/browser/pane/index.electron.tsx:131-146 |
| URL protocol guard, toolbar, annotate and screenshot | app/src/desktop/browser/pane/index.electron.tsx:159, 198-218, 1063-1244, 1436-1519 |
| Main process validates attached guest's owner and partition | desktop/src/features/browser-webviews/index.ts:66-89 |
| Workspace-bound background creation and default URL | app/src/desktop/browser/automation/handler.ts:297-318, 341-391 |
| Browser tool registration at MCP construction | server/src/server/agent/tools/paseo-tools.ts:1358-1365 |
| Default-off daemon switch stored as daemon.browserTools.enabled | server/src/server/persisted-config.ts:263-267; daemon-config-store.ts:187,215 |
| Every tool and argument | server/src/server/browser-tools/tools.ts:42-59, 66-712; map in ADR 0078 |
| Scheme-less tool URL uses http (different from panel's https) | server/src/server/browser-tools/tools.ts:721-751 |
| Broker deadline and host/affinity behavior | server/src/server/browser-tools/broker.ts:46-58, 80-110, 117-165 |
| Wait/screenshot/evaluate caps | desktop/src/features/browser-automation/service.ts:68-77 |
| Navigation scheme/empty-history refusals | desktop/src/features/browser-automation/service.ts:1160-1215 |
| Upload containment is lexical resolve/relative check | desktop/src/features/browser-automation/service.ts:1423-1473 |
| Tool engine enforces workspace ownership | desktop/src/features/browser-automation/service.ts:1640-1650 |

The survey's area-4 sections 7/8 and MCP catalogue section were read. The
feature catalogue currently contains headings only and CONNECTIONS has no
browser row. Use the source inventory above, not invented catalogue IDs.
The full differences and the one-by-one twenty-two tool mapping live in the
ADR to avoid a competing list here.

## Reuse analysis

Existing native app has no browser area. Existing root node has no registered
browser RPC. An earlier browser branch has BrowserCall, BrowserCommand,
BrowserSpace, typed limits/errors, and an absent product engine; the double
is not complete there and no real engine exists. Adopt only reviewed,
consumer-ready parts; preserve the other worktree and its drafts.
Reuse vessel-proc cleared environment/child tree, vessel-relay reach policy,
vessel-client authorization/older-peer checks and vessel-mcp tool machinery.
Use the pane owner's browser target/registration contract, existing shared
kit controls and the composer attachment boundary. No file grows past its
size record; registration lines only in shell and link root.

External alternatives inspected: wry vendor docs for child window/GTK and
Wayland restrictions; Chromium unified-headless and license docs; CDP Page
catalogue lists frame/input primitives. That establishes API candidates, not
working integration or sandbox proof. No runtime downloaded or globally
installed. Vendor links and the license/size decision are in ADR 0078.

## Implementation order and boundaries

1. Submit ADR 0078 for security/design review before product code.
2. Obtain app base and node integrator's method-table/accessor contract.
   Missing calls stop node wiring; never invent an existing node method.
3. Skeleton: typed view state/commands/events and hidden area; tests for the
   twenty-two commands and honest NoEngine path. Any fake is cfg(test) only.
4. Qualified engine with policy proxy, actor and typed CDP, then MCP calls,
   then real native frames and input through the pane registration contract.
5. Inventory/history/viewport/clear, errors, popups and element attachments;
   local-service Preview uses node-minted workspace/session/service-origin
   capabilities and isolated ephemeral profiles, including stop revocation.
6. Narrow checks per commit. Capture the final layout under an isolated fake
   fleet and qualified disposable browser, both languages/themes and desktop/
   compact sizes. One whole native gate and one root gate for node changes.
   Security review per security commit; panel review for the whole package.

Node additions proposed by ADR 0078, not yet available: browser.call,
browser.control, browser.watch, browser.clear_profile and browser_tools.
All use one unassigned feature constant. No numeric protocol ownership.
The panel remains hidden until these calls and a working engine exist.

## Evidence ledger

- Read ADR 0069 and the initial return plus corrected document pass. Earlier
  document acceptance did not certify Chromium or a panel.
- App base inspected: no browser code; root crates equal the supplied fleet
  binary base. Runtime/service account checks: not run; no live host touched.
- Engine qualification, Rust tests, native captures and Windows execution:
  not run. These documents are not a feature completion report.
- Document fast checks: root fmt, native fmt, native size, native boundaries,
  native tokens all passed. Native size checked 554 files (15 existing size
  records); boundaries checked four crates; token records gained no violation.
  Root compile/size/boundary gates are not claimed: no root code changed.

## Private Preview decision and port evidence

The first engine includes dev-server Preview. General remains public-only;
only a current owned service socket can mint the isolated capability in
ADR 0078. Reuse the existing scanner rather than a second port collector.
The scanner's cached list alone is insufficient for security admission.

Node source read on the node base:
- crates/vessel-proto/src/ports.rs:102-157: ListeningPort carries address,
  port, PID and session/terminal owner; PortScan distinguishes pending/error.
- crates/vessel-node/src/terminals/ports.rs:29-37, 81-115, 132-193: bounded
  scanner, three-second observation period, cached watch/revision. No
  process-generation attestation or HTTP scheme is present in that API.
- crates/vessel-node/src/service/scripts.rs:26-49, 247-280: script place
  derives project/worktree from the registry; port filtering uses the
  authenticated agent's session and created_by facts.
- crates/vessel-proc/src/listening.rs:37-53, 116-128: Listener root/PID,
  bounded async scan; no stable process/socket-generation token.

Add the missing generation/freshness/revocation boundary as part of this
package. Do not invent a guarantee that script status or an old PID gives.
Protocol/schema assignment stays with integration. Node work starts on the
announced node head in its own worktree.

## Identity skeleton

The first node types stage BrowserId, ElementRef and the single twenty-two
name catalogue without registering a method, changing settings or advertising
any tool. Reuse the earlier grammar intent, with an exact canonical UUIDv4
check: uuid's general parser also accepts braced/simple/URN forms that the
reference browser-id regex rejects. Tests cover those forms and legacy
minimum timestamp length, case normalization and non-ASCII element digits.
There is no engine, page fixture or visible native entry in this commit.

## Runtime correction shared with the native contract

The contract passed review before implementation. Unified Chrome then leaked
UDP with the shell's force-prefixed WebRTC flag; the Chrome-specific switch
is required instead. ADR 0078 records the exact source mapping, failed and
inconclusive experiments, successful scoped IPv4 canaries, runtime hash and
remaining qualification obligations. Product browser readiness remains
absent; no API or agent tool has been registered by this branch.

## Typed command skeleton after the shared method-table base

The pending browser protocol constant remains None. BrowserCommand and its
MCP name now come from one catalogue for all twenty-two variants. Command
arguments validate viewport/wait/log counts and nonempty upload intent;
text/function/URL/path Debug output is redacted. Tool URL preparation uses
HTTP; address-bar host preparation uses HTTPS except local hosts. Numeric
IPv4 aliases are rejected before URL canonicalization can hide their form.
Page destination and workspace/file authorization still belong to the engine.
A wait's URL argument is a nonempty substring, not a navigable URL, matching
Paseo service.ts:1005. No setting, successful engine, visible entry or RPC is
claimed by the command/value skeleton.

The shared Quality B node base was merged without conflicts; its method
and version policy remain authoritative. The app package base was likewise
merged without conflicts; only the browser module registration was retained
in the view root. No shared size limit was raised.

## Failure and wait semantics

Typed failures distinguish an undelivered timeout from an uncertain page
mutation, with fixed safe messages and existing RPC error classes. An
unknown future reason remains Unsupported, never Ready. Wait budget preserves
omitted versus explicit timeout: the host default is the same in both, but
Paseo keeps the ordinary broker deadline when omitted and uses explicit
wait plus response allowance when supplied. The command catalogue owns that
deadline projection; a second tool-specific deadline list is unnecessary.

## Resume point

Done: ADR and unified-Chrome correction reviewed; identity and twenty-two
command input skeletons; pure native toolbar/device models and translations.
Half done: narrow checks are still in flight; no browser RPC, backend engine,
policy proxy, service capability, MCP registration or panel is exposed.
Next: merge the reviewed shared workspace identities; implement the pending
call/response and typed owner/service interfaces, then the real engine path.
Keep registry ownership behind the agreed typed interface until its package
is reviewed. Source-only/runtime diagnostics are not whole-package evidence.


## Bounded response and authority-selector skeleton

Reuse shared WorkspaceId from reviewed identity-only e45a5d89; no registry/store
subset is imported. Agent BrowserCall has no workspace selector. Direct-human
BrowserControl and WatchBrowser name the logical workspace and optional opening
session; those are selectors, not authority. PreviewServiceHint is explicitly
untrusted until runtime-owned service observation proves generation and scheme.
No engine, successful RPC, or native renderer is advertised by these types.
Responses distinguish snapshots, evaluation, log rings, descriptors and PNGs.
ADR 0078 records the separate PNG/pixel cap and no-client-favicon-fetch decision.
The default-off BrowserTools value is staged but is not yet a saved setting.


## Direct human input skeleton

The fixed control vocabulary carries CSS-relative pointer/wheel events, key
press/release, text, and release-all on capture/focus loss. Coordinates have
a finite global bound and are rechecked against the actual current viewport
at dispatch. Input names/text are redacted in Debug. Modifiers occupy fixed
state; MCP array names fold once at its decode edge. Browser input/function
text has a new 128 KiB per-field ceiling, beyond Paseo's observed tool schema,
to keep bounded queue slots from retaining transport-sized strings. Oversized
input refuses explicitly rather than truncating a password/function. These
types do not persist human input or grant any engine/profile authority.


## Engine primitives

New vessel-browser owns browser execution/network state below node admission.
Start with per-target refs (monotonic ids across snapshot replacement so stale
@e strings cannot alias another element) and a public-only dial primitive.
DNS is resolved once per outbound connection; any forbidden/mixed answer refuses
the whole route. A current safe OS interface snapshot excludes this host's own
addresses before and after a pinned-address dial. Missing observation refuses,
including unimplemented Windows observation. No real browser/proxy is claimed
by those primitives. The full policy proxy and qualified engine remain pending.


## V3 identity correction

Review found uuid's version nibble alone does not enforce the RFC variant.
BrowserId now requires Variant::RFC4122 together with v4 and canonical spelling,
matching Paseo rpc-schemas.ts16-18 fourth-group [89ab]. Fixtures reject NCS,
Microsoft and future variant spellings through both direct parse and serde.
The earlier successful tests did not cover this predicate and do not override
the returned source finding.


## General HTTP proxy implementation (staged)

Reuse workspace hyper/axum HTTP types and streaming bodies, rather than invent
a raw parser or buffering page bodies. HTTP absolute-form and CONNECT authority
are decoded separately. Host is rewritten to the validated URL; connection-named
hop headers and proxy credentials are removed. Only a requested WebSocket can
upgrade. Each upstream is a pinned public dial; the entire DNS answer and current
interfaces are checked again at connection boundaries. No direct fallback exists.
A profile owns the listener, client sockets, upstream drivers and upgraded tunnels
under one cancellation token; 128 connections retain permits through upgrades,
64 headers/32KiB parser storage and 5s header wait bound admission. Backend header
wait is 15s. These are deliberate resource limits beyond the reference behavior.
Private-target refusal and profile-close fixtures use only own ephemeral loopback
listeners with independent positive controls. Passing them will not qualify a
real Chromium configuration, Preview service authority, or Windows support.


## Typed CDP transport (staged, no qualified engine)

Reuse tokio-tungstenite/futures/serde/url already pinned in the workspace. A local
launcher endpoint parser is not child-ownership proof: the controller must verify
the just-launched child separately, never accept a caller endpoint. Only sealed
request structs choose CDP methods and response types. Per connection: 64 queued
commands (each encoded control message <=256KiB), 16 pending replies, 32 essential
events (each <=256KiB), 16MiB transport frames/messages; overload closes/refuses
rather than dropping navigation/dialog invalidations. Delivery is recorded before
a potentially partial socket send; timeout or lost response after delivery is
uncertain. Private target/session ids, payloads and page exceptions redact Debug;
RawJson's default Debug is deliberately wrapped. Unknown unrelated events are
ignored without interpreting their opaque payload. Owned cancellation/shutdown
closes only this debugger connection and settles pending calls.
Scripted own-loopback peers cover out-of-order correlation, lost mutation reply
and essential event overflow. Those are transport fixtures, not product browser
frames or engine qualification. Primary CDP method references: devtools-protocol
Target and Page (https://chromedevtools.github.io/devtools-protocol/tot/Target/ ,
https://chromedevtools.github.io/devtools-protocol/tot/Page/). Actual launched149
protocol/runtime still must be qualified, with Preview support and all22 actions.


## V3 preparation ownership / A10 correction

The first proxy checkpoint was returned: dropping Tokio lookup_host or an
untracked spawn_blocking wait did not stop its actual OS job, and the executable
new-library API had no product consumer. All browser execution modules are now
behind cfg(test), with crate-private visibility. Planned removal is the reviewed
`browser: wire the qualified node browser actor` commit, together with its real
node caller/runtime/service policy and whole gate; no staging feature or dead-code
allowance substitutes for that wiring. This branch cannot be adopted as product
execution before that review. Proto value types remain ordinary wire models.

A single process-wide four-worker gate is shared across profile preparation
scopes. The permit and profile TaskTracker token move inside the actual queued/
started blocking closure and survive waiter cancellation/timeouts. DNS now uses
std ToSocketAddrs inside that owner rather than Tokio's hidden resolver job;
getifaddrs runs under the same owner. Overload refuses without an unbounded wait
queue. Cancelled/late results cannot start or publish a socket; each dial rechecks
profile cancellation at its execution boundary. Profile close cancels sockets,
then waits for actual preparation completion. Its caller deadline can report an
uncertain shutdown but does not release worker admission early. Barrier fixtures
cover held work/caller abort, caller timeout, profile close, saturation and eventual
release. Those common-owner fixtures do not pretend to induce a hung OS resolver.


## Pending node plumbing on the announced table head

Merged ba45a6e3; shared changes win, browser ADR/map additions retained in the
two documentation conflicts. Four rows are appended only to rpc/table.rs with
the same BROWSER_PROTOCOL=None: call/control Operate unary, watch Operate stream,
clear_profile Admin unary. No number selected and no browser library consumer
added. Generated guard/envelope/unary policy follows the common table. Explicit
agent, target, forwardability, older-reply, version and sample arms are updated.
Handlers are concretely wired to typed Disabled/NoEngine refusals: browser.call
requires an own local Agent principal; human methods require direct Owner/Device
and their own scopes, rejecting Delegated even when its origin was human. No
workspace selector, session id or service hint can start an engine or grant reach.
Client unary methods preserve those failures. Watch is a pending stream row; its
new frame/stream consumer path and functional engine producer are still owed.
No saved setting/session-start offering, active engine or native entry is enabled
by this plumbing. Tests cover None/scope/shape/no-forward projection and direct
human/agent/delegated admission before any future actor can inspect content.


## C5 HTML preview proposal

Read Paseo html-preview.web.tsx and html-preview-csp.ts directly after W13's
reference handoff. No sibling resource authority exists in that reference; CSP
does not block self-navigation. ADR0078 now proposes a separate opaque ephemeral
document surface with deny-all networking, owner/revision/ticket fencing and
actual-runtime navigation/file/egress qualification. It grants no current API
or supported surface. W13 retains Source until a checked hash exists. Initial
node-side active HTML requires direct human Operate plus the file-read policy;
this deliberate difference avoids silently granting server script execution to
Read-only devices. No General/Preview fallback or extra MCP tool.


## Flattened debugger response binding

CDP pending requests now retain the expected flattened session as well as the
monotonic command id. A mismatched response closes the transport and refuses
the result; it cannot be used as the selected page's reply. A scripted peer
regression sends a correct command id from a different target. No actual
browser/runtime qualification follows from that transport-only fixture.


## Staged actual-target controller

The page controller consumes only sealed CDP requests and engine responses; it
cannot construct an engine or grant workspace/network authority. Creation sets
up observed Page/Runtime/Network domains and a bounded surface before returning
a target. Partial setup attempts target close and remains an error; the future
profile owner must still retain process cleanup responsibility. Commands are
serialized through mutable page ownership; any failed CDP exchange quarantines
the target for close instead of allowing automatic mutation replay. History is
read from Chromium and checked before selecting a real entry id. Unsupported
file/debug/internal history entries refuse, a deliberate HTTP(S)-only safety
difference. Raster admission checks the existing 32MiB RGBA cap before metrics
change, in addition to the wire side limits. No script echo or test PNG is
published as a page. Controller fixtures are protocol tests, not qualification.
