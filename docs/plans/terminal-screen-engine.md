# Terminal screen prerequisite (C2, ADR 0095)

Base: announced node gate40 ba45a6e3; merge new node d432ef0f before checks.
The app stays on the package line and does not merge node heads. Native
keyboard work is retained separately at 9c3065231; no native edit belongs
in this prerequisite package.

Sequence: preserve exact upstream baseline; implement the pure facade and
measured projection fixes; add minimal capacity/reflow/grid access patches;
rerun all eight real-output checkpoints plus primary resize cases; run narrow
strict checks, one whole root gate and one V3 review; hand W17 the reviewed
API/hash. D1 retires its native parser only after switching to this engine.

Current source is unfinished and unverified: the initial facade/buffer access
and capacity setter exist, hidden-column/reflow changes and golden execution
are still owed. No consumer-ready head or parity pass is claimed.

The next correction candidate retains alternate-buffer right columns directly
in vt100's grid and reflows only the primary buffer, preserving the cursor's
logical run and using the original engine cells/modes. No ANSI is replayed.
The facade inherits lead-cell attributes for continuations and bounds each
capture append before allocating a row. New golden evidence includes actual
cat output across seven resize checkpoints, plus thirteen synthetic protocol
checkpoints for saved cursor, held cursor run, wide padding and colored wraps.
The original eight real-output checkpoints are unchanged. All are still
unverified until the next exact remote run. The first af29 check failed only
in the test digest formatter (sha2 0.11 array has no LowerHex); byte-wise hex
formatting preserves the independent expected values. That compiler error is
retained rather than called the expected parity regression.
