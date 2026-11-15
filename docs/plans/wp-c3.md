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
  introduced. The shared layout and workspace registry supply those identities.
- Alternatives: another session-bound code panel; parsing patch text in the
  app; a new diff/docking dependency. These duplicate ownership or bypass the
  typed node boundary. Standard collections and existing dependencies suffice.
- Choice: extend view/link/git area. Pure view comparison, presentation,
  demand, read/write lifecycle and document state; typed link commands/results;
  hidden staged screen until real calls work. Node additions are ADR 0087 in
  a separate node checkout. No user-visible placeholder or false empty state.
- Effects: root I/O correlation uses NodeId + WorkspaceRoot; persistence uses
  registry WorkspaceId and layout TabId only. The current selected session is not a
  workspace identity. B0 is merged only after its announced green base.

## Reference and defaults

- `git/working-diff-comparison/state.ts`: default All, workspace identity key,
  non-persisted override. Uncommitted = HEAD vs worktree; Committed = merge-base
  vs HEAD; All = merge-base vs worktree. Legacy Vessel Base includes working changes but misses the restored-untracked
  All edge; it is substituted for neither All nor Committed.
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
the shared panel registry. ADR 0087 owns committed-only queries, merge to/from base,
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
The new Fluent build rejected missing per-key translator context comments;
every new key now names its actual place and intended meaning. This correction
has not been claimed green before the changed-tree narrow check.

The initial tree arena would allocate all prefixes of every deep path before
compressing. The next tree builds compressed groups directly, so it stores only
branching/file-bearing directories. Stable internal byte order makes a group's
first/last parent sufficient for its common prefix; visible UTF-16 order still
comes from the one tree authority. Directory conversion failures are typed,
never dropped files. A real allocation-counter case bounds eight near-maximum
paths to 512 KiB of model allocation; its execution is still pending. Existing
reference rows and source indices remain the behavioural regression cases.
The queued words check at e1adc90c reached Rust and returned one generated-doc
markup lint from a translator comment naming a Rust state; the comment now
uses ordinary language. This is not a view test pass. The compressed-tree
source requires the next changed-source check and execution of its allocation
case before any measured-memory claim.


## Latest narrow evidence

At a5352cc6, view all-target clippy passed and view library tests passed:
882 passed, 0 failed, 0 ignored. Runs
`wp-git-narrow-20261115T072022Z-a5352cc6` and
`wp-git-narrow-20261115T073212Z-a5352cc6` compiled the changed view crate.
This includes the compressed-tree allocation bound and all panel model cases;
it is no screen, node-write, package or complete skeleton approval.
The package base 2 (6036eeb8) was subsequently merged at 2b7d4138. Those
immutable remote test results are for a5352cc6, not a combined-tree gate.
Shared machine node identity and input editability come from that base.

Existing kit split-button ownership is confirmed. Its returned corrections
(one-line typed density, menu-only shape, named parameters and its own radius)
will be a control-only change within C3. The current reference compact height
is CONTROL_HEIGHTS.sm = 28, not the older note's 32; native touch keeps the
44-point target required by ADR 0036. Shared loaders remain the loader owner's
producer, with no area spinner. The current integrated menu producer is not
ready; historical source clears are not a current API handoff.

## Split-button correction (control scope)

The existing `kit::split_button` is extended, not duplicated. Current reference:
`git/actions-split-button.tsx` (whole component/styles) and
`components/ui/control-geometry.ts`: pointer26, compact28; inset8/12, gap4/8,
borderAccent1, own md radius6, body14/21. Native touch remains44 (ADR0036).
`Density` selects one-line words or just the required glyph, chosen by the
caller from available panel width. Literal label breaks are folded through the
existing one-line helper. `Content::MenuOnly` uses branch16/caret12/gap4 and
width42/48, rather than drawing an empty primary half. Hover/held/open fills
remain surface2, disabled primary remains0.6, and busy remains nonactivating.
The shared ActivityIndicator's numeric16 vector in a centered20 slot preserves
the reference's small20 indicator transformed by0.8 without a second spinner.

