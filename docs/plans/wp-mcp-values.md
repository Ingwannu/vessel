# Canonical browser and MCP policy values

## Decision Log

- Purpose: let browser and native consumers share the same closed tool identifiers
  without importing the unfinished MCP node runtime.
- Existing assets and constraints: W14 owns the browser catalogue; W18 owns the
  policy wrapper. Both already compile in the MCP package, but neither a source
  review nor a narrow check of that package makes its runtime consumer-ready.
- Alternatives: copying browser names into consumers would create a second source
  of truth; importing the whole MCP branch would import unrelated runtime work.
- Choice: extract only portable `vessel-proto` values on the announced node base.
  Existing serde and validation code suffice; no external dependency is added.
- Effect: `ToolId::Browser(BrowserTool)` delegates parsing and naming to W14's
  catalogue. Exact provider policies preserve unknown stored identifiers but
  refuse introducing unknown identifiers through edits. Values do no I/O.
- Tradeoff: the full MCP package must later reconcile its module exports with this
  extraction. No policy, browser engine, launch injection or voice context becomes
  active merely because a type is available.

## Provenance and boundary

The 16 browser files are byte-identical to d6898cb5, except `identity.rs`, which
is byte-identical to the reviewed identity correction a833243c. The MCP policy,
identifier, bounded decoder, voice-context and permission projection files are
byte-identical to 08ab82027. Browser tests stay beside the values they validate.
The six MCP policy/catalogue tests are unchanged from that source. The seventh,
`exact_turn_tracking_stays_pending_across_fleet_hops`, belongs to the receipt RPC
package and is retained there; this extraction contains no receipt types or rows.

`browser` owns bounded browser command/result values and one tool catalogue.
`mcp` owns the policy interpretation of those identifiers plus core tool names.
Their only dependency direction is `mcp` to `browser`. Both use existing protocol
identities and serde. The original pure slice added no method/client/node/wire changes. The approved
Skills extension below adds only staged rows, SDK wrappers, denial projections
and their generated declarations; there is still no runtime handler, settings
storage, dependency or native source change.
`BROWSER_PROTOCOL` and `MCP_PARITY` remain `None`; no protocol/schema is assigned.
The transient `VoiceTools` value is not serialized and supplies no runtime
admission or lease. Settings calls, receipt calls and engines stay out; the approved Skills
transport-only extension below is part of the final carrier.

## Validation and handoff

Run strict all-target proto Clippy with the existing `ts` feature, then the proto
unit suite on the immutable extraction commit through the remote build runner.
Size and formatting checks precede submission. These checks are still owed on
this extraction. Earlier full-package checks are provenance evidence only.
V3 source disposition and the integrator's checked landing are required before
publishing this as a consumer contract. No whole MCP/browser readiness is claimed.

## Approved Skills carrier amendment

The coordinator approved extending this carrier with the already-owned Skills
DTOs, portable name type and tests, five client wrappers, and five `staged`
method rows. This is a transport contract for the guarded native Skills consumer;
there is no skill runtime, second store or `NodeSettings.mcp` field here. Native
Tools read/patch remains Unsupported until its actual node contract lands.

Each Skills row requires Admin, is nonforwardable, remains unnumbered, and uses
the existing staged default Unsupported handler. The only node change is one
explicit deny entry per method in the canonical workspace agent policy. This
keeps local and delegated agents outside host skill administration. Exhaustive
protocol projections and fictional request samples name the new variants; no
projection enables their execution. Wiring later changes these exact rows from
`staged` to `pending` together with their real handlers before number assignment.

The original skill admission test retains every assertion and additionally pins
`is_staged()` on all five rows. It covers Admin/nonforwardable/None properties,
full call round trips and nested-envelope admission. Existing staged-handler
regressions enforce Unsupported and forbid assigning a staged row a number.
Strict checks cover proto, client and the node that consumes the canonical denial
list. The proto/client unit suites and existing node policy tests remain owed on
this amendment; earlier extraction checks are provenance only.

The client regression uses the real in-memory framed handshake and all five SDK
wrappers. Every wrapper must return its exact UnassignedProtocol method. A later
accepted GetNode request is a writer barrier: the peer must receive it first at
request id zero. This verifies absent frames and absent id allocation, rather
than inferring either from an early empty-channel observation. It touches no
real node, home, service or credentials. Execution remains owed.

V3 row227 returned one A7 issue in the SDK fixture: its ten-second overall
budget was a call-site literal. `SDK_FIXTURE_DEADLINE` now owns that unchanged
value beside the fixture. Handshake, five refusal assertions and the accepted
request writer barrier are unchanged. Exact corrected strict/unit execution and
generated-wire freshness remain owed; the source review is not runtime proof.


The first wire-generation run stopped at compile time in the existing web
bridge's exhaustive method classification: its match did not yet name the five
new Skills variants. No TypeScript test ran or new wire bytes were generated in
that failed run. The carrier now explicitly refuses those five methods at this
bridge and extends its pinned refusal set; node policy remains exactly the same
five deny entries. This adapter enables no route or runtime. The real wiring
package must separately review its intended web admission with actual handlers.
Exact four-crate strict (including web), regenerated wire and freshness remain
owed; the original three-crate strict result is retained with its precise scope.


## Executed carrier evidence and central TS correction

V3 row227 source acceptance extends through `4e7c0d39` (including the exact
five web FALSE classifications). Run
`wp-mcp-values-narrow-20261115T150907Z-4e7c0d39` passed four-crate all-target
strict checking with proto TS: proto, client, node and web, 159 seconds. Run
`wp-mcp-values-20261115T141924Z-bb50f99f` passed 679 unit tests: client 80,
node 253, proto 346, none failed or ignored. It executed the real SDK memory
handshake/refusal/id-zero writer-barrier regression. The later web-only
classification change leaves those proto/client/node sources unchanged.

The corrected web compile then reached TS generation in
`wp-mcp-values-ts-20261115T150907Z-4e7c0d39`; the remaining failure was the
inherited stale `LimitPercent` primitive-alias entry. Both the wrapper and
classification were unchanged from the announced base. The coordinator required
the integrator's exact central fix `9dfdf0a1`, cherry-picked with provenance;
no local alternative was written. This removes only that stale entry, not the
wire type. One corrected TS run,
`wp-mcp-values-ts-20261115T155003Z-a1245f8e`, is queued. Generated-byte adoption,
current-source freshness, receiving union and announced integration remain owed.
Failed logs are retained and no earlier subset is called a whole-package pass.
