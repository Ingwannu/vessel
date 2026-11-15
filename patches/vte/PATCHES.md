# VTE 0.15.0 decoder metadata (ADR 0095 follow-up)

Status: staged; exact published archive retained, implementation checks owed.
Both upstream licenses and every original member checksum are retained.
This is the same transitive decoder vt100 already uses, not another parser.

1. Preserve whether each CSI scalar parameter was written or defaulted.
   Why: Kitty `CSI > u` defaults to flags one, but `CSI > 0 u` explicitly
   selects zero. Reconstructing presence from normalized zeros guesses the
   wrong keyboard protocol. Metadata follows the existing single decode pass.
2. Expose a read-only completed-token boundary predicate, including pending
   UTF-8. Why: presentation can withhold incomplete control tokens and remove
   only queries answered by the node without a separate raw scanner. No parser
   state is mutated or restored through this observation.

No runtime, PTY, toolkit, filesystem or environment dependency is introduced.
No generated terminal ANSI is ever fed back into an engine.
