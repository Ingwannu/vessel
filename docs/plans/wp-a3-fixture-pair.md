# Two-peer extension of the canonical Scenario

Status: isolated source preparation on announced base14.1 b54abbdc. The lead
assigned the fixture-extension writer slot; existing single-peer consumers and
all selected/delivery/protocol/history/actor source remain outside this change.
No compilation, runtime or consumer acceptance is claimed yet.

[Decision Log]
- Purpose and intent: prove that retiring one original search source does not
  redirect or invalidate a still-active source, even when their NodeIds match.
- Existing assets and constraints: Scenario owns one State, Link runtime and
  framed peer; State owns canonical admission. W13/W11 already consume its
  single-peer API. The frozen actor and the selected-connection owner cannot
  be changed by this package.
- Alternatives considered: two Scenarios would allocate two independent States;
  another router or a guard constructor would bypass the exact boundary tested.
  No external crate or replacement transport is needed.
- Choice: extend the existing driver with a bounded optional second peer slot,
  using typed Primary/Secondary selection and existing framing/admission. Keep
  the original methods as Primary delegates, with one shared event queue.
- Why: both peers share actual canonical identity allocation and State dispatch,
  while request/reply/transcript/retirement remain physically peer-specific.
- Advantages, costs and effects: existing single-peer behavior stays compatible;
  the fixture now owns at most two framed peers and two pending replies. Setup,
  control, worker and close phases retain their current finite bounds. Normal
  cleanup invalidates both admissions before awaiting either close. Devices
  worker draining remains scoped to its existing hub; no generic owner-task
  cleanup, production session authority or positive A3 endpoint is inferred.

The public test-only surface adds start_pair, typed peer/node relationship and
per-peer operations. It exposes only ConnectionContext facts, typed Call/Outcome
and existing Event values. No raw State, client, sender, allocator or guard
factory is exposed. Duplicate names fail before starting a runtime or peer.

Qualification: unchanged single-peer regressions plus actual two-peer guarded
Devices replies, same-node physical isolation, selected-peer reconnection,
wrong-Link refusal and missing/duplicate peer cases. These are fixture proofs;
actual A3 Search/UI installation and metadata admission remain separate tests.


The source suite exercises both reconnection directions: Primary/SameNode and
Secondary/DifferentNode. The latter also uses the original request/reply/context/
transcript methods after replacing Secondary, requiring them to keep selecting
the unchanged Primary. No test infers a valid guard from exported context facts;
all successful facts come from the actual Devices receive boundary.
