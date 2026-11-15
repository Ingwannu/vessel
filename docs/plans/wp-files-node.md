# Workspace file operation implementation

Status: implementation in progress; five canonical rows are registered pending protocol assignment.

## Existing assets and decision

Modify `vessel-workspace::files` and `paths::Root`, reuse the regular-file
open helper and SHA-256, and share the existing fuzzy scoring routine with
suggestions. Add no local-client filesystem shortcuts or shell programs.
The node service remains routing only, with authorization through existing
`Workspaces::resolve`. ADR 0083 records the method and dependency decision.

## Structure and flow

`vessel-proto::workspace::{panel_files,suggestions}` owns validated input
and opaque byte pages. `vessel-workspace::files::{binary,duplicate,observe,
suggestions}` owns disk work and its bounds. Node routing and client registration bind these types through the one
canonical method table and pending `FILE_PANELS_PROTOCOL` constant. No schema change.

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
and serde fixtures pin the chosen contract. V3 accepted these source corrections with executed scoped regressions on
exact100da283; later SDK extraction and enclosing package gates remain owed.

## Admitted SDK reader prerequisite

The existing transfer loop is reused through `fetch_workspace_file_with(root,path,reader,make_sink)`. The original `NodeClient` API delegates to it; no wire method, protocol number or filesystem authority is added. A native selected-connection reader can fence every original page without raw-client access or a second transfer implementation. Regressions pin first-page refusal before output construction and retirement between pages with the original root/path/expected revision retained and no replay. The sink still owns complete hash validation and atomic publication. New-source narrow evidence is owed; exact100da283's808/0/1 is prior-source evidence for the earlier security correction, not this extraction.


### TS prerequisite check

Exact17bcfc450 TS prerequisite Clippy reached compilation and stopped on two
owned wiring issues: the web bridge exhaustive forwarding match lacked the
five new unary variants, and a serde fixture used unwrap inside a Result helper.
The bridge now explicitly forwards those existing typed methods, retaining the
node protocol/scope checks; its original refused-method assertion is unchanged.
A package regression pins all five as pending Unary. The fixture propagates
serialization failure with `?`. Generation did not start in that run; no TS
artifact or freshness result is claimed. The earlier808/0/1 remains bounded
unit evidence, with current client reader extraction tests/strict checks owed.


### Accepted correction evidence and remaining package gates

V3 row171 accepted both returned source findings as SOURCE-SETTLED/GATE OWED:
own28f04800e..411106ad correction, executed on base-only100da283. The reviewer
independently matched the actual808/0/1 library summaries, real-file A→B→A
barrier and suggestion direct/serde fixtures; the interrupted infrastructure143
is separate. That acceptance does not cover the newer admitted-reader SDK
extraction or prove the whole package/runtime. Exactf0cc50d1 TS prerequisite
rerun is queued after the two concrete TS1 compilation/lint corrections.
Generated bytes/freshness, current changed-crate strict and reader tests, then
the enclosing root gate and integrator protocol assignment remain owed.
The SDK header now says a local timeout is not remote termination and refresh
alone does not authorize retry; no receipt or terminal-fence contract is added.


### Wire alias classification

Exactf0cc50d1 clippy-ts passed (92 seconds). The following real TS test run
compiled successfully, then correctly refused six unclassified new primitive
aliases before writing generated declarations (proto2 pass/1 fail; no valid new
artifact). The sorted existing alias table now adds FileBytes as carried
Content; FileModifiedMillis, FileOffset, FileSize and SuggestionLimit as
Quantity; SuggestionQuery as person-entered Named text, matching FindQuery.
Existing table entries and the head sorting/uniqueness assertion stay unchanged.
An owned regression pins these choices. The next generation must execute and
its exact artifact bytes must be reviewed/applied before freshness is claimed.

### Generated declarations applied from the exact VM artifact

Run wp-files-node-ts-20261115T131233Z-d9c96a5f executed on exact
d9c96a5f644171838053ff138fbf1e30b42cd51f: clippy-ts rc0 (156 seconds),
proto declaration generation tests3/0 and web bridge declaration test1/0.
The TS step rc1 means tracked proto declarations changed after successful
actual generation, not a successful freshness check. The artifact commit and
all six SHA256SUMS entries were verified before applying its exact wire.patch.
Only web/src/wire/proto.ts changes; calls.ts and bridge.ts match the artifact
and existing files. The generated proto SHA256 is
923618cdb7463c1b2e81cf2f5294765da1e2ef2b85a28ae9c06c01309755664b.
Its37 trailing-space lines are generator output, retained byte-for-byte rather
than manually formatting generated code. Freshness must rerun on the applied
source; current SDK unit/changed-crate strict and final enclosing gate remain
owed. No assigned protocol, app renderer or complete-download UI is claimed.
