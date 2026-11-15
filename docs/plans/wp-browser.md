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


## MCP argument edge (staged, no registration)

The browser adapter consumes shared BrowserTool values, not a second name table.
Paseo tools.ts fields are converted explicitly: browserId/ref, doubleClick,
modifier names, timeoutMs, fullPage, filePaths, sourceRef/targetRef, maxEntries,
function and CSS deltas/size. Defaults remain left/single/no modifiers, viewport
screenshot, 50 logs and omitted wait's distinct 15s broker budget. Tool URLs trim
outer whitespace and use the reference HTTP scheme default. Null optional values
refuse as in Zod; extra keys are ignored and never become node/workspace/session
selectors. All upload paths remain untrusted intent. Invalid inputs expose only
fixed field/cause diagnostics, never the submitted text. The outer cfg(test)
boundary stays until actual W18 shared policy/engine wiring; no schemas or tool
offering/dispatch are registered by the decoder.

Schema construction uses that same closed BrowserTool catalogue and its exact
MCP names. It describes both wait branches with an exclusive oneOf, numeric
limits from proto constants and optional defaults. JSON Schema maxLength counts
characters; decoding still enforces UTF-8 byte caps. Building these values does
not offer tools or bypass W18's shared ToolId policy/session-start admission.


## First combined execution evidence and correction

Exact f79633464 remote run wp-browser-execution-tests-20261115T103851Z-f7963346
executed browser23/0, MCP14/0, proto316/1. The one failure was the explicit Admin
method inventory missing the newly registered browser.clear_profile. That entry
is now added without changing its Admin scope or weakening the equality check.
Exact strict run exposed the shared client fake's missing four Handler methods,
empty sample-arm duplication, uniform-async refusal handlers and unused ACK
values. Both explicit client/RPC fake inventories now refuse those four methods;
they cannot synthesize a browser. Compiler warnings from the executed suite are
also corrected (ACK/result ownership and schema insert results). Changed-source
strict/tests are owed; the partial green suites are not an engine/whole gate.


## Staged pointer and text input

Sealed Input.dispatchMouseEvent requests preserve finite CSS positions, exact
button/modifier masks, held-button movement and wheel deltas. Held state belongs
to the page; ReleaseAll releases only buttons that target admitted. The page
rechecks coordinates against its last acknowledged viewport before sending, and
retains a changed viewport only after a real metrics ACK. Input.insertText is
direct CDP text insertion, never an eval string. Lost/failed delivery quarantines
the page as before. The keyboard adapter is not yet implemented and returns
Unsupported, without a text/eval fallback. These paths remain outer cfg(test),
not a supported/advertised input surface, and their new fixtures are unexecuted.

## Input focus boundary found in the reference

Paseo service.ts TabContents.insertText comment explicitly distinguishes its
isolated tab text commit from CDP Input.insertText, which can follow the window's
current focus. The test-staged Page input adapter therefore is NOT sufficient
input isolation proof. Before any product wiring, an actual profile-wide input
coordinator must serialize activation plus the entire input transaction, release
held pointer/keys before changing targets, bind the lease to target/session and
cancel the whole profile on uncertain/cancelled delivery before allowing another
target to acquire input. CDP session correlation alone cannot grant that proof.
A two-real-tab focus/canary regression (including background targets, simultaneous
callers and cancellation after activation/before ACK) is required. No private
constructor/test fixture or single-tab protocol ACK may qualify this boundary.
Node/renderer enabling remains blocked on complete real input routing proof.
The current page module remains cfg(test) and NoEngine. The reference comment is
an embedder-specific warning, not measured evidence that unified headless Chrome
misroutes every session-scoped insertion.

On 2026-11-15 12:43 UTC, a fresh sandboxed Chrome149 private-pipe diagnostic
created two targets in the same actual browser window. Both documents focused
their own synthetic input, then B was activated. Input.insertText sent to A's
flattened session changed A only; sixteen alternating session insertions also
stayed separate. The child exited0 after Browser.close; service UID/GID1000,
profile0700 and all inspected regular entries private. The retained script and
result are /tmp/work/wp-browser/pipe-multitab-input/{probe.py,result.json}.
This tests only that specific two-target text path. It does not prove pointer,
key/IME, dialogs, concurrent actor cancellation, product proxy, Preview or Windows
isolation and does not remove the required profile-wide transaction fence.


## Private debugger pipe staging after V3 contract PASS