The one existing app caller is adapted to the typed content/density; its view
policy is not changed by this control commit. An absent primary now opens the
menu through the whole branch/caret trigger. The caret's phase/anchor are a
named struct. The old menu renderer's placement tuple is left to its shared
owner's current integration, which must preserve `menu_right`; no second
renderer is introduced here. Shared producer434933b3/docs533b21ff were consumed
as51aea1ab/2b9e079c. Only the pre-existing slider size record was tightened719
→711; current base link records were retained. Neither previous producer checks
nor style assertions approve this combined binary's interaction or pixels.
Narrow compilation/tests and final C3 first-use captures remain required.

The first split narrow check at38ea09b0 returned two source lints (one unquoted
Rust type name and a borrowed-only label passed by value). The control now
borrows its content/label naturally; these corrections do not change geometry
or action policy. No unchecked allow/expectation was added. Its changed-source
narrow check is required; no combined UI pass is claimed.


## Typed local read failures

The existing correlated lifecycle now has a generic `ReadState<T,E>` and a
source-compatible `Reading<T> = ReadState<T,Problem>` alias. It remains in the
same module so other workspace readers can reuse it instead of copying the
late-answer logic. Panel reads select a typed local failure in addition to the
unaltered node/transport Problem: selected connection retired, actual node
identity mismatch, wrong returned file path, or unavailable comparison. No
node ErrorCode is manufactured for an app admission failure. The connection
owner's exact-route/epoch preflight remains the only resolver; this adds none.

Retiring a connection settles pending state immediately and rejects old replies;
a returned file must name the requested relative path before installation.
Whole-sentence EN/KO recoveries explain local faults without interpreting git
output. Added tests exercise the production read store, including old/new
requests after retirement and duplicate wrong-file replies. Existing fixed-error
Reading callers are unchanged. These changes are not yet checked or wired to
the connection owner's still-pending actor seam.

Split source atdd6ad9c5 passed all-target App+Kit narrow clippy (run
`wp-git-narrow-20261115T075117Z-dd6ad9c5`); kit library tests are separately
running. This is no rendered first-use or package approval.

## Source-preserving diff layout

Paseo `utils/diff-layout.ts` and every case in `diff-layout.test.ts` define
replacement pairing, context duplication and review-side/source-line identity.
The new document packs the existing `workspace::diff_rows` authority into
unified or split cells; it retains the original Arc rows/text and never runs a
second numbering or text engine. A retained cell is tied to the full repository
and file-read key, so another read cannot turn its visual index into a comment
target. Switching layout within the same read preserves canonical review keys.
Keys are typed path/side/positive source-line records, not concatenated paths.
Zero cannot be serialized into a valid review line.

Vessel's wire hunks omit header-line objects, so source line indices refer to
actual hunk lines. The source number/side/path key has the reference meaning.
Vessel also retains the node's explicit missing-newline/truncation notices in
both layouts, with no review target; these have no matching reference line kind.
Tests use the reference replacement/unmatched/context/header/numbering/hunk
cases, plus actual source sharing and stale-cell rejection. Drawing, measured
wrapping, selection, virtualization and C4 draft publication remain separate
work; this is not a document renderer or completed package skeleton.

Typed-local-failure source c0eb84e5 passed view all-target clippy in
`wp-git-narrow-20261115T080003Z-c0eb84e5`; its new tests still need execution.
Split dd6ad9c5 kit tests passed117/0/0 in
`wp-git-narrow-20261115T075426Z-dd6ad9c5`, with the changed kit compiled.
No unchanged-tree whole gate was run, and no first-use pixels are approved.

The initial document narrow check at48004f62 compiled production view code;
six test-only style lints returned (explicit enum arms, Arc clone spelling and
reference-to-pointer syntax). Those are corrected without allowances. The
punctuation fixture now uses a colon below a directory, preserving the existing
relative-path rule that an initial single-letter drive prefix is invalid.
Added a replacement-run/context boundary case. Changed-source checks and test
execution are still pending; no visual or review publication claim.


## Shared-base integration and remaining connections

App base4(3cd0d3fc) is merged at83f79cd13 after base3. Only README appendices
and size ratchets conflicted: both descriptions were retained; slider stays711
and the lead's extracted demo stays517, with no raised record. The combined
binary has not yet passed its changed-source checks.

