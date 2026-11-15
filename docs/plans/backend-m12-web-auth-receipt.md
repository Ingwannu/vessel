# Web tab-session receipt test

The shell can paint before the bridge verifies its welcome and stores the
independent tab credential. The history/reload security case incorrectly used
sidebar visibility as that receipt. Its unchanged storage assertion could
therefore race a valid asynchronous handshake.

Reuse the readiness predicate already used by openUi: a non-null tab-session
entry is written only after verified welcome. Wait for that receipt before
the existing storage assertion. Every original hash/local-storage/session
assertion, reload and replay check stays byte for byte; production auth,
retry policy and existing deadlines stay unchanged.

The bounded slow-welcome fixture relays every packet unchanged except holding
one actual client hello until the test releases it. This delays the real
server's welcome without manufacturing authentication data. It proves the
sidebar can be visible while tab storage is empty and the readiness wait is
pending, then completes after release. A five-second fail-safe prevents a
fixture hang; events and predicates, not a sleep, coordinate the test. Packet
contents and credentials are never logged. Only isolated fake-fleet UI launches
are used; no live node, account, credential file or vendor endpoint is touched.

Typechecks and changed-file lint pass. A complete byte comparison confirms
that removing the authorized import/comment/wait restores the original
security test file exactly. The bounded real-browser checks are pending
at this source checkpoint; no product authentication behavior was changed.

## Focused execution

The immutable source is `c5da5305`. Source review found no defect; the
production auth path and all original history assertions remain unchanged.
Two focused executions completed successfully, including the bundle and
both nested binary prebuilds. The delayed-welcome regression passes in
`webparity-20261115T071525Z-c5da5305`. Its line selector did not include the
original history case after the new import moved that case by one line;
this run is evidence for one new case only. The previously unselected
history case then passes by name in
`webparity-20261115T072843Z-c5da5305`, including storage, reload and replay.
Each execution runs one case and every selected step exits zero.

These are focused results, not a whole-package or combined-line approval.
The earlier page package's security failure remains in its historical log;
a new changed-tree whole gate is required after applying this correction.
