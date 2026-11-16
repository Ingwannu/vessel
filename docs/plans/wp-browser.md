# Browser package: reference, integration and evidence

Status: ADR0078 and its named amendments have contract review; the supported
engine remains **NoEngine**. Contract passes do not approve the later source.
Decision: **modify/reuse** the earlier browser value/policy work and the
existing MCP/client/proc/native layers; **new** engine/CDP/proxy modules are
necessary because none of those layers renders or drives a page. Do not
introduce wry, an Electron host, a second cookie jar or a general CDP tunnel.
The architecture/security contract is ADR 0078.

## Function fixture ownership correction

Exactdb3fc1bfd strict analysis returned one test-helper ownership lint: the
JSON fixture argument was passed by value while the construction macro borrowed
it. Borrow that fixture explicitly. No execution/script/result/loader behavior,
assertion, timeout or bound changes. The queued db3 functional suite retains its
original source and remains meaningful prior-style evidence; a corrected strict
check is required for this changed head. The original failed strict result is
retained. The first borrow correction missed one multiline call site and
therefore returned a test-only type error. Correct that remaining argument too;
all calls were rechecked together and no assertions changed. The original db3
functional source remains intact, with its original by-value helper and no type
mismatch; only the strict correction is replaced.

## Ordinary-page function evaluation (staged)

[Decision Log]
- Purpose: the tool must call the supplied function and return its observed JSON,
  rather than exposing the low-level expression API or echoing its input.
- Existing assets: reuse Page's sealed Runtime.evaluate request, finite command
  owner, pre-await uncertain-delivery state, loader fence and redacted content.
- Alternatives: a caller-selected private world or an untyped result tree would
  cross the snapshot boundary or lose the result's explicit byte/truncation rule.
- Choice: an independently written ordinary-page async wrapper with serialized
  configuration, typed tagged result and opaque JSON syntax validation. No
  reference source/assets are copied, adapted or vendored.
- Effect: invalidate old action refs before execution, even on exceptions or
  cancellation; navigation cannot publish an old-loader result. Missing element
  resolution remains an unsupported path, never an ignored requested reference.

Reference behavior: desktop browser-automation/service.ts:824–881 resolves an
optional element, invokes the function, awaits promises and returns resultJson
plus truncated. Lines1499–1588 specify non-functions/errors, undefined as null,
JSON serialization and the80000/79000 result/preview caps. We implement that
behavior independently. Vessel measures both caps in UTF-8 bytes and also caps
the JSON-escaped preview itself; multibyte/control-heavy output can truncate
earlier. Returned previews remain valid JSON strings. Returned JSON is validated
without inspecting arbitrary user fields. The existing input cap also applies
to the complete encoded wrapper, so extreme escaping/overhead can refuse an
otherwise individually valid function before CDP delivery. Exceptions currently
return typed generic failures; page-supplied error text is not copied into host
diagnostics (a deliberate stricter privacy difference).

The main world and its prototypes are page-controlled; this result is content,
not authority, and does not qualify an engine. No element argument, bridge, file
access, host secret or user-gesture grant is introduced. New result validation
fixtures and a controlled protocol-controller extension are prepared. An actual
Node execution of the independently authored script passed14 behavior cases
(sync/async/undefined/NaN/escaping, non-functions, exceptions/cycles/BigInt and
UTF-8/escaped/emoji truncation). This is JavaScript-only evidence; Rust tests,
CDP execution of this new path and receiving-source strict checks are still owed.
The older a21 artifact is retained for its unchanged Page/wait/snapshot/PNG scope;
it does not contain this function implementation. NoEngine remains in force.

## Real-controller fixture style follow-through

Exact99d46dad8 fixture Clippy reached the new helper and returned four style
findings: two redundant parent qualifications, the now-used listener's former
underscore name, and the HTTP handler's owned request not being consumed.
Use the imported sibling module, name the listener, and consume request parts/
body explicitly. Fixed-host/method refusal, content, assertions, cleanup and
all deadlines remain unchanged. No real controller execution has occurred.

## Next real controller fixture (prepared, unexecuted)

The existing ignored fixed-artifact test now additionally starts a synthetic
allowlisted HTTP fixture on its already-owned proxy listener. After actual child
identity/version verification, the real Page controller opens that fixed origin,
waits for a final document marker using its isolated condition probe, decodes a
same-traversal snapshot, captures PNG transport/geometry and acknowledges target
closure. Browser/driver/proxy/preparation cleanup and retained profile-lock checks
remain; semantic assertions run after cleanup. Reuse existing fixture/command/
shutdown bounds, dependencies and named viewport units; no second launcher,
raw-CDP escape hatch or new engine grant is introduced.

The synthetic handler is NOT the public resolve/classify/pin proxy or a Preview
lease. It never forwards a request and serves only fixed independent fixture
HTML. Normal VM suites continue to ignore actual CFT execution. A new exact
changed-source narrow check and fixed test-artifact export/runtime execution are
owed; the old41 Version-only pass does not execute these additions. Even a later
successful PNG would prove transport/header geometry, not first paint policy,
complete codec/GPU/native rendering, credentials or network isolation. NoEngine.

## Actual exported Rust/CFT child-pipe component

The fixed exact41ecc652 lib-test artifact, SHA67af57b8f9c0520bcae5248960d271c6acc06efd5557edb6417ae17ed7fa535a,
was built/exported by run wp-browser-runtime-artifact-20261116T003750Z-41ecc652.
Manifest, source/features/toolchain/ABI, build UID/GID1000, executable0500 and
approved CFT/shell hashes were checked before its one selected runtime fixture.

The first execution failed0/1 before Chrome launch: the shared temporary
ancestor was group-writable/nonsticky, correctly rejected by Profile's unchanged
permission rule. The original failure is retained. Only the external fixture's
TMPDIR placement changed to a fresh0700 directory beneath root-owned sticky
/tmp; no host permissions, Rust policy, assertions, executable or sandbox flags
changed. The same artifact then passed1/0/0 in6.22s as UID/GID1000.

That test uses the real Rust NUL-pipe driver and actual supervised Child:
Chrome149 reported its typed version, the launched child/bundle was verified,
Close was acknowledged, the driver joined, the owned leader was reaped with an
accepted original-group cleanup receipt, and the Plan's profile lock remained
busy while retained. The external fixture observed no remaining exact-profile
matches and sent no forced signals on the passing run. Private fixture profiles
remain quarantined; this is not Profile::release or reusable slot proof.

This supplies scoped component execution, not supported-engine readiness.
It predates gate57's process diagnostic helper and all later controller/DTO
changes. No real page/PNG, policy proxy, TCP/UDP/ICE/QUIC/WebTransport isolation,
Preview capability/revocation, all-descendant death, GUI, credentials or Windows
qualification is inferred. NoEngine remains until those separate obligations.