The split caller now carries the view's existing ControlDensity enum all the
way to the kit's Density, rather than projecting it through a public labelled
boolean. Its full localized action label remains available in either density.
The menu-only shape has no redundant primary half. The document model and typed
local failures remain SOURCE/narrow-only; no current C3 capture exists.

The current pane renderer/context, authenticated connection epoch/preflight,
and refreshed shared menu have not supplied consumer SOURCE handoffs yet.
Their owners were asked; no duplicate pane registry, alias resolver or menu
renderer is created here. Independent node/model work continues. C3 is still
neither a complete skeleton nor built/passed; no tracker row is upgraded.

## Command-center owner adapter

The command center consumes areas::git::commands::{GitCommands, GitCommandTarget}
through Shell::git_command_entries and run_git_command. It retains the opaque
repository/generation/session target and MenuEntry intention; the owner re-reads
the shown root and live entries on activation. The menu now uses the same adapter,
so this is not an unused second policy or RPC path. git_command_item supplies the
same localized label/busy gate; areas::git::glyph supplies Git action artwork.
An ActionRow's unavailable reason distinguishes an enabled primary from an
explanation-only query result. PR and merge artwork remain existing kit glyphs.

The area generation uses its lifetime RequestKeys and is replaced on every
followed-target transition, never reconstructed from a root name. Pure tests
cover A/B/A, changed machine/root/session, changed availability and removed merge
methods. This does not claim G4 connection-epoch admission or full C3 primary
policy parity; those shared/feature integrations remain pending. It needs a
changed-source narrow check and execution before another package consumes it.

## Typed base mismatch and current model evidence

The pending node comparison response preserves stored/requested base identities.
The view now has the corresponding BaseMismatch failure and neutral EN/KO
recovery, with both refs retained as typed detail for the future drawing. It does
not interpret git output or call either ref correct. This bridge is a model
addition; its C3 link and actual notice are not drawn or verified yet.

The immutable base4 model at1b440f6a passed selected View/Kit/App all-target
clippy (wp-git-narrow-20261115T084015Z-1b440f6a). View library execution
wp-git-narrow-20261115T085405Z-1b440f6a compiled the changed View crate and
passed921/0/0, including document source-key/row-layout fixtures and destination,
wrong-file, late/duplicate-answer cases. Those results precede the command adapter
and this base-ref recovery; no current screen or package approval is claimed.

The owner adapter at47e39d98 passed App all-target clippy in
wp-git-narrow-20261115T090604Z-47e39d98 (App actually recompiled). The return
values are now explicitly must-use, particularly the dispatch boolean that tells
a command center whether to close. This annotation and the newer base-ref words
need the next combined narrow check; four pure adapter cases still need execution.
No runtime command-center or C3 screen claim follows from the earlier clippy.

## Shared menu producer integration

Merged the announced074ade16 line (G4 captured dismissal/ReturnFocus; published
native1282/0/2), then isolated8158be6d's control-only ancestry. G2/V1 disposition
is exact KIT SOURCE/narrow only (Kit160/0, no skips); its App check and actual
consumer/F5/hit/clip/OS evidence were still pending at handoff. This is not a
full F1 merge or a C3 UI approval. The producer packet is described in the
shared G2 control plan files retained here.

Conflicts retain G1's owner-projected menu and captured dispatch, adding only
MenuItem constructor defaults for the three original row shapes. Assets union
all variants (92), keeping the existing ActivityIndicator/track/arc artwork and
every older glyph. The split button, current isolated loader and slider are
unchanged. menu_right/TopRight/QUICK/fill_in and the new named placement stay in
one renderer. G2's module/export additions and the single kit focus helper are
retained. Current union source requires its own combined narrow check; old506
and producer tests remain assigned to their immutable sources.


## Shared menu first consumer (source only)

The Git menu now opts into the isolated producer's Regular16 leading glyphs,
Pending/Success, Muted and KeepOpen policies. Pure menu_feedback uses only the
keyed pending intention or original success receipt, never refreshed branch
counts. Direct writes remain open; unavailable choices remain selectable and
explain with the existing Info clock without a node write. The owner adapter
checks MenuItem::selectable as well as its current captured authority, so a
command-center callback cannot bypass pending/success feedback. The primary
retains its established explain-on-busy path through the same target check.

