# Portable standalone timeline recovery and presentation values

This cut is based on announced node ed4cd137 (which includes the approved
27502436 base). It contains no app ancestry or moving runtime branch.

[Decision Log]
- Purpose: give W18/W3 one canonical original-request/receipt/binding vocabulary,
  and W4 one canonical approval/action/question-policy vocabulary for typed fakes.
- Existing constraint: adopting extended request/event fields also requires runtime
  compatibility and consumer joins that are not ready. Shadow enums/opaque JSON
  or a second receipt owner would hide that missing work.
- Choice: reuse complete canonical value modules byte-for-byte after separating
  their request-dependent attachment helpers and command-event tests on the owner
  branch. No declaration meaning or assertion is weakened.
- Tradeoff: these values let private state and presentation code compile, but do
  not grant any admission, provider capability or executable call.

## Exact scope and limits

Added values: SubmitPrompt and retained replies, actual prompt dispositions,
native command intent/status/parser and bounds, read-only observation/binding,
parent execution correlation/ceiling/causes, approval presentation/actions and
question policy/lifetime/delivery vocabulary. The existing adopted body, client
message identity and notice/admission budgets remain their sole owners.

No RPC rows/Request impl/SDK call, event variant/field/status attachment, handler,
guard/agent policy, compatibility traversal, constructor adaptation, Store/schema,
actor/watcher/provider/client runtime, dependency, generated wire or app is changed.
TIMELINE_PARITY stays the one None constant. These are public correlation/display
data, never a runtime execution/grant/parentage fence. Native command parsing is
not a live capability. Accepted/binding pairings and finite reservations remain
validated by the actual receiving owner; strings/Vecs are not producer admission.

Required before consumption: V3 source review; exact proto+TS strict and pure
value tests; actual root vesseld build; integrator checked adoption. No whole
unchanged worker root gate substitutes for or is inferred from those conditions.
Full rich fields/ACK/receipts/notice/creation/native transport and UI activation
remain held. Moved command-echo event tests stay on the full owner branch and
cannot be claimed from this values-only test set.

## Complete source-blob manifest

Source owner checkpoint: 84cde8e58, with the unused-qualification correction
shared with the owner branch. Each complete file below is copied unchanged,
including its remaining pure tests. Export/UUID wiring is receiving-only.

| Path | SHA-256 |
| --- | --- |
| crates/vessel-proto/src/native_commands/decode.rs | 793d7c5f547ad79700bf2c319549b096aaa7abe8d4b31d241d94059ef6bc1f94 |
| crates/vessel-proto/src/native_commands/mod.rs | 2d0439eaf46550eb73e4d755d4f3221606230bc760cfa8372fc3cce672f34cec |
| crates/vessel-proto/src/native_commands/objective.rs | 45558e52ad5f651bc4483bedba5bc6822c2555a7e407c52d0fe90979ad4e8639 |
| crates/vessel-proto/src/native_commands/parse.rs | 03c0e3946c789568591416fba60c10e20dc800d484406b8202ad366a9a0be024 |
| crates/vessel-proto/src/native_commands/tests.rs | e309993b2861f76d358fa3c13687765d4e8935a58f189d887674a437021103f1 |
| crates/vessel-proto/src/timeline/execution.rs | 7656eac7ed74b429636105ca4c91a28774608089127e3feeb555656c2d0d91ae |
| crates/vessel-proto/src/timeline/permission.rs | 47a249f07537b892b7b028cc4a8a3754809d2282dc143d55ccbd26a38444f9d0 |
| crates/vessel-proto/src/timeline/question.rs | 17c20914b254df586924952fe517dfeae9d1b376a6f74944b116711f35b716d7 |
| crates/vessel-proto/src/timeline/receipt.rs | 8432cab0b02158dc268ce7453d8a7036149e88e6c0dd59995e0c1d963e36dca8 |
| crates/vessel-proto/src/timeline/recovery.rs | f6afabee429f8970ee60de1f1e32a9f1e589a96b78ecced8df35d5c7ee8811d8 |
| crates/vessel-proto/src/timeline/submission.rs | 009ad10f482045187534071f3def7dda7e25d57675393df778cc8db0ceb44c7c |
