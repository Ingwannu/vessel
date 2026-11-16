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


## Feedback: modal ownership and toast clearance

Paseo's `hooks/use-keyboard-shortcuts.ts:315–346` gives the top overlay
first refusal before app shortcuts; `lib/overlay-root.ts:175–224` manages
focus/IME/keys for that painted layer. Vessel retains Base UI for those
behaviors and adds a small guard at background producers. Its public popup
role and open attribute are checked at the time of a page focus request or
a recorder key, rather than caching focus at mount. A second overlay stack
or Escape listener was rejected: it would compete with the kit's nested
controls and composition handling. The draft's mounted lifetime remains
independent of its optional focus; the recorder pauses while a dialog or menu is
open and can resume its pending choice after that overlay closes. A menu
uses the same public open-role boundary: waiting only for dialogs would
let a late lazy route move focus out of the Display menu. A controlled
browser test first observes keyboard focus inside that real menu, releases
the held route asset, then checks that focus remains there. Tooltips are
not keyboard-owning menus and are not part of this guard.

Paseo's `components/toast-host.tsx` places the app-shell viewport under the
header, centred, including on narrow screens. Vessel uses its own safe
inset/header/spacing tokens, retaining its 320px wide card. On narrow
screens the reference first insets its host 16px at each side, then applies
a 92% boundary capped at 480px. The card therefore measures
`min(0.92 * (viewport width - 32px), 480px)`: 329.36px at a 390px
viewport. Applying 92% directly to the viewport was an incorrect port. Positioning at
the top across sizes avoids moving the same overlap to a wider composer.
Existing success-four-seconds/error-until-dismissed and multi-toast semantics
are deliberately retained here rather than changing unrelated notification
lifetimes. Unlike the reference's single active message, Vessel bounds its
existing stack to a scrolling quarter-viewport, so accumulated feedback does
not keep extending down the page. This is a layout rule; it does not claim
that every notification or OS/device behavior has parity.

The scroll host itself must clear the actual header, not just its first
unpanned card. Shadow padding can make a normal card look clear while
scrolling reveals content inside the header. A browser regression checks
the host's top edge as well as its lower boundary; the existing larger
spacing token provides that clearance without a second positioning model.

A viewport fraction alone is insufficient when a question grows into a
short phone screen. The existing prompt dock is the boundary for the whole
composer, including queued prompts, permissions and questions. The toast
host therefore caps its viewport by that dock's actual top edge as well as
the quarter-viewport limit. Browser-native resize observations follow
folding and input growth; a filtered mount observer rebinds only when a
prompt/menu enters or leaves. Listeners are owned by the visible host and
released with its last message. The timeline observer is intentionally not
reused: it owns reading intent and row anchors, which feedback must not
change. No timer polling or second notification store is introduced.


The feedback correction's executed source is `dac496599`. Its whole gate
passes 2,307 Rust tests (twelve ignored), four TypeScript freshness checks,
464 web units, 79 browser cases and four forge fixtures. This is execution
evidence, not review or integration approval. The retained 32-image matrix
belongs to the earlier unchanged toast renderer `7e3418d8` and its caption-
only capture harness `37a723ba`; later menu ownership is exercised by the
real-browser regressions. Earlier failures and the precise capture/provenance
limits remain documented in the plan.

The inset-width correction's executed source is `811c18d53`. Its whole-web
run passes lint, both typechecks, 464 units, fonts and the fresh bundle/binary
builds; E2E records 78 passed and one timeout before a Usage interaction.
The retained DOM reports an actual identity-verification refusal. One
explicitly authorized same-source replay passes that case (7.0s; 26.3s
including its fixture). The original failure remains recorded as a named
intermittent, with an open source/receipt-path investigation; it is not
relabelled as a wholly clean run. The preceding exact Rust result is retained
only because every Rust/manifests/toolchain/script byte is unchanged.

Eight affected capture cases pass, producing sixteen newly inspected
originals at 390x844/390x600 in both themes and browser locales. These use
verified same-commit managed CLI/node/fake exports, without a local Rust or
bundle build. The selected source and binary identities match before/after.
This replaces the earlier narrow-width visual claim only; the plan retains
the exact execution, provenance limits and next investigation.


## F3 account usage presentation boundary

