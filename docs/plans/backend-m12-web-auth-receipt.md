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
