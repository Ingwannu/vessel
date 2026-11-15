# C3: git panel parity

Status: skeleton in progress. Source specification: Paseo 0.15.4,
`parts/p2-git.md` (all sections), feature rows 366–416. C4 follows separately.

## Decision Log

- Purpose: the three Changes/working-diff/commit-diff targets and their full
  git behaviour share one typed state and one node observation owner.
- Existing assets: retain `workspace::git` action preparation/request keys,
  `workspace::changes`/`diff_rows`, the existing git area, workspace RPC calls,
  kit virtual list/code rows/menu/dialog/tooltip, and the node's protected git
  runner. File explorer/editor state remains its owner's; no pane manager is
  introduced. W12 supplies layout and W11 supplies registry identity.
- Alternatives: another session-bound code panel; parsing patch text in the
  app; a new diff/docking dependency. These duplicate ownership or bypass the
  typed node boundary. Standard collections and existing dependencies suffice.
- Choice: extend view/link/git area. Pure view comparison, presentation,
  demand, read/write lifecycle and document state; typed link commands/results;
  hidden staged screen until real calls work. Node additions are ADR 0087 in
  a separate node checkout. No user-visible placeholder or false empty state.
- Effects: root I/O correlation uses NodeId + WorkspaceRoot; persistence uses
  W11 WorkspaceId and W12 TabId only. The current selected session is not a
  workspace identity. B0 is merged only after its announced green base.

## Reference and defaults

- `git/working-diff-comparison/state.ts`: default All, workspace identity key,
  non-persisted override. Uncommitted = HEAD vs worktree; Committed = merge-base
  vs HEAD; All = merge-base vs worktree. Legacy Vessel Base is All, never
  silently substituted for Committed.
- `panels/changes/state.ts`, `hooks/use-changes-preferences/storage.ts`:
  unified, no wrap, show whitespace, tree hidden, separate diff, commits closed.
  Presentation uses desktop capabilities for GPUI desktop, rather than Paseo's
  React Native platform flag (which denotes its mobile renderer). Compact panes
  still use combined/unified output; a desktop separate diff keeps the tree.
- `git/diff-pane.tsx:1077–1180,1455–1660`: status loading/error/not-git precede
  diff loading/too-large/error/empty/data. Empty mode action is suppressed
  under ignored whitespace; committed+dirty offers Uncommitted; uncommitted
  with branch commits offers Committed. Discard is uncommitted-only, including
  both paths of a rename. No stage/unstage/amend/message field in Paseo.
- `git/commits-section/*`, `use-commits-query.ts`: lazy expanded demand, unsupported/
  idle/connecting/loading/error/loaded states; branch commits precede ten base
  history entries, base entries hidden from the section; commit file bodies
  immutable once present, binary fallback for null; five-minute cache.
- `git/diff-tree.ts`, `file-header.tsx`: directory-first code-unit order,
  compressed single-child folders, subtree totals, filename has width priority.
- `git/diff-document/*`: virtualized rows and lazy measurement, unified/split,
  wrap, collapse, typed anchors, selection/copy, binary/too-large status rows.
- `hooks/use-branch-switcher.ts`: stash-and-switch then offer restore for the
  target branch; existing node stashes lack ownership/branch metadata.
- `git/policy.ts`, `actions-store.ts`, `use-actions.tsx`: primary/secondary order,
  unavailable Info toast 3200 ms, pending then success 1000 ms, per-root dedupe,
  ship default PR, archive promotion, generated commit message and add-all.
- Server read/cache/watch limits: diff debounce 150 ms; LRU 64; fresh 15 s,
  suppression 2 s; snapshot concurrency 4, observation setup 2; recovery
  30–300 s; degraded polling 5–60 s; background fetch 180 s without prompting.
  Existing safety/credential floors remain stronger where needed, stated in ADR.

## Structure and node gaps

`view::workspace::git::panel` holds target/comparison/preferences/read projection,
commits/tree/document state. `link::git_panel` holds typed commands/results and
finite deadlines. `areas::git::panel` draws through existing kit controls and
W12's registry. ADR 0087 owns committed-only queries, merge to/from base,
atomic generated commit, status/diff observation/cache, classified history and
owned branch-stash metadata. Pending calls stay unregistered until Quality B.
No second transport, WorkspaceId, layout tree, file editor or attachment format.

## Package verification / resume

Per-commit narrow checks only; one whole native gate and one package review at
completion. Node work receives its root gate and V3 review; security-sensitive
boundaries retain per-commit review. Captures once at package close, using fake
nodes and authenticated owned displays. No real agent, preview or real forge.
Current: view comparison/options/body/history projection, diff-only tree and
correlated read store exist. The tree shares its directory-first order with the
document, retains duplicate source order and deep compressed identities, and
keeps absent line counts unknown. It uses an iterative arena for bounded deep
paths; UTF-16 code-unit comparison preserves the reference's non-ASCII order.
`diff-tree.test.ts`, `diff-order.test.ts`, `use-commits-query.test.ts`, comparison/
presentation/preferences reference tests supply cases. No file explorer changes.

The read store reuses `Reading`/`RequestKeys`; mode/base/whitespace/repository
changes reject old tickets. A refreshed summary invalidates file bodies even
when counts match, until node revisions can prove content equality. Unsent
reads settle via the same failure transition. Request issuers must live above
disposable panes. No registration, runtime screen or complete skeleton claim yet.

The first local view clippy stopped on missing variant docs/markup; fixed in
the current source. The next remote narrow check found a nested-reference test
comparison and two markup/duration lints; corrected without changing runtime
behaviour. Checks of the corrected source are pending.
Node comparison staging has separate protocol/workspace clippy and library
tests green (382 passed, 0 failed, 1 ignored); no new node method admitted.
The announced app base is merged (8d5513e4); the node checkout separately merged
the Quality B base. The post-merge narrow submissions ended before receiving
run identifiers, with no step logs, so their result is unknown. Their server
state must be checked before resubmitting under the same labels.

`git-panel.ftl` in both languages and its typed words adapter now stage the
new surface's labels/recoveries. Labels follow `i18n/resources/{en,ko}.ts`
workspace.git.diff / panels.diff / diffViewer; error recoveries deliberately
use complete app sentences rather than raw git text. Unsupported history stays
hidden as `commits-section.tsx` does; unsupported reads never imply NotGit.
The words are unverified pending a fresh changed-tree narrow check.
Next: complete typed link/screen/words skeleton, node All private index,
classified history, observation/cache and the write policy. No C3 row is claimed
built or passed. Existing bounded Git/PR approval is not package parity.
