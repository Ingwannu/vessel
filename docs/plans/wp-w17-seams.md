# W17 shared registration seams

Base: Quality B `29eebb8b`. These two commits change structure only; signatures,
method rows, error text, startup order and test expectations remain unchanged.

[Decision Log]
- Purpose and intent: leave room for package registration without raising a size record.
- Existing implementation and constraints: rpc.rs is near 500 product lines; node.rs has a shrinking cap. New feature code does not belong in either assembly file.
- Alternatives: raise caps, duplicate registration, or extract existing independent responsibilities.
- Choice: move Request/UnexpectedReply contracts to rpc/contract.rs, then existing model-client/listener setup to node/setup.rs in a separate commit.
- Why: public reexports retain the same API and one method table remains the sole registration source. Startup calls retain their order and awaited results.
- Effects: fewer assembly lines, no new dependency or behavior; size records only shrink.

Fast validation: rustfmt, size and crate-boundary scripts. Narrow root clippy on the
exact second commit checks the unchanged expansion and startup types; no full package gate.