Paseo actions-split-button.tsx/use-actions.tsx and EN/KO resource rows supply
Commit/Pull/Push acknowledgement captions. Fetch and manual branch/stash entries
are Vessel additions: Fetch has a brief data-file caption, while other additional
entries retain their exact outcome notice. Manual dialogs/choosers close the menu
as previously approved; default direct quick commit generation is still missing.
Merge/create rows use shared pending feedback but their complete C4 keyed-success
and forge-brand integration remain pending. Existing kit glyphs are retained until
the logo owner supplies its accepted forge consumer. This does not approve any
actual pending/error/success/Info lifetime/keyboard/touch/F5 captures. New view and
consumer source requires its combined narrow checks and execution.

V1 supplemented the exact8158 producer packet with its same-source App all-target
clippy pass (schedules-narrow-20261115T091044Z-8158be6d); its actual spec was short,
not ordinary. That closes the producer compile-pending note above but does not
prove this union consumer, App tests or UI. No unchanged producer gate is rerun.


Combined View/App all-target clippy at506bf244 passed in
wp-git-narrow-20261115T092254Z-506bf244 (View compiled, App checked). It covers the
neutral base mismatch and must-use command API before the074/8158 integration;
it does not cover the new shared-menu consumer. The adapter's pure execution
cases are still unexecuted. W12 withdrew1720 pending a self-contained successor
and changed-source check; G1 has NOT adopted it or any plugin framework.


## Shared dependency union and readonly diff profile

The exact W12 producer9480d4d0 is adopted as9dddd5021 after its exact canonical
values e46 ancestry. The isolated G4 range074ade16..95c256bc is merged as75fbce0df;
no moving devices branch is used. Later the announced package base6(9e18cf0d)
is merged as c7234f229, preserving own split/menu/models and G4's extracted test
peer. The size list keeps slider711 and takes the reduced demo515, never raises
an allowance. These are source-adoption facts, not combined runtime approval.

The optional Base line-presentation proposal received same-row V1 external
revision3 source-precheck after closing one static unresolved-index return; the
lead authorized installation as vendor patch7. It is installed on this package
branch with the notices and patch README. The initial kit DiffEditor shares
CodeEditor construction and colour projection, disables editing, and admits
immutable text/metadata together. The seven syntax kinds and ordinary editor
engine stay unchanged; no file buffer or pane controller is added.

The source reference is diff-document/paint.web.ts267–310 (one mapped number per
cell, two independent cells in split) and palette.ts (success15%/error10%). Real
label measurement replaces the web's approximate0.62 glyph-width multiplier.
The initial profile retains Base's six-point gutter gap; exact gutter divider/
consumer geometry is incomplete and must not be called complete Paseo parity.
Actual-window tests for None, mapped widths/right alignment, per-frame provider
coherence, wrapping/tints/fixed horizontal gutter, selection/copy and read-only
input are added, NOT executed yet. Consumer first-use UI and the whole C3 gate
remain owed. The optional provider is drawing only and confers no review authority.

The prior a7fd77c1 View/Kit/App all-target Clippy passed, fonts0/cargo0, actual
changed crates checked. Its queued library run and the node40b9 library run
both ended cancelled143 before any step at10:35:42; G1 issued no cancellation.
The build-tools owner was asked to inspect that exact event before any retry.
No tests are recorded as passed from either cancelled run. The new vendor/profile
and current dependency union need their own exact checks and execution.

The readonly profile explicitly uses diff-document/palette.ts's terminal-blue
selection and muted context-number ink; these do not change ordinary CodeEditor.
The blue comes from the existing terminal palette, not a new colour table.
A real-window kit fixture constructs DiffEditor, selects across Latin/Hangul
rows, attempts text replacement and reflows the same buffer. It remains
unexecuted until the exact remote test run; no live IME claim is made.

Before vendor-test compilation, a static visibility audit corrected the new
fixtures to use the editor's existing public style setter/current style instead
of its private projected-style field, and its text_bounds accessor rather than
a nonexistent layout bounds field. Production patch behaviour is unchanged.
The queued8a342503 Kit/App Clippy does not compile vendor tests; actual Base/Kit
library execution must use the corrected fixture source separately.

