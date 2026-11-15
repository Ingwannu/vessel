# Terminal runner completion facts

Status: isolated runtime producer for review. Registry debt, physical-root
admission and workspace member-stop orchestration are separate consumers.

## Decision Log

- Purpose and intent: expose actual runner completion separately from logical
  terminal status, so teardown cannot mistake `ended()` for physical cleanup.
- Existing assets and constraints: TerminalHandle already shares status and
  cancellation with the owning shell/script actor. A process lifetime revokes
  capabilities but does not prove descendant exit or I/O completion. Existing
  bounded output drain and process sweep do not establish all postconditions.
- Alternatives: infer cleanup from status or handle absence; add a separate
  process registry; or attach actor-owned completion to the existing handle.
- Choice: extend the existing handle with a watch receiver whose sole sender
  belongs to the actual runner wrapper. Tokio watch and the existing task
  tracker suffice; no new dependency or parallel registry is needed.
- Why: only the actual runner can report its own final cleanup. Losing its
  sender yields Abandoned rather than an invented success. Unknown facts remain
  explicit and cannot authorize physical backing mutation.
- Benefits, costs and effects: observers may await beyond logical status and
  choose their own total deadline. The report is immutable, repeatable and
  correlated with TerminalId. Spawned descendants remain unconfirmed, so this
  foundation deliberately cannot yet unblock their destructive consumers.

## Modules and data flow

`terminals/cleanup.rs` defines CleanupFact, CleanupReport and CleanupObservation.
Their public surface has no report constructor: the private Completion sender
publishes only after `actor.run().await` returns, including owned-resource drop.
TerminalHandle::cleanup observes the same original handle, without a registry
lookup. Report fields independently describe process, descendants, I/O and
auxiliary-file cleanup. is_confirmed requires all four facts.

ScriptActor retains whether any spawn was attempted across retries. Approval
refusal before every spawn can prove no child existed. A spawn error can occur
after partial child/pipe setup, so even a failed attempt remains unconfirmed.
Wait errors and incomplete output remain unconfirmed across later retries.
`terminals/script/output.rs` retains the actual pipe task and joins it after
drain. Timeout aborts and joins the task but does not claim I/O completion.
Final auxiliary-file removal is reported only after its actual result.

ShellActor currently cannot establish reliable process/descendant/I/O facts
through its legacy PTY waiter and bounded sweep. Its completed report therefore
keeps these fields Unconfirmed. A completed actor, ProcessLifetime revocation,
ended status or absent registry entry never upgrades those fields.

The node's terminal registry/debt producer owns retention across forget/prune,
launch-root binding and read-only quiescence admission. The workspace owner
owns strict member-stop policy; it must keep the exact handle on timeout,
Abandoned or any unconfirmed fact. Neither consumer may derive physical-root
authority from this report. The existing legacy close API is unchanged.

## Validation and integration boundary

The isolated cut contains only the terminal module, actor/handle changes and
tests, copied from the A2 runtime source. No session archive actor policy,
protocol/schema, node registry, physical-root lock or workspace effect adapter
is included. Existing screen-control scheduling belongs to its separate owner
and must be preserved when joining that consumer's source.

Narrow verification runs on the build VM:

```sh
cargo clippy -p vessel-session --all-targets -- -D warnings
cargo test -p vessel-session cleanup
```

The actual actor integration cases distinguish no-attempt cleanup, an attempted
spawn failure, failed auxiliary cleanup and observed script exit with joined
I/O but unknown descendants. Unit cases cover sender loss and preservation of each
reported fact. Previous A2 runtime execution is prior-source evidence;
this isolated cut requires its own checks and review before adoption. No full
workspace cleanup, descendant proof or production restore is claimed.

The isolated producer now merges announced node `27502436` (schema 21). Its
first strict check on older `ec824ddb` found four ignored TaskTracker close
return values in integration tests; the correction names those results without
changing runtime facts or assertions. That older run is not a passing check.
Current strict/runtime review evidence remains owed before adoption.