## Upload fixture arithmetic correction

Exacta1787e07 browser-filtered proto/MCP tests completed50/0/0, including the
new completed-action, wait-kind and upload-count/UTF-8 cases. Its strict check
and the later3dce01dd2 broad strict check stopped on the same single test-only
integer-division lint. Make the intentional complete-character floor explicit
with checked_div; Unicode/ASCII boundary cases and assertions remain unchanged.
No product input/staging limit or behavior is relaxed. The corrected current
union still needs strict execution and generated TS.

## Gate57 union, TS classification and diagnostic privacy

Merged announced node head bc9cacc67 (gate57 wholly green, protocol25/schema21)
without conflicts, preserving the single pending method table. It adds the
shared opt-in command-diagnostic helper; candidate launch data now chooses its
Redacted policy so process Debug/startup diagnostics cannot disclose private
profile/argv/path data. Actual argv/env/binary/proxy/sandbox behavior is unchanged.
The launch-data fixture now also checks that policy and redacted profile path.
This does not qualify the new process helper/runtime combination.

Exact41ecc652 functional run completed1074/0/2 (browser68/0/2, client93,
MCP20, node334, proto445, RPC58, web56). Its TS run passed three shape fixtures
but stopped before generation because17 new primitive aliases were unclassified.
Classify BrowserId/ElementRef as opaque minted handles; coordinates, revision,
metrics, status and budgets as quantities; URLs/paths as named input; user/page/
function/PNG data as content. Reuse the one sorted ALIASES table and retain its
unclassified/sorted/declaration checks. Add a BrowserResult dependency/branding
fixture, without weakening the failing classification assertion.

The failed41 TS artifact has no generated diff. Do not copy that old snapshot
over the newer gate57 base: new canonical incoming declarations must survive.
Generate once from the current immutable union after these corrections. Current
DTO/aliases/diagnostic privacy and pending upload limits need their own checks;
older scoped successes remain exact older-source evidence.

## Actual condition-script diagnostic

The independently authored condition source SHA16b65f4d43be2e3bb58887445e2160fbb0c6f0dc5b678337527188873c1954a1
ran once as UID/GID1000 with the approved sandboxed CFT149/private child pipes,
cleared private environment and a synthetic allowlisted fixture proxy. Six
text/URL cases, hidden-text exclusion, a literal injection needle and poisoned
main-world String.prototype all produced expected booleans in the private
realm; URL fragment matching and frame/loader observations also passed.
The browser exited0, profile mode0700 and retained entries private; root TCP
listeners0 and post-run exact-profile matching processes0 were observed.

This proves only condition JavaScript/realm semantics for that diagnostic.
It does not execute Rust Page.wait, qualify product proxy/UDP/Preview/Windows,
prove all-descendant containment or authorize any file/engine/profile release.
The exact Rust test artifact is still separately queued. NoEngine remains.

## Completed-fact projection strict follow-through

Exactc5f5d7e1 all-target strict with proto TS reached all three selected crates
and reported only two ownership-style findings: the metadata encoder borrows
BrowserId, and its finite coordinate-field selector is Copy. Fix those without
changing output fields, selectors, content, assertions or limits. The later
upload-bound input slice was not part of that run; a combined changed-source
strict check is owed on the corrected head. No completion is inferred from
compiler analysis alone.

## Upload decoding boundary after row292 contract source pass

V3 accepted the exact upload contract source, with implementation/gates owed.
First implement only the shared wire/MCP edge: MAX_UPLOAD_FILES32 is owned in
proto browser/upload; path bytes reuse RelPath::MAX_BYTES. Reuse the existing
lists decoder/Reject seed through a sibling-private function rather than copying
its excess-payload algorithm. UploadFiles adds its nonempty/count invariant;
MCP checks the raw array count before member conversion and exposes the same
cap in schema. Schema character maxLength is only a coarse envelope: the typed
path decoder enforces UTF-8 bytes. No second byte-limit constant is added.

New direct/serde Unicode path boundaries, exact count/excess malformed sentinel
and MCP count-first fixtures remain unexecuted. None constructs a root anchor,
staging ledger, manifest, source handle or browser selection. In particular,
request validation alone does not implement the contract's GLOBAL retained
count/byte ledger; no upload executor or positive output is enabled.

## Completed action/result discrimination

Reference protocol browser-automation/rpc-schemas.ts:304–350,387–439,452–462
returns operation-specific refs/points/keys/selection/wheel facts and wait kind.
Reuse ElementRef, validated CSS coordinates/deltas and redacted input types.
Applied now requires a closed ActionOutcome instead of a tab-only success;
its single typed tool mapping and the MCP adapter reject a different action,
element/value/chord/delta or wait condition before projecting any success.
Projection consumes completed outcome fields; request fields only validate
correlation and never fill missing backend facts. Flat optional point/ref fields
are omitted when absent, rather than fabricated as zero/null.

The actual Page wait controller now returns the shared WaitMatch enum, avoiding
a second internal kind model. New selector/projection/round-trip fixtures are
unexecuted. These pending value changes need regenerated TS and current-union
checks; exact41 TS generation and older3d wait tests do not validate this later
DTO. BROWSER_PROTOCOL remains None; no previously assigned browser wire format
is replaced, and no tool-name table or call/setting number is introduced.

Upload has no completed-files/result producer yet and cannot pass the generic
Applied branch. Its output path refuses until the separately proposed row292
source/staging/retention contract is reviewed and implemented. All eight other
input actions still require their real executable/actionability producers; the
new typed facts alone cannot create a successful browser action. Log/dialog/
context/list-active/paint/full-page/evaluate-function and actual input work remain
owed, with the engine hidden.

## Wait fixture correction

The exact8a29290a strict run reached the new fixtures and rejected the held
oneshot branch: Receiver yields Result<(),RecvError>, not unit. Require the
actual successful arrival result explicitly before dropping the wait future;
no barrier, assertion or deadline is removed. The queued functional job has
not executed this known compiler failure and must be superseded only through
the queued-only cancellation procedure, retaining the original receipt.

## Bounded wait controller follow-through

Reference desktop browser-automation/service.ts:68–70,986–1040 polls text or
URL substring immediately and every25ms using the existing5s default/30s cap.
Reuse WaitFor, WaitBudget, the existing CDP driver and isolated-realm commands.
A new sealed ConditionProbe carries a borrowed typed condition as JSON data;
caller text never becomes function source or a trusted user gesture. The shared
foreign method name Runtime.callFunctionOn remains one Method::CallFunction
entry for both independently authored snapshot and condition adapters.

