# ADR 0075: Bring the existing web client to node feature parity

Status: accepted for the parity contract after source review, 2026-11-14.
Implementation, rendered parity and combined integration remain separately gated.

## Context

The node now exposes features absent from the browser page. A parallel
native client does not remove the browser client: ADR 0017 defines the
local bridge, and ADR 0036 retains the page until a connected replacement
proves equivalent. ADR 0035 chose the native client; ADR 0020 was never
accepted and is retained only as the historical proposal it replaced. Milestone 11's feature union and milestone
12's feature/gap rows are the scope, not a request for another visual system.

The source inventory and exact reference ranges are in
[the parity plan](../plans/backend-m12-web-parity.md). It separates current
node calls, existing page files, committed native source and remaining
work. Source presence is not a runtime/visual acceptance claim. Older
historical tables and unmerged branches cannot grant current capability.

## Existing assets and alternatives

1. Extend the existing typed bridge, `data/` decisions, feature modules,
   route/Settings registration, UI kit and tokens. This preserves one
   error/connection lifecycle, the page's controls and its test fleet.
2. Introduce another state/query/UI library. It would duplicate lifecycle
   and design without supplying missing node behavior. Rejected.
3. Replace the page with the native/Wasm experiment. A drawing experiment
   is not evidence of a connected browser client with the same behaviors,
   accessibility and lifecycle. Outside this change.
4. Add new browser-owned endpoints or credentials to fill node gaps.
   This violates the local bridge boundary. Rejected.

## Decision

Modify existing assets, with no new dependency, crate edge, wire method,
field, protocol number, native source change or visual redesign.

- Keep the existing direction: generated wire → bridge → data → features
  → routes. Domain decisions and formatting are pure and tested in data;
  components render the typed state. TypeScript strictness, exhaustive
  unions, no `any`, no assertions/non-null escapes and the kit-only rules
  remain unchanged. New features cannot grow a shared file into a new
  responsibility; split along model/call/state/render seams.
- First implement reported usage from the already assigned `usage.query`.
  Totals come from the node: context occupancy and routed calls are not
  recomputed into accounting. Preserve unknown versus zero, optional
  report coverage, overflow/retention gaps, separate cost units and cost
  redaction. Browser formatting cannot recover precision or completeness
  that the wire does not claim.
- Account limits and notices follow only after their assigned calls and
  types are merged. Durable inbox follows only after its own contract is
  merged. An absent or unassigned capability has no working-looking
  action. A feature number is assigned by integration; this page creates
  neither a sentinel nor a second version-gating mechanism.
- The first three feature families are usage/notices, inbox, then schedules.
  Schedules are a large wholly missing family whose list, cadence preview,
  timezone and action calls already exist. Split each family into a few
  hundred production lines per visible increment, with pure tests and
  actual isolated-node browser cases as detailed in the plan.
- Match the referenced behavior without copying source/assets. Keep the
  page's current frame, tokens, status colors, kit controls and font stacks.
  No purple, gradients or generic-first monospace/serif. Keep Pretendard
  before generic fallbacks. Existing reduced-motion/transition rules apply.

## Security and failure boundaries

The browser uses only the local bridge and generated typed node calls.
It reads no local vendor sign-in, secret, token file or arbitrary remote
endpoint; no new HTTP listener, helper process or external script is added.
Node credentials and vendor credentials stay at their existing owners.
The node's destination/origin scopes and ownership remain authoritative.

Read-only devices see only the fields the destination returns. Accounting
cost withheld is distinct from cost not reported. A future limits refresh
requires operate, policy changes require admin, and default-off saved-login
lookup retains ADR 0042's local-read/fixed-vendor disclosure. Merely opening
Usage never triggers a vendor probe or turns that policy on. Account-profile
None and accounting-profile None have different meanings and stay explicit.

Requests are visible-selection scoped. Changing node/period/session or
unmounting prevents a late response from replacing another identity. Work
is bounded; errors/old-peer refusals/offline/empty/loading/partial/stale
states are rendered in words, not silently replaced with zero or success.
A factual full-window notice does not prove why a turn stopped. No inert
wait/resume/account-switch action is advertised without actual recovery.

## Consequences and validation

The same typed node contracts become useful from a browser without widening
authority or creating another implementation of agent/accounting rules.
Costs are deliberate incremental route/state modules, review of every
reference behavior and platform-specific UI checks. Existing English page
labels and responsive layout are not a claim of translated or installed
phone-app parity; shared translation integration remains a separate gap.

Documentation uses fast checks only. Implementation commits run narrow
checks of their changed modules; each completed feature package retains
one complete Rust/web gate before hand-over. This avoids repeating the
whole queue for intermediate commits while preserving package-level
regression coverage. The whole run includes both testkit and daemon
prebuilds before tests, isolated owned SSD test data, identified source
inputs, actual rebuilt binaries and served-asset verification. Tests use
fake agents and synthetic vendor-shaped fixtures only; no real agent prompt, vendor account, real credential store,
live preview or production service is touched. Browser traces/video and
automatic failure screenshots remain off to keep credentials out of artifacts;
intentional content-only visual evidence follows the separate capture path.
Native, non-Linux/OS-IME, live-provider, independent security/licence and
combined-integration approval must not be inferred from a page source table
or one passed suite. No deployment or permission change is part of this ADR.


## A2a: reported agent/profile scopes

Purpose: make the ledger's existing agent/profile scopes available without
estimating totals or treating historical names as current credentials.
The wire is unchanged. A missing profile in an agent query includes every
profile; a missing profile on a reported group means no name was recorded.
It cannot be queried as a distinct unnamed group through that same field.

Reuse the existing single-flight/latest-choice reader and row/select kit.
One names-only catalogue, replaced by an accepted all-agent response,
keeps the available recorded names while a narrower query is selected.
It is bound to one machine, cleared on machine change/unmount/disconnect,
and holds no token or cost facts. Names may originate in a different
interval; the screen says they come from the last All agents report.
Its partial-coverage flag remains visible. A chosen historical scope stays
named if a later interval contains none of its reports.

Each selection is sent to usage.query and displays its node-provided total;
client summing of optional fields/costs or filtering a partial machine
report was rejected. This also keeps redaction/precision/retention rules
owned by the node. Refreshing All agents updates the catalogue. Queued
scope objects are copied, including added-agent references; semantic keys
are agent identity plus optional profile, so equality or label collisions
do not change the requested scope. The catalogue does not inspect vendor
logins or resolve a historical name against a current account.

The reference remains Paseo usage/usage-section.tsx, list.tsx and card.tsx:
separate read/error states and named source reports. Vessel's deliberate
increment is the existing Settings select for a reported ledger scope;
account-window cards and their lookup policy remain the separate limits
slice. Long scope names wrap using this feature's existing control style;
the shared select implementation is not replaced or changed.
