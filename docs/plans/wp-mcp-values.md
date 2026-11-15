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
