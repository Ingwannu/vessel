# Browser package: reference, integration and evidence

Status: contract reviewed; staged identities/native models only, no engine or
qualification claim.
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

## Native view skeleton

The pure browser module stages reference device choices and navigation
eligibility without inventing pane/WorkspaceId types or node RPCs. Fixed
menu dimensions follow the reference once; a custom agent resize will use
the wire's validated dimensions, not a nearest-preset fallback. Toolbar
buttons and keyboard routing consume one history/load eligibility rule.
Nothing is registered or visible until a real frame/input path exists.


## Runtime diagnostic scope

ADR 0078 records the measured unified-Chrome switch correction. Sandbox-on
startup and a synthetic page/PNG now work; force-prefixed WebRTC handling
failed the UDP canary. The Chrome-specific switch completed ICE with zero
IPv4 direct TCP/UDP packets after successful canary positive controls.
The Preferences-only run failed navigation and proves no traffic rule.
This is not the product proxy or a supported/qualified browser. Runtime
fingerprint changes must invalidate qualification. Full IPv6, QUIC,
WebTransport, profile isolation, service revocation and Windows proofs are
still owed, alongside all node/app gates and native captures.


## Native frame input projection

[Decision Log]
- Reuse the existing finite-number type and fixed device dimensions. Terminal
  cell rounding is unsuitable for browser CSS/subpixel input.
- A pure SurfaceMap distinguishes physical raster, CSS viewport and logical
  pane geometry. It preserves aspect ratio and rejects letterbox/right/bottom
  border clicks instead of clamping them onto a page element.
- The panel owner must supply the CSS dimensions and revision belonging to
  the displayed frame. This geometry creates neither input authority nor a
  fake painted page. The node rechecks target ownership/current viewport/revision.
- Tests cover a doubled-density frame, centered fixed viewport, compact pane
  scaling and invalid/empty geometry. No independent pane/layout is introduced.


## Shared pane producer adoption and unavailable owner renderer

Merged the advancing app line d6aac578d, then the exact canonical values e46cd57b0
and only W12's portable producer9480d4d0 (local cherry-pick c9a002e66). The held
1720 producer and whole W12 branch were not adopted. W12 is the sole layout and
resident/ticket owner. Browser slots now register the browser area's renderer in
that one exhaustive registry. With no qualified engine/typed frame stream this
renderer shows the localized NoEngine state; it does not expose a creation entry
or substitute a page. The existing lifecycle guard continues to refuse closing
a live resource without owner settlement; there is no fabricated close ACK.
Actual resource state, stream/control and keyed close handler remain owed.


## Announced package base6 merge

Merged18eedad4 after the previous narrow run ended; no whole-gate freeze was
active. Its reviewed shared pane successor wins the nine producer add/content
conflicts, preserving W14's sole Browser renderer arm in the registry:
- docs/plans/wp-layout.md: canonical shared plan and successor evidence.
- areas/layout/actions.rs: canonical contextual action/close handling.
- areas/layout/draw.rs: canonical visible/max/compact/sheet behavior.
- areas/layout/header.rs: canonical shared tab header/action capture.
- areas/layout/mod.rs: canonical shared pane area and query module ownership.
- areas/layout/registry.rs: canonical visibility guard plus W14 Browser arm.
- areas/layout/rename.rs: canonical owner/ticket rename behavior.
- link/machines/native/tests/mod.rs: canonical shared regression inventory.
- view/layout/mod.rs: canonical LayoutActionContext/query value exports.
The first seven abbreviated native area paths are under spikes/native/app/src;
the last two are under spikes/native. No new independent panes/focus listener
or W19 staged preferences consumer is enabled. Base6's lock is announced stale;
await the lead's lock correction before any locked build. Changed-union native
verification is owed; previous376674c93 view/app strict0 proves its prior source.


## Current announced app union

After prior native f05b1051 view/app strict0 ended, merged announced base7,
base8 (91407173), then base8b c3429952 via the official app line. The latest merge
is2e4cb7046, clean, with no whole-gate freeze active. These bases carry the fixed
native lock; the historical base6 stale-lock hold above no longer applies.
Root node files still arrive only through that official line, not a node-head
merge or an unreviewed values carrier. The Browser arm in W12's one registry and
the localized NoEngine renderer remain intact; no staged resident minter,
resource close acknowledgement, focus context or raw key forwarder was enabled.

Current-union native checks/captures remain owed. The prior f05 strict result
cannot qualify later base7/base8/base8b. The shared image owner now has a staged
PNG-only bounded RGBA/RenderImage extension; W14 waits for its exact checked
leaf and uses that same decoder owner, never a parallel image decoder. Canonical
browser wire values, a real owned backend and frame/input authority remain
prerequisites before any supported visible page.


Exact505405143f0cdb02684e4f085d615dde7fb43d36 app base8b union narrow check
completed in run wp-browser-native-narrow-20261115T153236Z-50540514: native-fonts0,
locked view/app all-target strict Clippy0 (229seconds), all prescribed steps0.
No functional test, whole gate, capture, visible browser or backend/runtime
qualification is inferred. Later changes in other branches are not covered.


## Staged decoded-frame geometry composition

[Decision Log]
- Purpose: a valid PNG header cannot prove the pixels that will be painted,
  and CSS input must correspond to the decoded image rather than claimed size.
- Existing assets: W6 owns the single bounded PNG decoder and CPU RenderImage
  projection; view::browser owns SurfaceMap and distinct raster/CSS units.
- Choice: modify/reuse those checked APIs in a test-only owner preparation
  module. Decode on a future background executor, compare actual raster size
  with expected metadata, then retain the CPU image and associated CSS size.
- Alternatives: another codec, renderer or inferred CSS/device scale would
  duplicate ownership or confuse units; no external dependency is needed.
- Impact: only accepted pixels can form this model's map. It mints no browser
  identity, frame revision, permission, focus or input authority. No stream,
  UI image, GPU upload, visibility or supported engine is implied. The outer
  cfg(test) is removed only by the qualified node-browser actor/panel wiring
  review, with actual frame fencing and bounded retained-image lifetime.

The bounded W6 pixel leaf40ddff88c is adopted over checked prerequisitee1a171623.
Its four Rust blobs match checked703268a01 (Kit strict0 and image tests5/0/0);
this receiving app union still owes its own compilation/tests. Announced base9
(db42d9c5) was merged cleanly before starting the next changed-source checks.