[Decision Log]
- Purpose: prepare the account quota cards and their sidebar/session views
  without equating account quota with historical tokens or context occupancy.
- Existing assets: reuse Fleet, Outcome, Store, Settings and the existing kit.
  The ledger's `data/usage/facts.ts` expects integer token counters and reports
  per-field coverage. The context meter owns context occupancy. Neither is an
  account balance formatter: provider balances can be fractional and carry
  separate units, while a quota has reset and projected exhaustion facts.
- Alternatives: broadening the ledger reader would conflate historical profile
  aggregation with a current account binding. Reusing its integer formatter
  would reject legitimate fractional balances. Adding a client pricing library
  would introduce estimates and dependencies that neither contract needs.
- Choice: small pure modules under `data/account-usage` validate finite paint
  percentages, preserve independent used/remaining observations and source tone,
  format each reported amount in its own unit, and model reset, projected
  exhaustion and successful-snapshot age as separate states. They use standard
  Intl formatting and the existing dependency set, with no credential, network,
  account discovery, prices, polling, local persistence or wire declaration.
- Why: a source's own used and remaining values need not be complementary.
  Unknown and zero differ. Only a reset state carries an exact reset instant;
  an exhaustion warning must not accidentally show that instant as the time of
  exhaustion. Retained past risk is suppressed, as in the reference.
- Consequences: this is an inactive presentation skeleton. It does not register
  a screen, issue a pending call or create an account binding. The account
  report/inspect/source-consent adapter waits for its reviewed, assigned and
  merged node contract. Unsupported sources get no working-looking action.
  Agent-reported ledger/context remains independent when external lookup is Off.

Reference: pinned Paseo `usage/model.ts:8–19,209–244`, `format.ts:29–38,48–204`
and `tone.ts`. The source tone overrides inferred thresholds; the fallback is
muted below 70, warning from 70 through 90, danger strictly above 90. Amount
formatting uses USD currency, compact tokens, and two decimal places for other
units. Following the conservative guard used for ledger token counts, this account
amount formatter flags values beyond the browser's safe integer range rather
than printing trustworthy-looking exact digits. This is a stated difference
from Paseo's amount formatter; it is not a change to ledger cost formatting.

Host query data precedes a host query error in the reference, while an agent
query error hides its prior/partial list (`model.ts:114–181`). A successful query
containing a per-report failure is different: it may show the last successful
values and their original successful timestamp. The eventual reader must keep
these distinctions and its own request lifetime separate from a cached remote
Refreshing phase. None of these view states diagnoses a stopped turn or enables
automatic wait/resume or account switching.


The shared quota meter consumes the same private-constructor percentage value
as the account projection. Its implementation lives in the dependency-free
`quota-percent` module, enforced as a leaf by ESLint; the data module re-exports
the unchanged value API. Keeping that value inside data would force the kit to
import application data; keeping it inside the kit would force the reverse
edge. Duplicating the guard would make two facts out of one. This extraction
changes neither percentage behavior nor existing test expectations. The kit
performs no second validation/clamp and no controller/slider behavior.


The shared web quota meter is a read-only kit control, with 6px height, 3px
track radius, clipped overflow and a full-radius fill. It accepts only the
validated percentage value and typed tone/emphasis; zero has zero fill width.
It imports no data or wire layer and has no slider, keyboard controller or
minimum context arc. Its isolated `/kit` specimens precede any app usage.

Two missing surface roles are added as named tokens from the pinned default
Paseo themes: surface3 light/dark `#e4e4e7`/`#434645`, surface4
`#d4d4d8`/`#595b5b`. Normal muted/success/warning/danger fills use the web
page's existing `text-tertiary`, `online`, `warn`, `danger` tokens, retaining
the palette and contrast choices documented by `tokens.css`. No existing
theme role is replaced. This theme adaptation is explicit; the control's
geometry and quiet/normal semantics follow `usage/meter.tsx`.

The pure renderer regressions cover names, read-only semantics, 0/50/100,
overflow/negative clamp and quiet override. The gallery capture cases measure
actual track/fill bounds and resolved colors, and capture the original
light/dark specimens at desktop/390 widths in both browser locales. The
control must pass the reviewer and coordinator's comparison before a Usage
screen consumes it. These are control captures, not account-report fixtures.