The page controller probes under before/after frame+loader fences. A changed
loader refuses/invalidate rather than publishing an old-document match. This
is a deliberate fail-closed difference from the reference's unfenced polling;
a later caller can retry against its newly observed document. Outer timeout
reuses the existing host wait budget and retains its Elapsed cause. Cancellation
inside CDP keeps Page's existing Uncertain state; no completed wait invents a
navigation or effect receipt. Polling sleeps are production scheduling; fixture
ordering uses oneshot barriers rather than timing sleeps.

Four controlled protocol fixtures cover text/URL kinds and repeated false
probes, loader replacement, caller cancellation during a held reply and an
exhausted explicit budget. One serialized hostile-input fixture guards source
separation. They are unexecuted; actual DOM/CFT, world loss and product actor
qualification remain owed. Runtime export41ecc652 predates this controller and
cannot qualify it. No protocol/setting/offering or product engine is enabled.

## Evaluation result projection follow-through

Reference protocol rpc-schemas.ts:443–449 and server browser-tools/tools.ts:770–824
require resultJson as a string and an explicit truncation fact. Reuse CappedText
and its existing UTF-8 bound; project observed output without parsing truncated
JSON or copying the request function into a success. No new DTO, bound, engine
method or authority is needed. Two fixtures distinguish observed output from
request source and preserve Unicode truncation. They are not executed yet.
The previously queued41ecc652 strict/tests/TS checks retain their exact source
scope and do not validate this later MCP-only projection.

## Resume: announced node union and stopped-time results

Merged announced node head ed4cd137 (gate56b counted17/18; the named
startSession stays /new intermittent remains open). The architecture table
retains both independently owned browser and plugin rows; no code conflict
or protocol assignment was introduced.

Exact79439089 functional run wp-browser-execution-tests-20261115T185714Z-79439089
completed1039/0/2: browser68/0/2, client93/0, MCP20/0, node329/0,
proto419/0, RPC54/0 and web56/0. Both ignored tests require the approved
runtime account/artifact; this does not execute CFT or qualify an engine.
The strict run failed only two default_trait_access test findings; the TS run
failed its browser-settings fixture because ts-rs12 requires a Config argument.
Reuse the existing ServicePorts fixture's Config::new pattern and name
ScreenshotExtent explicitly. No assertion or product behavior changes.
Generated TS artifacts from that failed compilation contain no changed files;
fresh generation remains owed. The completed runs were attached, never resubmitted.

Next: changed-union narrow strict/TS checks and functional checks, then one
fixed test-executable export after strict succeeds. Actual CFT pipe execution,
D2 verified connection, retained cleanup, full actions and native wiring remain
owed. NoEngine/hidden and BROWSER_PROTOCOL=None are unchanged.

## Current implementation boundary

| Area | Implemented source | Remaining acceptance |
| --- | --- | --- |
| Node protocol | Typed browser values, four pending method rows and closed refusal handlers; BROWSER_PROTOCOL=None | Assigned version and reviewed real owner wiring; no browser method is enabled |
| Settings | Shared AA writer consumer, strict present-only string preference and composed reply mask | Changed-source writer/permission/protocol checks and real session-start lease; saved On grants no runtime |
| Execution | Test-only proxy/preparation, sealed CDP, page/input/ref/snapshot and candidate profile/launch models | Actual retained child-pipe supervisor, complete operation results, runtime and process/profile cleanup proof |
| Preview | Accepted exact-origin service capability and connect_verified design | Checked D2 actual ownership/scheme/accepted-peer producer and revocation; no independent metadata-derived dial |
| Native | Separate app worktree owns the sole pane NoEngine renderer and staged decoded-frame geometry | Actual fenced stream, bounded resident image lifetime, GPU/paint/input/focus/close and captures |
| Evidence | Scoped earlier narrow runs, bundle-read component and four minimal-DOM script fixtures | Latest complete source union, actual Rust/browser execution, security qualification, whole gates and reviewers |

All execution modules remain outer cfg(test), crate-private. The single future
qualified-actor wiring commit must remove staging only with its real consumer
and required proof. No fixture, source-only review, readable bundle or saved
preference may become a supported engine. The sections below retain chronological
decisions and their exact scoped evidence; early checkpoints are historical.

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

## Initial document checkpoint (historical)

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
the page as before. The later staged keyboard section below extends the sealed
vocabulary with a finite human raw-key subset; unrecognized keys refuse without
a text/eval fallback. These paths remain outer cfg(test), not a supported or
advertised input surface; their new Rust fixtures remain unexecuted.

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


The actual149 two-target keyboard diagnostic completed at13:19 UTC: while B was
active, A's session received Control+A then Backspace; A became empty, B retained
its synthetic value, and only A's field observed3 trusted keydowns. Same browser
window, service UID/GID1000, sandbox on, normal close0 and private inspected
profile/files. Script/result: /tmp/work/wp-browser/pipe-multitab-keyboard.
This mirrors the raw-key wire shape but does not execute the Rust adapter, prove
all keys/layout/IME/pointer/dialog paths, or settle simultaneous actor cancellation,
Windows, product proxy, private Preview or the future input-focus transaction.

New human key state normalizes ASCII physical identity for repeated upper/lower
letter keydowns, refuses keyup for a key this target never pressed and releases
only this target's held set. It never accepts arbitrary editing commands or
keyboard text: committed text uses the separate Text channel. Shared pointer/key
modifier masks now have one CDP owner. Node/renderer/MCP admission is unchanged.

## Snapshot form-state correction under qualification

Disabled form state now uses the DOM's native :disabled semantics so a fieldset's
inherited disabling is retained. A contenteditable ref retains an exact bounded
text-node value fingerprint (including whitespace) instead of an empty value.
That read shares the traversal's work/text budgets; if it cannot complete, no ref
is assigned and truncation is explicit. These are our own DOM operations, not
copied reference code. The updated real149 semantic/form fixture is in flight;
old script-hash evidence does not qualify this changed source. Engine stays closed.

## Pending browser setting value plumbing after row220 CONTRACT PASS

Proto now carries default-Off browser_tools and a present-only strict decoder on
the optional change. Null/boolean/object/array refuse instead of becoming None;
unknown strings remain a named Unknown for explicit refusal. Any named value
(including Off/Unknown) requires BROWSER_PROTOCOL at every envelope hop. Reply
projection composes the existing recovery mask and omits unsupported On/Unknown.
Required settings literal adaptations are otherwise unchanged old behavior.

