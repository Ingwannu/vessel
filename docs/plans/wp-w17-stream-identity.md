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
