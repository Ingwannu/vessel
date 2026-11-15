# Schedule target lines from node facts

[Decision Log]
- Purpose: match the reference row's single target line and optional agent mark.
- Existing assets: typed Card details, ProjectInfo and WorktreeInfo already carry
  titles, identities and remote paths; the schedule state is authoritative.
- Alternatives: render raw ids, infer target deletion from a missing cache, or
  join a Windows node path using the viewer platform's path implementation.
- Choice: a borrowed pure `schedules::target::Target` contains a typed Line and
  optional HarnessRef. Missing facts have explicit unavailable variants. Unix
  home shortening operates on display text only and never opens a path.
- Why and impact: one renderer can choose body/path type without deciding target
  identity. No extra network request, dependency or local filesystem access.

Reference: Paseo `components/schedules/schedule-derivation.ts` and its tests,
`schedule-project-targets.ts` and its tests, and `utils/shorten-path.ts`.
The cases preserve matched project names, unmatched home shortening, unnamed
sessions and missing-provider behavior. Extra contract cases preserve explicit
project relative directories, unknown target variants, stale session identities,
missing worktree facts, Windows paths and UNC paths. The complete results are
compared, not debug names or string prefixes.

Necessary differences: Vessel calls its conversational target a chat, uses the
node's actual target-gone state rather than inferring it from a cache, and keeps
an explicit unavailable line when an identity cannot be resolved. It retains
node-authored relative directories rather than guessing a native joined path.
These pure tests do not prove any screen or official mark first-use rendering.

Narrow verification on the integrated working layout: changed crates' all-target
Clippy passed; all 877 View tests passed, including five target resolver cases.
No whole native gate was rerun for this small prerequisite under A14.
