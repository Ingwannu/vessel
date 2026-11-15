# List failure presentation boundary

[Decision Log]
- Purpose: every local list failure and node refusal has a complete phrase in
  the selected language, while diagnostic sources remain typed and out of UI.
- Constraint: write-dispatch uncertainty differs from an unsent request. Browser
  builds have no node transport and must not invent an empty success.
- Alternatives: use CallError Display, inline app strings or the editor's
  accepted error table for a different job.
- Choice: exhaustive Failure/ErrorCode selection through the generated catalog;
  a separate NoTransport fact and non-retryable explanation for browser builds.
- Why and cost: one authored phrase per place, explicit input/Alert policy and
  stable portable cases. Transport errors themselves are kept until this edge.

All18 local-reason/wire-code cases compare complete English and Korean phrases,
retry policy and shared Alert variant. Mutation Forbidden is still consumed by
row authority feedback before this read-refusal phrase, so the accepted
read-only line is neither duplicated nor reused for denied list reads.
The new browser sentence is in schedules.ftl; accepted cadence/action/state and
editor error Rust tables remain where they were, without a second word source.

Self-review also separated mutation reply validation from node snapshot
application. Both stay exhaustive over operation and payload variants; no
own-enum wildcard or long compound settlement function remains. Existing
state/race expectations are unchanged. This is a pure-rule correction, not a
claim of product UI completion or runtime transport verification.

Narrow checks:39 schedule tests pass, zero failed,833 filtered out, including
all18 portable failure cases; all-target View Clippy passes and actually
compiles View. Formatting, size, boundaries and token checks are checked at the
commit boundary. No new UI capture or whole-head result is claimed here.
The queued A13 checkpoint is fixed to the earlier5b2b2b93 commit and does not
travel with these later working-tree edits. A later exact head needs its gate.
