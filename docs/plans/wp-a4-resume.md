# W2 detach execution resume — quota warning checkpoint

Current product sourcefe0ad411ea4a77049030e3137d631aa433e862e4 is frozen for
V3 row310. The final source is docs-only after tested3cf77901a9e15f44550a7ede776dedf7fb1eb10a,
on announcedd5bfde878598da21fd9ac1abfcb130571c9c1748. No new merge, capture API,
control, VM run or whole gate should start while the quota warning is active.

The actual owned detach task holds the same marking lock, reads committed
Parentage, refuses Root/corruption before invalidation, invalidates Attached with
ExecutionInvalidation::Detach before the first store write, and skips rotation
for a valid repeated detach. Writer-atomic receipt and tracked publication stay
intact; no stop, identity, workspace, credential or history replacement occurs.

W23 explicitly delegated and read the exact31-line diagnostic observer plus its
two-line cfg(test) re-export at1ff54e856. The wrapper only subscribes to the existing
execution watch under its registry lock; current/has_changed/mark_seen carry no
capture, grant or serialized authority and add no channel/registry. Keep ONE
canonical wrapper. GKS lifecycle waits for this same reviewed receiving producer
for stop/resume/archive order tests; do not duplicate or import an unqualified
moving branch. Its actor/caller joins remain GKS-owned.

Exact corrected3cf passed Node --locked all-target strict0 in ordinary run
wp-transfer-detach-execution-20261116T023758Z-3cf77901. Actual Node detach filter
6/0/0 passed in the immutable actual run
wp-transfer-detach-execution-20261116T023941Z-3cf77901. Use that exact receipt.
All web/binary preparation and cargo steps passed; Node compiled freshly.
The two new tests prove successful first/repeat/root/no-entry behavior and both
sides of the held marking boundary with a writer-only failure (reads remain
available). All four old publication/ancestor/resume regressions also passed.
The first1ff strict qualification diagnostic remains recorded as a failure.
The corrected test owner retains children before fallible persistence and joins
cleanup before resuming scenario failure. Normal cleanup executed; an induced
failed-cleanup scenario is not claimed.

Unverified: V3 final bounded source acceptance and integrator receiving/announcement,
positive capture/atomic queue admission, real provider/other OS and full A4.
No VM request remains active for this leaf; no fixture process remains running.
Next: read V3 row310's coherent verdict, correct only a requested finding if any,
then hand off the exact qualified leaf to the integrator. Tell GKS/W23 only after
the receiving head is announced. Do not qualify a new capture API or process
liveness as admission. The later announcedee8 head was not imported here while
this review candidate was immutable.