## Exact pane binding and immutable comparisons

The pure panel binding uses the shared LayoutKey+TabId and original G4 context.
The actual shared enum is LayoutKey::Session (the earlier handoff called it a
legacy-session bridge); the source wins. Only that exact machine/session may
be located through session.get. A real Workspace key waits for W11's registry
adapter, with an explicit NeedsRegistry result and no fake loading read, cwd
inference, selected-session fallback or first-alias-by-node lookup. Discovery
keeps the whole returned SessionInfo and rejects a different session identity.
Retired epochs, duplicate answers, same-alias/node A→B→A and equal tab numbers
in different workspaces cannot establish a repository. Unsent settles failure.
The actor and screen consumers are next; these new pure tests are unexecuted.

Comparison identity now includes an immutable commit kind instead of reusing a
branch-mode ticket for commit panels. Commit comparisons retain no mutable base
and map exactly to the existing Commit scope; empty branch-mode suggestions do
not appear for them. This does not assign a new wire number or imply the pending
committed-only RPC can already run on an old node.


## Patch7 public-literal compatibility correction

The exact8a342503 remote Kit/App check returned E0063 from the external
component input.rs539: its complete InputEditorStyle literal cannot accept a
new field. Original log diff-editor-clippy is retained (fonts0/cargo101), not
counted as runtime evidence. After V1/lead external revision4 precheck, the
public field set is restored. The existing shared HighlightStyleResolver gains
a default-none accessor; with_line_presentation wraps/delegates it and retains
one optional Send+Sync provider. No component registry file or new vendored
component is edited. A fresh projected base style is wrapped once per render;
intentional nested wrappers use the outer provider, and None clears mapping
without changing syntax. Tests pin that full old literal, both lookup outcomes,
nested override and clear, plus the same per-visible-row/prepaint geometry.
Per-instance test counters use Arc atomics, never process-global preferences.
These corrected tests and the changed-source remote check are still pending.

The lead also ruled that split+wrap cannot change Base display_map in this C3
package. The intended bounded departure is: split uses horizontal scrolling;
turning wrapping on selects unified. A later separate immutable display-height
proposal may restore split wrapping only with selection/copy invariance proof.
This is not implemented or visually verified yet; no hidden split-wrap alignment
or filler-newline copy workaround is claimed.


## Scoped pane reader skeleton (2026-11-15)

The main registry now registers WorkingDiff and immutable CommitDiff through
`areas/git/panel`. Its address is LayoutKey + TabId, and each discovery keeps
the exact original G4 ConnectionContext and legacy SessionId. The link uses
only `selected_connection().call`, with eight outstanding reads including event
backpressure and a 15-second read deadline. Full/closed sends settle through
the shell's existing Unsent router. Device retirement is processed before the
devices area's no-redraw early return; newer fleet snapshots retire pane facts.

This is the C3 skeleton, not final pane parity. It currently mounts a bounded
file-summary list and localized loading/retry/failure/partial states. Mapped
editable-engine diff bodies, tree controls, mode/options/history integration,
actual Explorer registration, registry Workspace root admission, observation
and the final capture matrix are still incomplete. Workspace keys do not use
selected-session or cwd fallbacks. The legacy All/Base RPC remains a temporary
compatibility path; committed-only returns explicit unavailable instead of All.
The values-only compare producer awaits its conditional node macro prerequisite
and V3 review; no whole node head was imported into this app tree.

Added source regressions exercise exact alias/node/generation refusal before a
real framed RPC, retirement after the status frame, semaphore refusal with zero
frames, and actual bounded Link::submit rejection settling PanelReads. They are
not executed yet. English/Korean failures use complete language-table sentences;
node diagnostics remain separate code detail. Changed crates need an exact new
remote narrow check; the queued df4 check proves only the earlier control tree.
No whole package/source/UI pass is claimed.

The display policy now makes wrapping select unified and selecting split clear
wrapping. Restored split+wrap data also projects unified. This deliberate Paseo
departure preserves the one engine's selection/display map while independent
split editors lack an equal-height wrapped-row adapter; a later display-only
height design needs separate review. The setters and restored-state table are
new unexecuted source tests.