V3 row188 accepted only the child-pipe contract. The staged transport now has
two statically dispatched I/O adapters under one typed driver. Anonymous pipes
use NUL framing, the existing message limit, bounded read quanta and owned
partial-read storage. Cancelled reads preserve already-read fragments; EOF in a
partial frame is a protocol failure. Write/outer/pending/event/shutdown limits
remain the existing CDP constants. Bad UTF-8 retains a content-free Utf8Error;
raw bytes never become diagnostic text. No fallback switches transports.
Duplex peer tests cover fragmentation/read cancellation, multiple frames, NUL
writes, malformed UTF-8, oversize/unterminated input and the actual typed driver's
version/reply/closure path. They are not a real runtime, OS child/descriptor or
sandbox/profile qualification. No public pipe constructor or engine is enabled.

## Child-pipe runtime diagnostic and profile initializer corrections

The fixed shell descriptor control passed under uid/gid1000, including a new
0600 file from the child-only umask. Two initial actual-Chrome launches ended
SIGABRT before a version response; those failures are retained, not qualification.
Using a separate short private TMPDIR then yielded the actual149 version,
matching kernel fd3/fd4 pipe pairs, fixture title and a390x844 real PNG. Its
strict profile audit still failed on cache/fontconfig mode0755. Precreating that
known cache directory0700 in each fresh profile corrected the initializer rather
than repairing a running profile with a blanket chmod. Long TMPDIR startup is
consistent with Chromium's Unix SingletonSocket path limit, but the earlier
stderr was discarded, so that historical cause remains an inference.

Diagnostic4 now passes its own scoped checks: actual uid/gid1000, approved binary
SHA2562d18db9d..., fixed dash launcher SHA25686d31f6f..., sandbox retained, private
pipe kernel correspondence, actual title/PNG7855bytes at390x844, zero root-browser
TCP listeners, normal Browser.close exit0 and zero permissive profile entries.
The PNG was opened and inspected. Reports/scripts/images are under
/tmp/work/wp-browser/pipe-runtime-diagnostic{,-2,-3,-4}; fd control receipt is
/tmp/work/wp-browser/pipe-descriptor-control.json. No Chrome stderr text was kept
by these diagnostics. This is not the Rust pipe implementation or product proxy,
complete process-tree listener proof, cross-tab input isolation, UDP/ICE/QUIC,
Preview/credential revocation, Windows ACL/job or distribution qualification.
No operator/system permission or sandbox configuration changed. The engine stays
NoEngine. The future initializer must own a short private runtime temp directory
and cache/fontconfig0700 on every creation, and retain the profile until actual
child/descendant cleanup is observed, including failed/aborted startup.


## Staged Linux profile filesystem owner

The initializer creates/validates a node-supplied absolute profile and short
private runtime temp directory, plus home/config/cache and precreated fontconfig
cache. It sets creation modes, refuses existing unsafe paths instead of chmod
repair, rejects symlinks and writable non-sticky ancestors, and holds a regular
private no-follow/no-hardlink lock file exclusively. A root-owned sticky temp
ancestor is permitted; arbitrary writable group parents are not. This stricter
ancestor policy can refuse custom data paths and is intentional; a diagnostic
profile under shared /tmp/work does not independently qualify production storage.
The temp-root byte ceiling has one named owner and leaves a conservative margin
for Chrome149's Unix SingletonSocket suffix. Linux unit fixtures use their actual
service UID, small private /tmp directories and real file locks/permissions; no
browser is launched. The sync initializer must run under Preparation's finite
actual-worker ownership. Its lock cannot be released/reused until the future
supervisor has observed child/descendant cleanup. Windows ACL admission remains
unimplemented/NoEngine; this module is test-staged on Linux only.


## Staged candidate launch data

Reuse vessel-proc CommandSpec instead of adding another spawn/group/job path.
The plan clears inherited environment and supplies only private HOME/config/
cache/temp, explicit C.UTF-8/UTC/PATH, fixed pipe/proxy/sandbox-preserving flags
and about:blank. The fixed shell source receives positional argv only. Its
profile is retained by the plan and passed with the command to the future
supervisor; no profile/process lifetime is reported complete by this data type.
The diagnostic candidate's actual browser and shell SHA-256 digests are pinned
and checked with streamed reads, root ownership and nonwritable ancestry. A
post-start check accepts the actual owned Child, not a caller/stored PID.
These two executable hashes are NOT the complete immutable runtime asset/
node-binary/launcher-policy qualification fingerprint. Candidate inspection or
unit fixtures never grant a QualifiedRuntime or publish a ready engine.
Actual process supervision, private pipe constructor and lifecycle evidence
remain owed. Windows admission remains NoEngine until its handle/job/ACL proof.

Candidate hashing also refuses nonregular/oversized files before reading and
counts streamed bytes against one named executable-size ceiling, including a
file growing during inspection. Actual service execute permission is required.
A harmless owned shell-child fixture checks that an unrelated executable cannot
inherit the candidate's post-start identity; it is never a browser fixture.


