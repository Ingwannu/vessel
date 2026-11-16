# W17 terminal reservation prerequisite

Current source includes the announced gate58b base d5bfde87, merged at b5f47bd9.
It still changes only the eight prerequisite paths listed in this plan's scope:
no RPC, protocol/schema number, root binding, debt ledger or quiescence proof.
The original priority strict on88ed failed one entry-borrow lint and returned to
ordinary. Corrected9e3 strict is already queued; it remains immutable and must
not be cancelled or duplicated. Later source adds named fixture safety-net
constants and the announced base merge. Current-head strict, nine focused cases,
existing terminal/script regressions and V3 acceptance are all still owed.

Pre-start ownership check: node-line log ed4cd137..backend/m12-node contains no
registry.rs change; C2 explicitly confirmed no committed or work-in-progress
edits there and its node tree is byte-identicaled4. W3 is waiting and does not
edit the registry. W17 is the single writer for this leaf.

The existing registry owns ID admission. Its registration submodule owns the
synchronous reservation-to-handle handoff and retained refusal cleanup. The
reservation captures the expected TerminalId, registry state identity and a
private non-reusable allocation token. Identity, reservation owner and vacant entry
are checked under the same mutex; duplicate existing/starting IDs refuse before
spawn. The old numeric wrapping counter is removed. Actor internals and process
wait/reaping semantics are unchanged.

A RegistrationRejected owns an observer of an already-tracked cleanup task.
Calling into_wire observes the existing CLOSE_WAIT bound, not OS cleanup proof.
Dropping that observer does not cancel the task. Unknown/incumbent handles are
retained instead of killing a known registered terminal; no incumbent is replaced.
Both existing terminal.create and script.run paths use the reserved ID and
handle registration refusal. Successful registration remains synchronous.

Only fake TUI programs with isolated environment/storage appear in the authored
process tests. Required checks are strict vessel-node alltargets, the focused
registry tests, existing terminal/script integration tests and scoped source/V3
review, then integrator landing. The exact accepted/landed hash goes to W3; it
must not consume this unexecuted source. ADR0031 records Why and limitations.

GKS lifecycle owns agent-run proof, descendant/drain results and forgotten-agent
debt inputs. This leaf neither produces nor composes those proofs. W17's future
root-bound proof will receive those inputs alongside terminal/forgotten-terminal
debt under W9's reviewed canonical RootLease. No StartupHooks/ended-status proof.

Local static preparation: direct rustfmt, size and diff checks passed; the source
preflight reports no findings and cargo_run=false. No compilation or actual test
result is implied. The original CLI suite remains immutable on136c1395.

The authored actor cases keep the canonical60s outer vessel_testkit::within
safety net. Shorter named10s observation and15s actual tracker-drain deadlines
identify readiness/output, rejected actor/process-waiter and worker-drain failures.
Fixture drop always signals the real runner stop token before yielding, and a
dropped cleanup observer never counts as settlement. These remain unexecuted.

The tracked owner also observes the existing runner shutdown token. A lost actor
watch cannot block the node task tracker forever: after shutdown, the canonical
close budget ends in an explicit local ShutdownUnconfirmed observation, not a
cleanup certificate. Two authored pure wait-selection tests cover missing-end
shutdown and already-ended/shutdown readiness. They use the canonical outer
within and a shorter named phase deadline. No new task owner, writer, RPC, proof
producer or unknown-debt retirement API is introduced. All leaf execution is owed.

Exact88ed priority strict run wp-cli-narrow-20261116T020812Z-88ed99bd returned
Cargo101 after123s, one drop_non_drop lint on explicitly dropping OccupiedEntry
to end its borrow. No focused runtime units ran. End the entry scope through the
match result and retain the owned rejected payload outside it before unlocking.
The vacant-only policy and cleanup ownership are unchanged. Per the approval,
this failed strict step goes to ordinary; do not repeat priority on this fix.

Source-only fixture budget correction: each independent test module now names
its canonical outer SAFETY_NET once, instead of repeating the literal at every
within call. Durations, assertions, actor ownership and all product bytes are
unchanged from9e3. The already queued9e3 strict remains immutable and is not
cancelled or duplicated. This newer fixture source and the later announced-base merge still owe exact
strict and all nine focused cases; earlier/queued source is not a current-head gate.

The d5 union is clean: the node-line log frombc9 throughd5 has no registry.rs
changes. Own product code is byte-identical9e3; only the two test-module budget
constant extractions differ, plus this plan. Base-owned files, manifests, lock,
proto and session remain byte-identicald5. Source format/size/diff checks passed;
these are not combined-head compiler or execution evidence.
