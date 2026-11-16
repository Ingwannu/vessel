# CLI consumer of stream identity refusal

This isolated source leaf depends on W21 SDK source66bf17e41 and its node-test
follow-up1eac1fe06, based on announced82ae7c91. Neither the SDK nor this leaf is
claimed checked/reviewed/consumer-ready here; main W17 does not adopt the SDK yet.

Reuse the existing exhaustive attach notice match for the actual unit variant
StreamError::UnverifiedIdentity. It is displayed like the other final stream
refusals; retry/stream retirement policy stays in the SDK. No alternate target,
credential, source error, ordinary RPC, parser or output contract is introduced.

The missing arm is a compile-time consumer connection, not a separate behavior
implementation. Strict checking belongs on the SDK plus caller union; W21 owns
the initial/resubscribe identity and existing-RPC-source regressions. No local
Cargo or service/ownership change was performed for this source leaf.

The joint SDK consumer range now merges announced node head `9c150c6e` by the
integrator's direction. The three original owned leaves are preserved; this
merge does not transfer unrelated CLI diagnostics or registry ownership.
Current union checks and V3 disposition remain required.

The current SDK range also merges announced `1c762ccf` (registry protocol 25).
The web stream consumer handles the same fatal identity refusal in its existing
Failed/Error/last arm; wire states and ordinary RPC errors remain unchanged.
Global exhaustive-consumer audit found the CLI and web matches; MCP uses generic
stream results. Native selected-follow domain adaptation remains an app-line
integration task, separate from this root-only range.