## Non-durable watch values (transport still owed)

Browser watch data is separate from EventRecord/session history. A nonzero
watch sequence orders updates; document/viewport BrowserRevision remains the
displayed input context and is not incremented merely for another same-context
capture. A frame carries matching BrowserId/workspace/CSS viewport/revision/PNG.
Initial descriptor and every update must keep the same immutable target/profile
scope. Ends are Closed/Revoked/Lagged/typedFailure, with no fake durable resume
cursor. Slow watches must reopen. These are value types only: no new ServerFrame,
RPC producer/client route or successful frame fixture is registered. Pending
browser.watch remains NoEngine until the real scoped stream/engine wiring.


## MCP result projection (staged)

The output edge verifies the returned operation kind and target before emitting
content. An inconsistent reply produces a fixed uncertain-effect refusal, never
a mutation-retry instruction or another tab's image. Screenshot data appears
only once as an MCP image block; text and structured metadata include dimensions
and identity without base64. Tab favicons are omitted from agent metadata as UI
content, preventing large image duplication in tab inventories. Non-image results
retain typed result metadata. The structural screenshot regression uses a real
PNG of our own synthetic private-pipe canary, with provenance and scope notice;
it does not advertise a browser implementation/qualified engine or visible frame.
No dispatcher, tool list or policy lease is enabled by this test-staged encoder.

Latest strict68 stopped on nine style/test diagnostics (docs, pointer enum prefix,
helper value ownership, moved DTO declaration and consumed read counts). They
are corrected without changing domain behavior; exact00ae functional suite
completed493/0/0 before the later launcher/watch/output additions. Current
changed-source strict/functional evidence remains owed.


## Updated common bases and launch permission predicate

Node merged announced ede20014 without conflicts after the active strict suite
was pinned; app merged announced dd13ce37 (base6 lock fix) independently. Neither
was a whole-package freeze. New changed-union narrow runs are queued; previous
strict/tests are not promoted to these unions. Candidate executable admission
also now checks the opened regular file's other-execute bit in addition to root
ownership, nonwritable ancestors and fixed digest. A non-root service may not
accept a non-executable candidate just because its contents hash correctly.
No chmod/chown or host policy change is performed on refusal.

## Independently implemented snapshot, staged after row206 contract PASS

The coordinator and V3 rejected reuse/adaptation of Paseo snapshot code/assets.
The corrected contract755b500ef (app66805baca) passed contract review only. The
new JS traversal is written here from behavior requirements; no reference script
or notices are vendored. Reuse is limited to Vessel's existing sealed CDP edge,
current-ref owner, redacted bounded content and geometry/lifetime boundaries.

The new tagged flat graph decoder rejects excess nodes before their payload,
unknown roles/states, depth/count inconsistency, aggregate text+fingerprint bytes,
nonconsecutive/wrong-generation refs and invalid output. It clears partial refs on
failure. The existing500 ref owner reserves a full monotonic range before sending
any traversal; failed or cancelled traversals consume their range. Script config
serializes the host's one node/ref/depth/text budget and exact decimal ref start.
JS BigInt avoids Number aliases above2^53. Output escapes content into complete
YAML lines and reserves the visible reference truncation marker within80k UTF-8.
Fingerprints and element handles stay in the private realm/current host map.

The page controller creates an isolated world with grantUniveralAccess=false
(the upstream spelling), then invokes only our fixed function with typed JSON
arguments. Frame and loader identities must match a post-traversal frame query
before refs publish. User evaluate cannot select this world. The official CDP
command definitions inspected were Page.createIsolatedWorld/getFrameTree and
Runtime.callFunctionOn in https://github.com/ChromeDevTools/devtools-protocol/tree/master/json .
These current schema facts require actual149 runtime verification, not assumed
API compatibility. Malformed/exceptional output never returns an empty success.

Current algorithm limits/differences needing behavior and real-engine fixtures:
label text scans share the1500 considered-work budget (including repeated nodes),
DOM wrapper depth also stops at32, aggregate graph strings reuse the80k byte cap,
and a fingerprint that cannot fit gets no ref. A visible truncation fact accompanies
these limits. Semantic leaf roles emit their name without duplicate descendant
text. The role/name coverage, same-origin frame traversal, hostile-page isolation,
replacement fingerprints and actionability are not yet parity-qualified. No live
snapshot/tool/frame is offered by these outer cfg(test) modules. All changed-source
Rust execution evidence is still owed; static graph tests are not an engine.

