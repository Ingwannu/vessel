# Workspace file operation implementation

Status: core implementation in progress; requests are not registered.

## Existing assets and decision

Modify `vessel-workspace::files` and `paths::Root`, reuse the regular-file
open helper and SHA-256, and share the existing fuzzy scoring routine with
suggestions. Add no local-client filesystem shortcuts or shell programs.
The node service remains routing only, with authorization through existing
`Workspaces::resolve`. ADR 0083 records the method and dependency decision.

## Structure and flow

`vessel-proto::workspace::{panel_files,suggestions}` owns validated input
and opaque byte pages. `vessel-workspace::files::{binary,duplicate,observe,
suggestions}` owns disk work and its bounds. Only later, on the announced
method-table base, node routing and client registration will bind these
types to the one pending `FILE_PANELS_PROTOCOL` constant. No schema change.

Binary input is capped at the reference's 64 MiB session-download budget;
pages at its 256 KiB stream chunk. A continuation must name the original
content revision; source changes before/during a page fail. File reads
run on the blocking pool and use the opened handle's regular-file check.

A duplicate creates the first available copy name exclusively. Partial
outputs begin private on Unix; final permissions and timestamps follow
the source. Failure removes only the destination this request reserved.
Copy budgets are named constants; links, special files and git metadata
are refused. No existing file is overwritten.

Observation watches a parent directory nonrecursively, registers before
its initial read to close the setup race, filters self-generated access
events, and uses a finite heartbeat. Watch failures are sticky, visible
errors; a deleted file is a typed state. Suggestions reuse root-checked
listings and git ignore pruning before traversal, with the reference's
hidden directory allowlist, result/default/depth and scan budgets.

## Reference

Paseo 0.15.4: `server/file-explorer/service.ts` for scoped reads and copy
names; `server/file-explorer/observer.ts` for saved-file observation;
`server/utils/directory-suggestions.ts` for discovery. Full app survey and
explicit differences are in the companion `docs/plans/wp-files.md`.

## Validation and resume point

Implemented tests: exact binary page reconstruction and mutation refusal;
copy naming, no overwrite, root refusal and nested-link cleanup; hidden
directory discovery and invalid limit refusal; atomic-replace/delete watch
without sleeps. First narrow clippy found unused results and error-binding
style issues, now corrected; the next narrow check is pending. No green
gate or runtime integration claim yet.

Next: narrow clippy/tests, add metadata for modified tree order, then
Quality B registration, bounded node wait admission, client calls and
old-peer tests. Review filesystem/security commits individually; submit
one complete node gate when the package is connected end to end.