This commit does NOT integrate persistence: shared settings writer repair row204
(aa70782dc source pass/gate owed) and privacy repair row187 are not yet in this
announced base. No writer is copied. The existing settings owner currently rejects
every explicit browser change before its lock/I/O (unknown choice Invalid, known
choice unavailable); the generic service additionally checks direct human Admin.
Effective browser preference remains Off. The field and codec are pending value
plumbing only, not a consumer-ready saved preference or session-start offer.
Once the checked shared owner lands, extend its existing overlay/merge under the
same guard/tracker and remove the pending refusal with the actual browser wiring.
No protocol number or supported engine is assigned by these types.

Exact47f43541 strict completed red on13 snapshot lint diagnostics. They are now
fixed: typed binding comparison supplies its actual field consumer, fixture math
and explicit results/enum matches are checked, complete takes a borrowed graph,
and rendering appends without a redundant format allocation. The earlier run
predates the later keyboard/proxy-drain/settings work, whose current combined
strict/tests remain owed. No assertions or bounds were weakened to pass it.

On13:25 UTC the debugger-EOF diagnostic observed14 actual owned Chrome processes
by exact fixture profile/process group and kernel PID fds. All observed identities
exited without forced cleanup when the debugger pipes closed; leader exit0 and
profile/files stayed private. This is a finite observed set, not every possible
future descendant's death, the Rust process supervisor, Preview or Windows proof.
Artifacts: /tmp/work/wp-browser/pipe-eof-lifetime-diagnostic. The shared checked-wait
producer b37a3ee8 separately means leader reap plus original accepted cleanup;
its contract does not turn that receipt into an all-descendant OS-death proof.

The changed own DOM script5b091b2fa89c54700a0675449f2762261264a17e027a797eadf2d40d74bfb6f9
passed its actual149 expanded semantic fixture: inherited fieldset disabling,
exact contenteditable whitespace fingerprints, prior same/cross-origin/hidden/
main-prototype canaries and exact big refs. UID/GID1000, sandbox on, exit0 and
private inspected entries. Rust path and product qualification still separate.

## Staged client watch cursor, transport not yet wired

[Decision Log]
- Purpose: a browser frame/state must never silently retarget another node,
  subscription, workspace, tab/profile or document/viewport context.
- Existing assets: BrowserWatched/BrowserUpdate and the canonical typed metadata
  already exist; their per-watch sequence is distinct from BrowserRevision.
  The generic EventRecord/log cursor is unsuitable: it persists/resumes content.
- Choice: a client-owned ephemeral cursor validates immutable scope, exact sequence,
  monotonic announced contexts and matching frame CSS viewport before any owner or
  image decoder sees the update. Any mismatch poisons the watch; no gap skipping.
- Reason: metadata is not authority or paint proof, but an actual transport owner
  must prevent cross-resource/stale deliveries before rendering. A new connection
  gets a new cursor; it cannot retarget, resume a durable log or reuse a closed one.
- Impact: outer cfg(test) model until qualified browser-watch transport wiring.
  No ServerFrame, RPC sink/client subscription, node producer or visible frame is
  added here. Tests use real PNG bytes of our own synthetic canary solely for
  metadata admission; no engine/codec/paint/security qualification is inferred.

The announced node82ae7c91 was merged unchanged (staged-row support/stop signals/
provider child values/CLI output seam); it does not contain the checked settings
writer or checked-wait producer. App independently merged announced packagebase7
(d18bb794), with no browser registry/visibility/focus override or node-head merge.
Native f05b1051 changed-union view/app strict passed0 before that later base7 merge.
No native whole/capture or union acceptance is inferred for the new base.

Coordinator authorized a fixed exact-SHA vessel-browser lib-test export, now
installed by T. It builds only that crate's --locked test executable on the VM,
exports immutable0500 plus commit/features/toolchain/UID/SHA/ABI proof into a new
out directory, then selected fixtures may run on the approved CFT host as its
service account with sandbox on. No arbitrary target scp/local Cargo/second
runtime is used. No actual artifact has been exported/run by W14 yet; existing
remote ordinary tests remain intact, and source/runtime qualification is owed.

## Candidate bundle observation, not qualification

[Decision Log]
- Purpose: an unchanged chrome executable path/hash must not silently preserve
  a candidate launch identity after another runtime asset has been replaced.
- Existing assets: Candidate already streams SHA-256 under finite Preparation and
  checks root ownership/nonwritable ancestry. It observes only Chrome and shell.
- Alternatives: trust only the executable, or observe the complete fixed bundle
  and retain metadata versions for a post-start unchanged check.
- Choice: extend that same inspector with a bounded sorted bundle manifest. One
  named owner limits4096 entries,16 directory levels and1GiB total regular bytes;
  files reuse the existing512MiB per-file/64KiB streaming owners. Links, special
  files, unsafe ownership/modes and incomplete/changing reads refuse.
- Reason: bundle resources/libraries/snapshots must not inherit earlier evidence
  just because the main executable was unchanged. Relative names, file kind/mode,
  byte count and file digest participate in one deterministic observed digest.
- Impact: this remains a candidate observation, not a QualifiedRuntime. Node
  binary/configuration, OS/dependencies/sandbox policy, transport and all required
  actual runtime/Preview proofs remain separate. The actual fixed bundle currently
  has303 regular files/396335288bytes/depth3, root-owned/nonwritable to service.
  No files, installation, permissions or host policy are changed by inspection.

Bundle byte admission counts actual streamed bytes through the shared digest
owner, not merely the pre-read metadata length. Stable metadata is checked on the
opened file, after hashing and again over all retained entries. One fixed read
quantum may observe an over-budget chunk only to refuse it; no digest is accepted
for that excess. Root ownership/write masks reuse the existing candidate policy.
The staged explicit ignored fixture observes the approved CFT bundle as non-root
through finite Preparation. It starts no browser and can prove only that component's
real host read/identity checks; it cannot grant runtime/engine/network qualification.

Target context/ref exhaustion now quarantines the actual page controller for
close, rather than merely returning an error while leaving other commands active.
This follows the existing no-wrap contract and prevents further target operations
after its revision/namespace can no longer identify new work. A protocol-peer
fixture forces the final revision, confirms the next command refuses before CDP,
and verifies only close remains available. The peer is not a browser engine;
current changed-source execution evidence remains owed.

## Scoped remote evidence, not current whole-package acceptance

Exact61c4936b functional five-crate lib suite completed518/0/0 at14:07 UTC, all
prescribed steps0 (node-launch-watch-output-tests). It covers the then-current
launcher/watch values/MCP output/announced ede union, not the later snapshot,
keyboard, drain, settings, cursor or bundle changes.
Exact60c17b44 browser lib suite completed39/0/0 at14:20 UTC, all prescribed steps0
(node-own-snapshot-tests). It executes the initial independently written snapshot
host/decoder/controller and earlier browser primitives, before later lint/form/
keyboard/drain/settings/cursor/bundle changes. No assertions were waived and no
runtime/paint/Preview/package approval follows from either receipt.

