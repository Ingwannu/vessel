# Staged original-execution observation for concrete admission tests

This cut contains the already qualified invalidation producer on the announced
node line, followed by observation-only capture under literal cfg(test). It is
not a positive private queue/creation admission API or an app values carrier.

[Decision Log]
- Purpose: concrete W3 launch-capsule tests must retain the ORIGINAL capture and
  watch without importing the entire unfinished timeline runtime or cloning a
  public stamp into authority.
- Existing assets: the single Agents registry owns incarnation/current grant;
  its qualified invalidation producer owns the latest-change watch. Existing
  private Sessions fixtures provide real fake SessionHandles and named cleanup.
- Choice: reuse the exact capture and fixture source blobs from the common owner,
  with cfg(test) reexports only. No second watcher, registry or authority token.
- Consequence: observation can report revocation/closure but cannot authorize any
  later effect. Actual capsule insertion must recheck under the SAME Agents
  guard; pure stamps, process lifetime and current-looking observations cannot.

`CapturedParentExecution` has private non-serde fields for the original registry,
ParentExecutionStamp, FinishGrantCeiling and existing watch receiver. Only a
current direct LOCAL Agent with matching registered non-ended handle and both
principal/current ReadOperate scopes can capture. Subscription precedes reading
under the same mutex. `invalidated(&mut self)` awaits only that original receiver;
`observation()` re-reads that same registry. Neither recaptures or enlarges a grant.

The complete capture.rs and execution_capture.rs files are byte-identical to
owner checkpoint26b69327d. The real-fake test encloses setup, observation waits,
runtime shutdown, task settlement and Store close in one named 120-second bound.
It covers all principal kinds, unregistered/ended handles, both grant ceilings,
rotation, original versus new capture, retirement and original watch closure.

No current metadata lease, concrete queue/creation capsule, grant-atomic effect,
transition call-site join, provider, Store/schema, RPC or positive notice/creation
capability is added. Existing invalidation-only scope remains distinct. Required
before even a staged receiving claim: exact source review and actual changed
proto/node strict plus fake-runtime tests. Production use additionally requires
the concrete capsule and retained settlement supervision contracts and tests.