After the announced base6 18eed merge, pane discovery checks the sole owner's
actual layout_panel_visible query before issuing requests. A closed tab's answer
is dropped and its entry removed using layout_tab_exists; neither query creates
a layout. These are source additions, not executed visibility/runtime proof.

The newly scoped reader now refuses BOTH All and Committed on the legacy RPC
compatibility boundary. Legacy Base can omit a path restored from the index
that is now untracked, so it is not exact All. Uncommitted and immutable Commit
remain exact supported reads. Until canonical pending git.compare values and
its assigned handler land, the default All pane shows a capability recovery
rather than a success-shaped approximate diff. Old toolbar consumers are not
rewritten by this correction. New regression expectations remain unexecuted.

## Document owner prototype

`DiffDocumentEditor` is a thin same-engine wrapper retaining a reveal observer.
It reads the complete existing display map for caret rows and full file height,
removes ordinary source-panel vertical insets and empty bottom rows, and lets
the actual parent VirtualList move on focused cursor changes. New actual-window
source tests drive Ctrl/Cmd-End and Shift-Up on a 2,500-line file; they are not
executed. No new pane consumes the prototype yet. Cross-file selection is not
implemented by per-file editors and is not claimed as reference parity.

The external viewport element2921f9ae received direction/source precheck. The
first external position helper91f1c83e was returned because bounds_for_range
had its own earlier saturating lookup; e5e657f5 additionally guards both endpoint
rows there. Lead prechecked it; V1's same-row disposition remains pending.
Nothing in that external source packet has been installed or compiled yet.
Prepared Base tests include real owner+cached-child scroll, empty/before/after
IME bounds, crossclip highlight/visible touch-end/full copy and folded row-order
checks. These are source drafts, not executed native/OS evidence.

The final external element2921f9ae/statee5e657f5 viewport and coordinate
proposal received V1 same-row SOURCE PRECHECK PASS and lead approval, and is
now installed in vendor change7 with current timestamps. Earlier paragraphs
describe earlier source snapshots. New Base/Kit tests remain unexecuted and
no installed-source, runtime, consumer, UI or OS acceptance is inferred.

Exact df4 compatibility narrow ended fonts0/cargo101 on two View doc-markdown
findings (ConnectionContext and TabId in binding docs). Base was checked but Kit
and App were not reached; no tests ran. Fixing these in the current source does
not relabel the old result. Current new Link/App/document changes still need
one combined changed-source remote Clippy pass and targeted window execution.

## Editor buffer projection

The view now snapshots the existing DiffDocument packing into engine-ready
unified or split columns, preserving source identity, old/new numbers, empty
source lines and split fillers. Hunk headers include their section text; missing
newline and truncated-hunk facts stay explicitly unnumbered and non-reviewable.
One embedded newline inside a wire line refuses the whole buffer through a typed
error, leaving the original document intact instead of guessing new source rows.
Paseo utils/diff-layout.test.ts replacement/unmatched/context fixtures remain the
packing authority; new whole-buffer fixtures cover copy order, final empty rows,
section/notice/cut retention, empty split layout and malformed peer text. These
new fixtures are unexecuted source. Existing queued111 checks exclude this change.

This projection is not the final split header renderer or cross-file selection.
The consumer must still enforce a bounded editor cache and display cut notices;
no UI parity, native interaction or body-cache proof is claimed by the values.

## Base lock correction and exact failed run