The fixed export of exactd37826ca is queued on the terminal five-crate label's
warm target (run wp-browser-execution-tests-20261115T142049Z-d37826ca, out
candidate-test-executable). It compiles but does not execute tests on the VM.
Its explicit approved-host bundle fixture reads only the runtime bundle and
starts no browser; passing that component will not qualify a supported engine.
Later source changes cannot inherit its exact-SHA evidence.

## Development-only checked wait dependency (coordinator exception)

The coordinator explicitly authorized merging EXACT b37a3ee836c96951061c4006897eb80ac72bb2e0
in this private node tree for development. The only conflict was append-only
ADR0001: retained both the browser boundary/launch sections and the producer's
checked-cleanup section verbatim. No Proc implementation or producer assertion
was changed. This is not adoption, a gated union or a current package head.

Before ANY new gate, review handoff or adoption request on this union, merge the
announced gate49 node head carrying the same producer. Nothing built on this
pre-announced dependency merge goes to the integrator/lead. Already pinned
bba strict/d378 export precede the dependency and keep their own evidence scope.
NoEngine remains. OwnedGroupExit is leader reap+accepted original cleanup, never
an all-descendant OS-death proof or permission for premature profile/slot release.

Exactbba strict failed only one proto documentation backtick at14:38 before later
crate analysis; corrected00bb27a7f without weakening code/tests. Current combined
strict remains owed after the required announced-head merge. Do not treat that
blocked broad check as evidence for the later cursor/bundle additions.


## Staged profile input ownership

[Decision Log]
- Purpose: a target's activation and its entire input transaction must share one
  profile-wide order, including cancellation while another target is waiting.
- Existing assets: the page controller already retains typed pressed keys and
  serial target commands; Tokio mutexes, bounded semaphores and TaskTracker are
  the existing task/cleanup owners. A target-local mutex cannot keep another tab
  from becoming active between two input events. No external dependency is needed.
- Choice: stage one generic profile input lane under outer cfg(test), below the
  actual node's target/context admission. Its64 active+queued permits live in
  registered workers, with activation+apply under the same async mutex. Caller
  timeout does not cancel the retained worker or release its permit early.
- Why: pending withdrawal and start compete on one atomic state, so a reported
  NotDelivered cannot race a later activation. Once started, cancellation/error
  closes the profile permanently and starts one independently tracked stop/drain;
  no next target enters before cleanup. Shutdown registration shares the admission
  gate and a cancelled shutdown waiter cannot cancel the actual cleanup task.
- Impact: controlled fixtures cover cross-target ordering, busy admission, caller
  abort/delivered timeout, queued withdrawal, backend failure and retained stop
  refusal. A stop error is retained and never retried; the trusted driver must
  itself quarantine physical resources if actual cleanup is unconfirmed. This
  model is not that driver, a process-death receipt or an engine qualification.
  Product removal of cfg(test) remains the named qualified node-actor wiring
  commit. No raw native key forwarder, focus ticket or visible engine is enabled.

Current source-fast rustfmt/size/diff checks passed. No changed-source compile or
execution result is claimed: the development-only b37 dependency must first be
joined by the announced gate49 head before the next gate/review/adoption request.


The same pending/start/withdraw owner now protects the CDP command queue. Its
previous read-only delivery flag could race a timed-out queued command's later
socket write. The sealed request driver must atomically start before its first
write; a caller timeout/drop withdraws that same pending state. An independently
held-write peer fixture blocks one write, times out a queued request, then joins
shutdown and asserts the withdrawn request never reached the transport. This is
byte-transport cancellation evidence when executed, not actual browser proof.

The command enqueue/typed result wait are separated by a private Queued value;
its caller guard remains alive through result decoding and withdraws on drop.
This keeps the delivery-state owner explicit without changing the public sealed
request vocabulary or selecting a different ordinary command deadline.


## Retained debugger close ownership

[Decision Log]
- Purpose: caller cancellation must not remove the actual debugger driver's join
  and let another close report success from an empty slot.
- Existing constraint: Client.shutdown took the JoinHandle out of Shared before
  awaiting it; a dropped shutdown future detached that handle. ClosingProxy already
  distinguishes its retained task from the caller's deadline.
- Choice: retain Waiting/Done inside the same connection's Tokio mutex while
  joining, including across cancellation. Store the first completed typed result
  with its original error source for all later callers. A close deadline asks only
  this socket driver to abort and retains its join for actual later confirmation.
- Why: one connection owner and one task outcome prevent an invented second close
  acknowledgement. Closing also cancels an in-progress transport write directly;
  an already-started request remains uncertain rather than being replayed.
- Impact: controlled task and held-write fixtures exercise cancellation, repeated
  failure/success and delivered-request uncertainty. These are staged transport
  ownership fixtures, not process/profile/Preview cleanup or engine qualification.

Existing overflow/wrong-session shutdown assertions now inspect the retained
Cleanup source and still require the original Busy/Protocol failure respectively;
no failure is weakened to generic is_err or an accepted successful close.


## Exact artifact and service-account bundle component

The fixed test-executable export of d37826caa433167d756ebaf0da01422ea069d21f
completed rc0 in run wp-browser-execution-tests-20261115T142049Z-d37826ca. The
single0500 test binary's SHA256 is
`e6c781cb536e53adc857c1f0d2f0c811bf202d8080513541582e6349d5b48397`;
its manifest records source/features/toolchain/build UID and matching host ABI.

The approved-host read-only bundle fixture first failed0/1 with PermissionDenied
under the real non-root account: the provisioning-owned dependency-list files
had0600 mode. The coordinator corrected ONLY those two root-owned package lists
to0644. The worker verified their readability/ownership/mode and unchanged Chrome
and shell digests, then reran the SAME artifact once:1/0/0 in3.93 seconds, exit0.
Both original failure and corrected-state argv/completion receipts are retained.
The worker changed no runtime, policy or file permissions. This fixture starts no
browser: it proves the actual account can read/observe this bundle, never a
qualified launch, isolation, Preview permission, rendering or supported engine.


## Positive forwarding fixtures below destination admission

[Decision Log]
- Purpose: negative private-address canaries alone cannot show that accepted HTTP
  streaming, requested WebSocket upgrades and CONNECT tunnels work or drain.
- Existing assets: the same forwarding implementation already receives a pinned
  TcpStream after dial_public. Reuse it, Hyper and controlled own ephemeral TCP
  peers; no new resolver, connector trait, dependency or product policy bypass.
