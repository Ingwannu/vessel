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
identities and serde. No crate dependency, method table, client, node handler,
settings storage, generated wire declaration or native source changes here.
`BROWSER_PROTOCOL` and `MCP_PARITY` remain `None`; no protocol/schema is assigned.
The transient `VoiceTools` value is not serialized and supplies no runtime
admission or lease. Skills, settings calls, receipt calls and engines stay out.

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