The11113b32 combined narrow stopped after native-fonts0 and native-cargo101
because the remote runner enforced --locked against the stale base6 native
lock. No crate was compiled or tested in that attempt. The package-base's
dd13ce378 correction (vessel-rpc's existing serde_path_to_error dependency) is
merged as84fa8b8c8. The already integrated G2 producer source stayed unchanged;
its result-note additions were retained and its duplicate README block removed.
The queued111 Base-window attempt keeps its original source/result; it is not
cancelled or relabelled. A changed-source replacement covers the fixed lock plus
the buffer projection once the remote queue permits it.

## Retirement redraw boundary

Source inspection found that the new panel state was retired before the shared
device-area early return, but the shell could still return without requesting
a frame when no device page changed. The owner now returns whether an exact
original admission was retired; that result joins the existing device redraw
decision. Only two registration lines change in shell/events.rs. Duplicate or
another-epoch retirement remains a no-op. Retiring also clears stale path
anchors and tooltip state, while pending reads settle DestinationChanged. A
new source fixture pins exact-epoch/duplicate/redraw-decision behavior; it is
unexecuted and does not claim a real shell/window redraw or native UI pass.
The already queued84fa narrow excludes this correction.

The buffer projection's exported split-column and malformed-row fields now
document their original/modified/source identities explicitly under missing_docs.
This documentation correction does not change the buffer packing or fixtures.


## Diff lexer state isolation

The initial readonly profile fed unified removed and added text through one
normal lexer state. Source inspection identified that a deleted comment opener
could colour an uncommented new line as old-side code. The profile now wraps
the same existing LineSyntax with two cached states. Removed/added advance only
their own source; context advances both, selecting modified-side output by
default or explicit original-side output for a split old column. Headers reset
unknown omitted context; notices/fillers do not enter the lexer. It adds no
parser, syntax-kind mapping or ordinary editor change, and changes no engine
selection/clipboard path. Context tokenization is twice per cache build, never
per paint. Numbered non-source rows are refused by DiffText's existing admission.
New unexecuted fixtures pin side divergence, header reset, notice/filler skipping,
unchanged language forwarding and metadata admission. Existing queued84/5ac
checks exclude this source. Earlier sparse-hunk syntax context and final header
paint/cross-file selection remain unresolved; no complete syntax/UI claim.

The malformed metadata diagnostic carries kit's own zero-based DiffRowIndex.
The vendor BufferRow constructor stays crate-private; no engine API is widened
just to construct an owner-side diagnostic. This is a static source correction
to the new syntax/admission slice, not an executed compile result.

The document-window test source now also sends actual ScrollEvent packets over
the full-height editor (pure vertical, small trackpad horizontal drift, and
wrapped content), asserting the real parent VirtualList moves while the inner
vertical offset stays zero. This supplements direct ListState and keyboard
reveal tests; it has not run and does not establish wheel behavior. No vendor
scroll-routing change was made from a source-only suspicion.

Closed tabs without an outstanding answer previously had no answer-side cleanup
trigger. Opening another C3 pane or reconciling a changed fleet now prunes closed
entries through W12's existing read-only layout_tab_exists query. The original
key issuer stays above entries, and no cache of tab authority or selected-session
fallback is introduced. This bounds accumulation across repeated open/close;
actual retained memory and full body-cache byte budgeting remain unverified.

## Loading and empty body geometry

The C3 body now follows diff-pane.tsx1077–1139 and2053–2086 for loading/empty
placement: flex center column,64-point top inset,16-point loading gap, neutral
body text and the shared ActivityIndicator Large36. The repository phase has
Checking repository; the later diff phase has only its spinner. Empty uses the
same center/inset with8-point gap. This does not change the old toolbar spinner
or claim a redraw/capture/whole-screen pass; the provisional header, failure
surface and final complete body still need their reference/capture pass.

## Exact compiler correction after the pane/document check

The fixed-lock 84fa native narrow reached the changed View/Kit crates and
returned source errors: the diff profile addressed Tokens::status as a field,
and the packed-buffer projection used a wildcard import. The profile now calls
the existing theme-aware status accessor; the projection names its dependencies
explicitly. The public-field documentation reported in the same run had already
been corrected in 5ac39be2. This is a source correction, not an execution pass;
earlier queued checks retain their original source and cannot validate it.

## Patch7 executable regression boundary

The fixed-lock Base test attempt reached Cargo but executed no tests: gpui-base
is a nonmember dependency requiring dev-dependencies. The lead rejected both an
independent lock graph and adding the vendor as a workspace member. New vendor
private fixtures are therefore removed; public InputEditorStyle/EditorState,
VisualTestContext, the real parent ListState and rendered scene quads are tested
from kit. Existing upstream tests are untouched. Narrow/readiness claims must
name the actual kit test results, not the removed private fixture inventory.
The exact per-label right alignment and private folded-array ordering have no
public query; these remain inspected source obligations, not executed proof.
All new tests are unexecuted until their immutable remote run completes.