- Choice: extract a private connected-stream helper, called by serve only after
  the existing destination parser and dial_public success. Test fixtures invoke
  that same helper with a controlled local stream after parsing their request.
- Why: the fixture starts strictly below destination admission and exercises real
  HTTP framing/streaming/upgrades and tracked cancellation without making a
  loopback address public or permitting real outbound test traffic.
- Impact: assert origin-form/Host rewrite, hop/proxy credential removal, prefix
  delivery before the backend completes, bidirectional upgrade/tunnel bytes and
  EOF/reset after actual joined shutdown. These fixtures do not qualify DNS,
  public/private policy, Chromium, Preview authority, TLS or UDP isolation.

The three new forwarding fixtures are UNEXECUTED source at this checkpoint;
changed-source Rust checks wait for the required announced gate49 dependency
join. Source-only rustfmt/size/diff checks pass; no policy/runtime result follows.


## Target cancellation before acknowledgement

[Decision Log]
- Purpose: dropping a page operation after CDP delivery must not leave its target
  Active and permit another mutation against an uncertain document/input state.
- Existing constraint: handling only execute's Err result misses future drop,
  because cancellation never reaches that match arm. The new atomic CDP queue
  prevents late NotDelivered writes but cannot restore a target's known state.
- Choice: mark Uncertain before the first awaited command and restore Active only
  after a typed acknowledgement. Reuse this single execute path for operations.
  Close borrows the resident target instead of consuming it before its ACK; only
  acknowledged closure makes it Closed, and later non-close commands refuse.
- Why: page lifetime and target capacity belong to the retained profile actor,
  not the caller's waiter. Cancelled close must not lose the only owned target
  handle. A repeated acknowledged close reports the same completed fact without
  another backend mutation; an uncertain close does not fabricate that fact.
- Impact: controlled delayed-reply fixtures cover cancelled reload/close, refused
  subsequent operations and idempotent acknowledged close. They are unexecuted
  staged source, not a qualified actor, actual browser close or cleanup proof.

Close discards current references without allocating a new generation, so
exhaustion cannot block terminal cleanup. The Closed state cannot be moved back
to Uncertain by revision invalidation. The delayed-close fixture waits for its
late peer ACK before confirming the cancelled operation did not restore state.


The crate's [execution staging guide](../../crates/vessel-browser/README.md)
records current module responsibilities, pending data flow, cancellation and
cleanup boundaries, one-owner bounds and exact verification limits. It does not
advertise the staged primitives as a supported runtime or product browser.


The shared parity tracker D3 rows509–513 now use PLAN section7's sole W14
ownership instead of the withdrawn duplicate W18 assignment. They remain
skeleton, with the unimplemented annotation/attachment row marked missing.
Each row names the actual NoEngine/dependency boundary; no row is built/passed
from source review, a bounded component test or another package's gate.


Input result handoff also has an explicit bounded receipt. An answer successfully
sent into a oneshot does not prove the caller consumed it: it may time out/drop
immediately afterwards. The worker retains its serial slot/permit through that
handoff. Only the run caller's consumed-result receipt completes it; abandonment
or profile close triggers the same retained stop/drain. One controlled fixture
holds that exact receipt after backend success, confirms Busy/capacity retention,
then abandons it and confirms cleanup before capacity returns. This source is
unexecuted until the required announced-head union permits changed-source tests.


The input lane's ordering fixture does not prove cross-callback key isolation.
A tool chord must be supplied as one transaction. Human down/up callbacks remain
separate transactions, so the actual Driver.activate must release the previous
target's held keys/buttons with real acknowledgements before changing targets;
an uncertain release stops the profile. This is an explicit trusted-driver
obligation, not behavior supplied by the generic mutex or its fake fixture.
Actual held-key/pointer multi-target/connection/focus tests and the W22 owner
transaction remain required before raw forwarding or an engine can be enabled.


A fresh reference result-schema/output comparison is retained in
[browser-result-parity.md](browser-result-parity.md). It identifies the current
foundation's missing per-operation facts, dialog envelope, log timing/source
fields and MCP summary/projection as pre-qualification debt. The catalogue/input
work must not be relabelled complete22-tool behavior. No new DTO, method, number,
renderer or engine is enabled by that checklist; missing facts cannot be filled
with request echoes or joined from a later document.


## Staged completed-tab MCP projection

[Decision Log]
- Purpose: the foundation's generic Tab shape must not leak an internal nested
  descriptor as the final new-tab/navigation/resize result, or echo a requested
  URL/size instead of a redirect/effective backend observation.
- Existing assets: BrowserResult::Tab already carries the completed typed node
  descriptor; the single BrowserTool enum and output edge already match result
  kind/target before exposure. No new DTO or external dependency is needed.
- Choice: independently project those existing facts into flat operation results:
  creation id/workspace/address, navigation id/address, traversal/reload id,
  resize id/effective dimensions. The operation discriminator still uses the
  one catalogue's name, not a second string table.
- Why: typed backend observations are sufficient here, so missing-engine facts
  need not be manufactured or queried from a later document. No favicon or
  private profile descriptor is serialized into these completed-tab results.
- Impact: staged-only fixtures deliberately return a different observed URL and
  viewport from the request, and check fixed/responsive projection and traversal
  shape. These tests are UNEXECUTED pending the announced-node join. Other result,
  dialog, log, summary/context and actual execution debts remain in the linked
  checklist; this change enables no tool, method or browser engine.


## Announced gate50c dependency join and actual preference owner

Merged announced9c150c6efbff583762d02e6a8576b3fa66cc7ae9 as9d6bc48dc,
not a trial head or rebase. It contains the EXACT portable b37 already used for
private development, the checked AA writer and privacy decoder. The prior
pre-announced dependency hold is now satisfied. Eight append/exhaustive-list
conflicts retain canonical registry/diagnostics/detach/book values plus W14's
browser module/refusals/samples; the four None browser rows remain at the END of
the single method table. Checked Proc implementation/assertions are unchanged.
No ConnectionLife consumer was invented to remove staging expectations.

[Decision Log]
- Purpose: save the explicit agent-browser preference without another file,
  writer, runtime grant or a cancelled write overtaking a later update.
- Existing asset: announced AA holds an owned saved-settings guard through its
  independently tracked write and runtime publication, and preserves later or
  unreadable files with the shared privacy-safe decoder. Reuse that exact owner.
- Choice: validate the browser choice while holding its owned guard, merge only
  a named field into next, and use the existing persist. Missing/unknown saved
  choices remain effectively Off; omission preserves a saved choice. Known On/Off
  now persist, rather than the old pre-AA BrowserPending placeholder refusal.
