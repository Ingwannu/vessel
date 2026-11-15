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
   local-service URLs remain an explicit security refusal per the ADR.
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
