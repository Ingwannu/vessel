# ADR 0083: bounded workspace file panels and live source state

Status: proposed.

[Decision Log]
- Purpose and intent: supply native file panels with byte-preserving
  downloads/PDF data, duplicate, live file versions and directory discovery.
- Existing implementation and constraints: `file.read` sends only UTF-8
  text or a binary marker; `file.find` indexes files, not directories.
  Existing `Root`, regular-file open, content hashes, git ignore checks,
  and bounded blocking disk work remain the only filesystem boundary.
- Alternatives considered: use local client filesystem paths (incorrect
  for remote nodes); shell out to copy/download (argument/process surface);
  poll every file from the app (wasteful and loses disconnect ownership).
- Choice: additive typed requests under one pending feature constant,
  version assigned at integration. Binary pages carry exact offsets and a
  stable version; later pages require that version. Duplication reserves
  a new sibling exclusively and removes incomplete output on failure.
  Single-file observation waits finitely and reports ready/missing plus
  typed failures. Suggestions are bounded root-scoped discovery, preserve
  exact-path retrieval and honor git ignores and the reference hidden-dir
  allowlist. No persistent schema is needed.
- Why: reuse reviewed path and file primitives, avoid a second dispatcher,
  keep all work authorized by the existing root and principal, and make
  incomplete transfers or unsupported older peers explicit.
- Advantages, costs and effects: binary pages avoid whole-file responses;
  content version checks refuse a mixed transfer. Duplicates do not follow
  links into another tree. Resource bounds are named constants. A watcher
  belongs to the request/connection and cannot survive disconnect forever.
  Existing path checks accept same-user directory swap races (ADR 0027);
  this does not claim an OS filesystem sandbox.

Reference: Paseo 0.15.4 `server/file-explorer/service.ts` (workspace scoping,
copy naming and revision guarded reads), `file-explorer/observer.ts`
(file versions), `utils/directory-suggestions.ts` (limits and discovery).
App-facing details and renderer dependency analysis are in the companion
`docs/plans/wp-files.md`. Protocol and schema numbers are never reserved
locally. This feature uses no database migration.
