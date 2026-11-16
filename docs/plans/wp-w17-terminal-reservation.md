# W17 terminal reservation prerequisite

Status: source/test preparation only, based on announceded4cd137. No new RPC,
protocol/schema number, root binding, cleanup-debt ledger or quiescence proof.
Coordinator authorized this independent preparation while immutable CLI test
run wp-cli-narrow-20261116T005236Z-136c1395 waits; one owned Cargo run at a time.
No leaf build may start until that run finishes.

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