- Why: preference persistence and browser admission are different facts. A saved
  On is not an engine qualification, provider start or offered-tool lease. Public
  mutations require an assigned browser field version at every hop and direct
  human Admin; the constant remains None, so public named mutations are refused.
  No method number or tool offering is enabled.
- Impact: actual-writer fixtures cover restart, unrelated concurrent updates,
  all-or-nothing unknown refusal, write failure, cancellation/serialization,
  service-account file ownership/modes, and future/unreadable original backups.
  Existing AA writer code/negative controls are not copied or changed; only its
  test fixture/gate helpers are shared with the browser's own tests.

BrowserTools also now decodes ONLY a JSON string. Its generic enum form must not
accept a unit-variant object such as an on/null map as a preference. Direct enum,
named request and real saved-file fixtures require those object forms to refuse;
null/malformed input cannot become omission. Later strings remain Unknown/Off and
follow the existing preserved-original backup rule. These new implementation
checks are still unexecuted at this source checkpoint; prior scoped passes do
not qualify the changed writer consumer, decoder, field or actor admission.


The first exact3f joined strict check returned compile errors before the complete
changed-source analysis: the announced web method inventory needed four explicit
browser refusals, and five new settings-test calls discarded NodeSettings values.
The web bridge now explicitly refuses all four native/MCP browser channels with
inventory/raw-call fixtures, never a wildcard or an alternate frame/control path.
The settings tests now assert those returned preference/parking facts rather than
ignore them. Directory-backup tests also fail on enumeration errors instead of
filtering them out. No test assertion or policy was weakened. Current corrected
source still owes the changed-source checks; the failed exact3f receipt remains.


The browser preference's optional request field explicitly generates a non-null
optional TS property. The pinned ts-rs12.0.1 API distinguishes ts(optional) from
optional=nullable; only this strict field uses the former. Existing nullable
legacy settings are unchanged. A feature-ts test named for the fixed TypeScript
selection checks the declaration against the decoder's omission/null contract.
Default-feature strict/functional runs stay pinned to0b because this change is
feature-ts metadata/test only; the still-queued TS run is superseded once after
terminal cancellation, and its new exact-head compilation/generation is owed.

The exact0b strict run stopped at the CLI diagnostic error projection: its
closed ErrorDetail match needed Browser in the existing non-diagnostic fallback.
That explicit arm preserves the safe generic diagnostic text; browser reasons
are not interpreted as diagnostic collector evidence. No wildcard or private
error text is exposed. The announced gate52 head1c762ccf is merged before the
corrected strict run; the earlier functional and TS jobs retain their exact
source provenance and do not qualify this later union.


## Same-traversal snapshot document facts

[Decision Log]
- Purpose: snapshot address/title must describe the same traversal, rather than
  a later document joined to older element references.
- Existing assets: independently authored isolated-world script and typed graph,
  PageText redaction/UTF-8 cap, host reference namespace and before/after loader fence.
- Choice: modify/reuse the single graph return to carry a required document
  descriptor captured synchronously with traversal. Its address is display-only
  PageText, never BrowserUrl authorization, and is refused rather than shortened.
  Title and node text share the existing aggregate budget and truncation flag.
- Alternatives: a later Runtime evaluation or tab inventory lookup would not
  establish the descriptor's association with the old snapshot. No extra CDP
  endpoint, dependency or copied reference implementation is needed.
- Impact: MCP explicitly projects aria-yaml/text/truncation/named stats and these
  observed facts. Workspace context remains the authenticated owner adapter's
  responsibility. This staged result does not provide a browser, authority,
  actionability, complete accessibility tree or runtime qualification.

Reference behavior: packages/protocol/src/browser-automation/rpc-schemas.ts:292–301
requires URL/title/aria-yaml plus measured snapshot fields. The stronger combined
UTF-8 budget and display-only address keep ADR0078's bounded/private data rule.

The same source's minimal-DOM Node fixtures execute the independently authored
script:4 passed/0 failed. They check one read of address/title, a display-only
non-HTTP address, oversized address refusal and shared UTF-8 truncation. This
is script semantics only, not browser DOM/CDP or Rust runtime evidence. New Rust
fixtures also require metadata, reject aggregate overflow and prevent YAML
escaping from spending the metadata budget a second time; their execution and
updated TS generation remain owed at the next changed-source narrow batch.


## Changed snapshot document script: actual isolated-world diagnostic

The script at1d4b93f5a (SHA256 c64e39d0db845ca3871825ce4519da958a62851820c3ce57a57412c4430e9828)
was exercised through the approved Chrome149 candidate's private child pipe.
The disposable probe ran as UID/GID1000 with sandbox enabled, cleared private
environment, synthetic proxy/page and700 profile. Binary/shell hashes match
the earlier candidate. Same-origin frame content, cross-origin opacity, hidden
content exclusion, hostile main-world Map/prototype separation and exact refs
above2^53 remained intact. The first snapshot kept its observed title/address
after a same-document title/fragment change; the next traversal reported the
new values. Main frame/loader remained stable. The owned leader exited0, profile
inspection found zero nonprivate entries and a post-run exact-profile scan found
zero matching processes. No service, permission, runtime installation or real
credentials were changed.

Evidence is the snapshot-document-runtime diagnostic's result/provenance and two
graph files. Its executed probe is retained unchanged. The total-alarm handler
was incomplete; no alarm fired and normal close completed. A corrected future
probe is explicitly unexecuted: this receipt proves no timeout cleanup.

This is Python/CDP execution of our script, not Rust decoding, the product policy
proxy, private Preview ownership/revocation, process/profile supervisor, UDP/ICE/
QUIC/WebTransport or Windows qualification. NoEngine and pending versions remain.


## Changed-source checkpoint after the first full browser analysis

Exactb1e7f6e9 strict runwp-browser-node-narrow-20261115T172100Z-b1e7f6e9
returned13 findings: the bundle manifest helper still declared a u8 parameter
after its field became typed Kind (three related type errors); eight test path/
name lints; two forbidden unreachable macros in watch fixtures. The helper now
accepts Kind while retaining the exact tag encoding. Test paths are unqualified
where already imported, the cancellation barrier receiver is named resume, and
the four watch-scope cases are a closed enum instead of a numeric range with
unreachable fallback. An invalid frame fixture fails explicitly. Existing scope,
late-ACK, cleanup, fingerprint and viewport assertions remain intact.

That run did not execute functional tests. The queued old0b functional source
shares the proven bundle type error, and3c TS predates the observed-document DTO
and announced registry version. Both own queued runs were cancelled through
queued-only admission and independently observed terminal cancelled/143 before
their one replacements. The new combined batch includes these corrections and
the same-traversal document source; older receipts do not qualify that union.

