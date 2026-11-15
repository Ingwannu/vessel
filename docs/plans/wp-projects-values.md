# Reviewed registry values for provisional native compilation

[Decision Log]
- Purpose: the native projection needs the sole canonical registry types while the combined node head is still gating.
- Existing assets: V3 row213 scoped PASS through0140ac0a, already integrated on the node candidate; app and node share d432ef0f and reviewed plugin values e46cd57b.
- Alternatives: duplicate view DTOs or import a whole unannounced node head into the app.
- Choice: one exact registry-values delta on the shared d432+e46 ancestry. It contains proto values, six pending method declarations/classifications, explicit client/RPC fake classifications, exact generated wire types and the owner-agreed parser constant alias. No executable node/store/service/actor/native implementation is included.
- Reason: every type/validator and registry addition is the same source reviewed in row213. Both common-base parents are already ancestors of the current app line, so merging this producer cannot import the later ede node runtime through ancestry.
- Impact: PROJECT_WORKSPACE_REGISTRY remains None. These values permit compilation and in-memory tests, not real registry RPC dispatch, provisioning, restoration, generation or resource authority. This extraction is NOT a standalone node-runtime build head: the corresponding handlers exist in the separately reviewed full dependency.

The new registry module files and generated wire are exact blobs from0140ac0a;
existing-file additions are the exact registry-only ede..0140 delta. The parser
public MAX_BYTES name is a value re-export only, preserving64KiB and every check.
No MAX_COMMANDS, dependency, protocol number, store schema or runtime changes.
Generated TS is retained byte-for-byte, including generator whitespace.

The source subset is prepared, not yet an approved provisional app adoption.
It needs the reviewer/integrator's exact values-only disposition under the
published exception. No app tree has merged it, and no compile/test/whole gate
is claimed for this new extraction. Original full dependency evidence remains
scoped:105 whole2402/2/12 with two named accepted base timing defects;
258 two-crate all-target425/0/1;0140 is documentation-only.
