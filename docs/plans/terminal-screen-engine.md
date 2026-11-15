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