The exact191 combined strict followup returned two fixture-style findings in the
new closed ScopeChange matrix: its enum declaration followed the opening binding,
and the block assignment lacked a semicolon. The declaration now precedes that
binding and the assignment terminates explicitly. Cases and assertions are
unchanged. Functional and TS jobs remain exact191 because this correction changes
only test declaration/statement layout, not wire or runtime behavior; no later
whole/union acceptance is inferred. A new strict check is still owed.


## Prepared real Rust child-pipe component fixture

[Decision Log]
- Purpose: Python/CFT diagnostics and fake Rust byte peers do not prove the
  actual Rust CDP driver can communicate through the approved child's pipes.
- Reuse: the existing fixed candidate Plan, Proc-owned Spawned handles, finite
  Preparation scope and Client's private Pipe/start path; no new decoder, socket
  endpoint, generic raw-CDP constructor, runtime install or external dependency.
- Choice: a Linux-only ignored diagnostic fixture inspects the actual candidate,
  launches only that plan, keeps Plan/Child through read-only version observation
  and post-start identity check, and then closes/drains the owned resources.
- Impact: normal VM suites never launch Chrome; only the fixed exact-SHA lib-test
  artifact may run the selected fixture on the approved service-account host.
  Its synthetic private profile is kept at a unique quarantine path, never used
  as General/Preview storage. No shared profile, capacity token or engine is
  released/granted from a leader/accepted-group receipt. That receipt is not
  proof of every descendant's death; product supervision remains unimplemented.
  The fixture reuses existing CDP shutdown and test outer deadlines and discards
  stderr without retaining browser text. No source or execution pass yet.

Exact429 strict then reached the new MCP snapshot test and found one constructor
argument mismatch: PageText::capped takes &str, not an owned String. The fixture
now borrows its literal; expected output and assertions are unchanged. The old
queued191 functional source shares this compiler error and must be superseded
only after terminal queued cancellation. The191 TS source is unchanged by these
MCP/CDP test corrections and keeps its separate exact job.

The announced gate53 head93970acbb5c44cfc9649e2e93c3889575592f0cd is now
merged as1acdc04b92c6e10f526c1ceb5c85b40c21c76d28, with no conflicts or rebase.
Schema21 belongs to lifecycle; browser adds no migration or protocol number.
No own browser fixture uses the old unit ListSessions constructor. The older191
TS job was subsequently superseded because this canonical base and its listing
shape changed: terminal queued cancellation/143 was verified before replacement.
Its receipt is retained and supplies no execution result. The known-red191
functional job was likewise cancelled while queued and terminal143 was verified.

The ignored candidate diagnostic uses the execution wrapper's fresh private
short TMPDIR, with a short nonce prefix and the existing profile-path guard.
It keeps private fixture files for inspection rather than deleting uncertain
process storage. The wrapper places temporary/runtime data on its approved
artifact volume; no host-specific scratch path is hard-coded in source. It is
still uncompiled/unexecuted and enables no browser API or product supervisor.


## Full execution-module strict analysis followup

Exact888 strict reached vessel-browser and returned seven findings: enqueue
borrowed its owned request, physical-key fields repeated the old Key type name,
function-key arithmetic was unchecked, event construction copied rather than
consumed key labels, a snapshot fixture constant followed bindings, and a held
reply's BrowserVersion was discarded.

The queue helper now borrows the sealed request. Its async owner explicitly
drops the second typed input after serialization and before waiting for the
reply, so private page input is not retained twice through that deadline; the
encoded command and any harness/session history remain separate copies. The
PhysicalKey domain type moves its validated labels into the outgoing event and
checks the unchanged function-key offset rather than allowing wrapped codes.
Fixture declaration order is corrected and the held version's actual product
is asserted. Existing bounds, wire data, input-state and cancellation assertions
are unchanged; no lint allowance or runtime permission was added. New strict
evidence remains owed; the888 normal functional job is retained as older-source
evidence because these were Clippy findings, not a demonstrated cargo-test
compiler failure. It is not receiving-union/current-source acceptance.


## Announced gate54 receiving-union join

Merged2750243693543a8469477a3586eda472423079c3 as8c7dd894f4ec35067e924ff46f16d7edb692779b.
Its reviewed MCP ToolId::Browser wrapper remains the single canonical tool-ID
seam; W14 adds no second name table. Twelve conflicts were inspected. Eight
browser value files retain W14's later bounded input, modifier set, human input/
watch, same-document result and strict string-only preference changes: the
incoming differences were the older foundation without unique new API behavior.
BrowserId's corrected RFC variant predicate remains byte-identical. Shared
file/skills calls, samples and ordered expectations are retained, with W14's four
pending rows last, after skills.import_selection. Both skills and browser
channels remain explicitly denied by the legacy web bridge. No InputText::new
or EmptyBrowserInput caller remains, and no protocol number was assigned.

The incoming generated wire file is retained byte-for-byte. A staged whitespace
check against the old parent reported its already-generated trailing spaces;
the scoped own diff against the announced275 source is clean. Generated output
was not trimmed or reconstructed locally. Correct merged-head generation is
owed through the VM's fixed TS route.

The older888 functional and TS jobs were both still queued; each was cancelled
through queued-only control and terminal cancelled/143 was verified before its
one receiving-union replacement. They produced no execution evidence. This
choice supersedes the earlier plan to retain888 baseline tests: the announced
MCP/browser/skills/file union now needs one actual combined-source suite instead
of spending an extra build on the pre-carrier head. Whole gates, runtime,
Preview, renderer and supported-engine acceptance remain separate and owed.

Reference creation default rechecked at packages/app/src/desktop/browser/
automation/handler.ts:350: omitted tool new_tab URL selects https://example.com.
It does not select a private dev-server origin or expose the launcher's internal
about:blank as an invented tool default. Preview authority remains a separate
explicit opening; the real creation executor still remains owed.

## Checked generated browser wire declarations

The fixed TS run `wp-browser-ts-20261116T012549Z-3dce01dd` executed all seven
declaration/alias shape tests successfully. Its step returned 1 solely because
`web/src/wire/proto.ts` differed after generation. The recovered source commit,
artifact manifest and each SHA256 were checked before adopting that exact file;
no generated declaration was edited by hand. Subsequent proto changes before
adoption affected only a test's checked division, not a declaration or generator.
The committed output includes the pending browser calls, typed action outcomes,
wait matches, bounded upload intent and default-off settings. It enables no
method, engine, tool offering or file authority. A committed-output freshness
check remains owed on the receiving announced base.
