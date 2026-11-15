# Binary argument seam for MCP startup flags

Coordinator approved this behavior-preserving extraction on ede20014 before
W18 adds MCP startup flags. No MCP implementation or pending contract is part
of this commit. The integrator may land this seam independently.

[Decision Log]
- Purpose: keep vesseld's startup root below 500 lines while extending its CLI.
- Existing constraint: main.rs was 564 lines with its parsing declarations and
  runtime in one binary module. Increasing the size allowance is prohibited.
- Alternatives: add flags to the oversized root, or move runtime into a new
  library API. Both would widen an unrelated boundary.
- Choice: move only existing Args/Run/Command/RelayArgs/Bind and parser helpers
  into the sibling daemon_args binary module; change only parent visibility.
- Why: clap derives, doc comments, option order, defaults, aliases, range parsers
  and runtime dispatch remain the existing implementation. This introduces no
  dependency or product API and does not make binary parsing part of the node
  library.
- Effects: main.rs is 318 lines and its old size allowance is removed. The
  new parser is 255 lines. Help and parse regression tests cover all public
  entrypoints, hidden detached readiness, relay options, range failures and
  conflicting modes, without starting a node or opening a real home.

The original parser block and the moved block after stripping only pub(super)
were byte-identical before rustfmt (attestation retained in the task scratch
folder). Exact remote compilation, strict checks and test results remain owed.
No help/runtime PASS is inferred merely from source equivalence.


Integrator steering: 1cea5de1 was merged, without rebasing the existing seam.
Compared with that signal-fix base, only the original five owned parser/test/
plan/size files differ. StopSignals is still installed before relay bind and
before Node::start/readiness; both stop.stopped arms and the detached hangups
guard are preserved. The CLI-help regression accepts clap's documented source
processing that removes a final period from short descriptions (confirmed in
the pinned clap_derive4.6.7 source); command attributes/text were not changed.
Old cd25 runs remain old-head evidence; merged-head execution is still owed.

V3 source passed the extraction at1cea5de1..a2d01334e, with gates owed.
The older cd25 strict run exposed a help-test compilation defect: hide_env_values
is an Arg method, not a Command method. The test now applies the pinned API
through Command::mut_args for each rendered command, preserving the intention
not to expose real environment values. The product parser/runtime move remains
unchanged. This test fix and exact merged-head execution still require checks;
source approval is not compilation or runtime proof.
