# Provider source identity values (A4 prerequisite)

[Decision Log]
- Purpose: let the shared keyed pane/timeline contract name a provider child without pulling A4 store/auth/RPC implementation or fabricating SessionId.
- Reuse: existing SessionRef owns node+managed-parent identity; ProviderChildId/ProviderText are unchanged from V3 scoped-passed55181bb9 (values first introducedfaa5).
- Choice: ProviderChildRef{parent:SessionRef,child:ProviderChildId}, clone/equality/hash/serde/TS; one canonical value for pane keys and read-only navigation. Root path remains vessel_proto::subagents.
- Effects: no pending method/event, wire number, migration, process, permission or authority change. Consumers still require the actual A4 protocol gate before obtaining live child data.

Provider IDs preserve exact provider spelling, are nonempty, at most512 UTF8bytes, with no control characters; descriptive text is at most64KiB with newline/tab/carriage return allowed. Existing validating serde/construction tests remain unchanged. This is an isolated base4 bridge, not proof of provider support or complete A4 parity. Exact narrow results are supplied separately.