The first real149 diagnostic of our own script (SHA256ff2d6208d221a2906504dfbafbf6e1bee678788fb8ae7e40dedf6a5879795625)
passed on2026-11-15 12:57 UTC. Fresh sandbox/profile/private pipes, service UID/GID1000,
normal close exit0 and private inspected entries. It observed semantic form labels,
a same-origin iframe button, opaque cross-origin frame content, hidden-button
exclusion, main-world Map/DOM-prototype poisoning isolation and exact refs starting
above2^53. Frame+loader stayed equal. Source/results under
/tmp/work/wp-browser/own-dom-snapshot-diagnostic. Config mirrors inspected Rust
constants; this was not execution of the Rust CDP/graph adapter and proves none
of the product proxy/Preview/Windows/paint/host actor obligations. New Rust
fixtures cover nonempty graph refusal, isolated context0 rejection, fixed source
and disabled universal access, and loader replacement clearing previous refs.

A separate unchanged-script boundary diagnostic at13:02 UTC exercised1000 button
siblings,40 wrapper levels and80000 Korean characters. It stopped at1500 considered
work items/500 refs, excluded the too-deep button, bounded aggregate graph string
bytes and reported truncation in all three cases. A replaced private map no longer
contained the previous snapshot's ref. Service UID/GID1000, normal close0, private
profile/files and the same script hash are recorded under
/tmp/work/wp-browser/own-dom-boundaries-diagnostic. This is additional script-only
boundary evidence, not a changed Rust test receipt or product engine qualification.

Exact68c1 strict run completed red at13:03 UTC:10 launch/profile lint diagnostics
(unnecessary test qualifications/index idioms, unused execute predicate, two
unused-self methods,64KiB stack buffer and SingletonSocket docs). No compiler/type
or functional result is inferred from that run. The execute check was already
fixed3744; remaining corrections now retain the candidate digest as inspected
instance data, make its fixed path accessor associated, put the bounded read
buffer on heap and correct tests/docs. Candidate::FIXTURE is explicitly test-only
launch data and cannot grant runtime qualification. New snapshot+union strict
must execute on the changed head; source-fast checks only are complete here.

## Actual cleanup receipt before process supervision

[Decision Log]
- Purpose: a process/profile owner must keep cleanup join ownership after a caller
  deadline, rather than infer network quiescence from the completed outer future.
- Existing asset: the proxy actor already cancels and joins streams, upgrades and
  finite Preparation work. Its consuming bounded close was unsuitable for a
  process owner that must retain and resume the actual join on an outer timeout.
- Alternatives: extend that deadline, or separate caller result from real cleanup.
- Choice: modify the existing proxy with begin_close and a retained ClosingProxy.
  Its cancel-safe mutable wait returns an opaque receipt only after actual actor
  success. Failure remains unconfirmed on later waits; no false idempotent success.
- Reason: the future process actor can retain this value with profile lock and
  capacity, apply the existing outer command deadline to its caller, and continue
  actual cleanup without admitting a replacement profile early.
- Impact: no new timeout, permission, listener, connector or product export. Existing
  close still uses the same5s caller deadline. Its timeout is not a cleanup receipt.
  A held-worker fixture drops and resumes the wait while proving preparation
  admission stays occupied. This source is A10-staged and execution is owed.

The process supervisor itself remains next: reuse vessel-proc group/job ownership,
private actual-child pipe handles, finite runtime inspection and bounded discarded
stderr; hold profile/capacity through child wait plus real proxy drain. Candidate
identity and scoped diagnostics cannot qualify a supported engine. Do not release
on a wait/actor failure without cleanup evidence. No new generic spawn/raw CDP
surface or process identity implementation is introduced by this prerequisite.

## Next staged human keyboard adapter

[Decision Log]
- Purpose: trusted page key-down/up and release-all must be distinct from text
  insertion, an arbitrary eval string, a shell, and the native global keyboard.
- Existing assets: HumanInput already distinguishes Key/Text; pointer modifier
  masks, target serialization and uncertain-delivery quarantine can be reused.
  W22b owns the only native interceptor and supplies no raw forwarding yet.
- Alternatives: emulate DOM events in JS, forward raw arbitrary CDP, or extend
  our sealed CDP request vocabulary with a typed finite key grammar.
- Choice: modify that vocabulary with rawKeyDown/keyUp, target-owned held state
  and repeat detection; committed text remains Input.insertText on its separate
  channel. Unsupported physical/layout/IME keys refuse; no text/eval fallback.
- Reason: our human text channel must not insert the same character twice, and
  held keys must release only in their owning page. The official Input CDP schema
  distinguishes raw keys from char/text and specifies modifier/virtual-key units.
- Impact: this is an initial subset (ASCII letters/digits, named navigation and
  modifier keys, F1-F12), not complete browser_keypress chord/layout/IME parity.
  It remains test-staged and does not register a native focus/raw-key producer.
  Actual multi-tab keyboard routing/cancellation and Windows behavior are owed.
