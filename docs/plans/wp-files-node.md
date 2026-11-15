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
read-pass content revision; page bytes are captured from the same buffers as its complete digest, with continuation identity checked. This does not promise an atomic snapshot of a mutable file. File reads
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

### Wiring check corrections

The first registered wiring checks reached the new handlers and SDK. They found
an omitted old-agent reply arm, a missing test-only CallError import, and manual
FakeNode handlers that needed the five new methods. Both client and RPC test
fakes now declare those methods with their existing unsupported-method macro;
no original assertion or admission expectation changes. A redundant Future
qualification in the tracked work helper was also removed. These are source
corrections; new unit tests, TS generation and the full gate are still owed.

### V3 source corrections (row 171)

The original separate before/page/after reads admitted A→B→A substitution.
`files/binary/pass` now owns one bounded SHA-256 pass and captures only the
requested page from those same buffers. The real-file barrier test preserves
the exact adversarial schedule and pins page/digest agreement. No full-source
buffer or new transfer actor is added. Query input now uses the existing JS
trim helper after explicit raw scalar/control admission; ADR0083 records why
the 256-scalar and control/line-separator restrictions differ from Paseo. Unit
and serde fixtures pin the chosen contract. These corrections still await the
proper changed-source narrow checks and V3 follow-up; no green claim is made.

## Admitted SDK reader prerequisite

The existing transfer loop is reused through `fetch_workspace_file_with(root,path,reader,make_sink)`. The original `NodeClient` API delegates to it; no wire method, protocol number or filesystem authority is added. A native selected-connection reader can fence every original page without raw-client access or a second transfer implementation. Regressions pin first-page refusal before output construction and retirement between pages with the original root/path/expected revision retained and no replay. The sink still owns complete hash validation and atomic publication. New-source narrow evidence is owed; exact100da283's808/0/1 is prior-source evidence for the earlier security correction, not this extraction.
