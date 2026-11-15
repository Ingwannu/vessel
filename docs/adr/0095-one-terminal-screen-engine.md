# ADR 0095: one terminal screen engine

Status: proposed; engine selection and implementation pending measurement.
Owner: C2, D1. Coordinator ruling: PLAN section 8, terminal screen engine.

## Decision log

- Purpose and intent: node capture, reconnect/restore, protocol queries and the
  native grid must interpret the same ordered PTY bytes once per screen owner.
- Existing implementation and constraints: native `view::terminal` has a typed
  vte decoder and retained grid (ADR 0073); W17 staged a vt100 0.18.1 adapter in
  session at `c0b9d71c`. Neither may remain as a second product interpreter.
  Session actors own PTY/channel ordering; toolkit drawing remains in kit.
- Alternatives: extract the native engine; extract the staged maintained engine;
  keep divergent node/view parsers. The last option is rejected because equal
  event bytes would then yield different capture rows, cursor and restore state.
- Selected structure: a pure root crate `vessel-terminal-screen`, with no Tokio,
  PTY, harness, filesystem, environment or toolkit dependency. It owns the sole
  decoder, screen state and row projection. Session retains actor/channel glue;
  native view retains presentation, ordered input, viewport interaction and words.
- Engine choice: compare both unchanged candidates against Paseo's exact server
  factory over the same frozen actual-program corpus. Match the required cells,
  cursor, alternate screen, history and capture rows. Prefer the maintained crate
  when equal. If neither matches, record and repair the concrete differences;
  a smaller mismatch count alone is not an acceptance decision.
- Why: one lower owner gives both consumers the same semantics without making
  the native view depend on session actors or copying node parsing into the app.
- Costs and effects: migrating native facade/tests is required; the old decoder
  must be removed, with existing behavior assertions retained or a specifically
  reviewed reference correction. The crate/dependency table and ADR 0001 change
  in the manifest implementation commit, not before the edge exists.

## Measurement contract and provenance

The frozen scratch plan is `/tmp/work/m12-conv-c2/engine-compare/PLAN.md`, with
its SHA256 sidecar. The corpus records actual isolated coreutils `ls`, Git diff
and Vim PTY output, command/version, bytes, chunking and resize/checkpoint events.
Eight checkpoints cover colors, wide/CJK/combining cells, alternate screen,
scroll regions, cursor and a resize before Vim redraw. No agent is launched.

The oracle copies the exact read Paseo `server/src/terminal/headless-terminal.ts`
and `terminal-capture.ts`, using its exact installed server pins:
`@xterm/headless` 6.1.0-beta.334 and UnicodeGraphemes 0.5.0-beta.330.
This runs the factory and capture function, not the whole daemon or PTY wiring.
The browser's different xterm pin is not substituted for the server oracle.

Native uses immutable reviewed `c66357e8` terminal sources (byte-identical through
D1 base 2). W17 adapter sources and the confirmed capture accessor typing-only
correction are hashed separately. Candidates build separately: enabling vt100's
vte/std feature must not silently alter the native decoder's bounded parsing.
Raw text differences remain differences even when NFC-equivalent. Timings cover
feed/resize only, 31 trials; process high-water memory includes the measurement
harness and projection and is not an exclusive parser allocation measurement.

Preliminary native result: Git diff matches, but the corpus exposes raw combining
text/emoji presentation differences. Full candidate results, exact engine decision,
all counterexamples and their reference corrections are owed before implementation.
No core API, performance improvement or D1 package pass is claimed here.

## Consumer boundary to implement

The core exposes validated geometry/history bounds, incremental bounded feed,
resize, typed immutable cells/attributes, active and historical row inspection,
cursor/modes, capture-range projection and typed query effects. PTY output has
one ordering authority. Queries are answered by the authoritative node owner;
replayed bytes and passive native observers never duplicate replies.

Protocol capture DTOs remain in `vessel-proto`; its wire mapping does not define
another screen interpretation. W17 keeps session actor/query/final-state glue.
Native paint converts the same cell facade to the kit's single grid control.
No untyped JSON tree, toolkit color, host identity, terminal lifecycle manager,
secret or transport belongs in the core. Bounds are named once by their owner.

## Verification owed

Both candidate comparisons; relevant preexisting native semantics/regressions;
bounded split input, history/eviction, resize/restore/query/capture tests;
changed-crate narrow tests/strict lint; one whole root and native package gate;
review and inspected private-fleet D1 captures. The existing T1 PASS covers its
original scope only and does not establish these engine or integration claims.
