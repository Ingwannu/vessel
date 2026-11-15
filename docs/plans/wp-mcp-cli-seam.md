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
