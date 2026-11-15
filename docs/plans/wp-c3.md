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
owned branch-stash metadata. New calls register staged or pending in Quality B,
with admission None until the integrator assigns the completed runtime.
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
will be a control-only change within C3. The reference header is pointer26 /
touch32: control-geometry.ts HEADER_CONTROL_HEIGHT uses the touch sm value32,
not the ordinary pointer sm value28. Native touch keeps the44-point target
required by ADR0036. Shared loaders remain the loader owner's
producer, with no area spinner. The current integrated menu producer is not
ready; historical source clears are not a current API handoff.

## Split-button correction (control scope)

The existing `kit::split_button` is extended, not duplicated. Current reference:
`git/actions-split-button.tsx` (whole component/styles) and
`components/ui/control-geometry.ts`: pointer26, touch32; inset8/12, gap4/8,
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

The ported clipboard fixtures use actual Copy shortcut input after focus/select,
not the vendor's private copy method. This source correction was found before
its queued test executed; that older immutable run remains prior-source only.

## One body projection and empty navigation

The scoped renderer now consumes PanelReads::body rather than reimplementing
status/diff/empty precedence. The new reader fixture covers retained refresh,
a late successful summary after status failure and inactive/disconnected demand.
An actual empty alternative uses the shared Ghost button and rechecks visibility,
original selected admission, branch target and complete captured comparison
before normalizing/reissuing through the same keyed reader. Commit targets never
inherit a mutable branch mode. Pending Committed/All remain explicitly unavailable;
this does not grant wire support or mount the unverified document editor.

## Announced line union at e1b73c9e

The app merged the announced locked-green line e1b73c9e (base8 node82ae,
release line, shared menu lease, selected-tab getter and portable Subagents
values). Only the README had a content conflict; both the G1 diff-profile/pane
notes and incoming external-link/circle-button contracts were retained. The
shared selected getter already adopted as an exact leaf remains identical;
no staged resource implementation or moving owner branch was imported. Prior
narrow jobs stay tied to their earlier source. This merge is not a C3 gate or
UI pass; the staged comparison values still await their own V3 review.

The empty-navigation callback's production guard now has direct fixtures for
no/current/replaced admission, another root, another full comparison and an
immutable commit target. The captured action carries the original context and
repository; it never recaptures a replacement route when activated.

## Approved queued-run replacement

The coordinator authorized cancel-queued for exactly four obsolete checks.
The 5ac consumer had already taken a slot, so it was left untouched. The88e
editor-test/Kit-Clippy runs and762 public-port test became terminal cancelled143;
this is an approved queue cancellation, not a compiler result. Their evidence
is retained. One latest union App/Kit source batch follows rustfmt, file budget,
boundary/token checks and import/doc-markdown inspection. Values194, nodee103
and fa1 Clippy remain untouched. No whole package pass or capture is implied.

## Retained mismatch diagnostics

The provisional body kept typed base/path/node/session mismatches in state but
only displayed a recovery sentence. The view now projects both original and
returned identities (or stored/requested bases) into separate code detail with
EN/KO catalog labels. Git's redacted own output is retained verbatim; no string
selects a failure policy. Whole-output fixtures cover Korean/spaced paths and
ref-versus-commit mismatch. This source correction is not covered by the earlier
queued0af check and has no inspected screen proof yet.

## Replacement-admission late reply correction

Source audit found the scoped app receiver retired its current entry when any
reply context disagreed with the current machine, including an old A reply after
a healthy B entry had replaced it. Delivery now first compares reply with owner: a
different owner is ignored; only that owner's retired admission invalidates it,
with an explicit redraw. Matching live context still requires the exact ticket.
The pure production delivery function has same-alias/node new-epoch, absent-owner,
absent-current and matching cases. This is a source-discovered bug/correction,
not a reproduced UI failure or an executed regression pass.

The public tint fixture checks actual wrapped background height and horizontal
bounds alongside readonly selection/copy; its name does not claim selection-path
z-order. Exact mapped-label right alignment, private folded-array order and
selection/touch-handle paint remain source or first-use capture obligations. The
readonly profile explicitly disables folding; no new test-access API was added.

## Post-line mapped-control capture handoff

The lead accepted retaining exact label alignment, folded-array order and
selection/touch paint as explicitly separate source/capture obligations without
widening the vendor for tests. `kit/examples/mapped_diff_audit.rs` supplies an
optional real-window control consumer with2500 fixed rows, seven-digit mapped
number, English/Korean, wrap/scroll and ordinary selection/copy. The patch7 README
names exact steps; no build/capture/UI pass exists. It has no node authority and
cannot replace later C3 fake-fleet first-use or prove unsupported touch behavior.

## Refresh retains the chosen comparison without retaining authority

The initial pane restart recreated default All on every Refresh, including after
the person chose Uncommitted. Restart now carries only the normalized comparison
from the same target and physical admission. Root discovery still runs again;
no old ticket, metadata, failure or body is copied. Another context or immutable
commit target keeps its own initial comparison. A direct Entry fixture pins all
three cases. This is not a new identity registry or a reconnect fallback.

Public viewport counters observe optional row projections, not every internal
text-system shape call. Source inspection shows the existing cold longest-line
width-cache path can measure one offclip source line to retain horizontal scroll
metrics even with zero visible mapped rows. The lead is asked to rule on this
bounded extent-measurement distinction versus the earlier empty-mask wording.
The empty fixture's name is restricted to zero row projections/direct IME
bounds, not a total shape-call count. No runtime result or silent geometry
change is asserted.

The lead accepted the cold-extent exception: zero projected rows for an empty
mask, one longest-line measurement only on a cold width-cache key to preserve
horizontal metrics, unchanged gutter/space samples and ordinary None path. The
existing element takes/reuses and stores that width with revision/font/size/wrap/
tab identity. A minified very long line has a proportional first-measurement cost;
there is no measured performance pass or total shape-call count claim. README
wording now states this explicitly; no extra vendor correction/API was added.

## Isolated control source prepared

An independent control-only worktree now holds9664fa914 on announced e1b73c9e:
17 paths, no C3 View/Link/app or root runtime. It retains the exact owned vendor
patch7 and Kit profile/tests/example; only its README is narrowed to that scope.
It is unchecked, not a consumer handoff. No extra build was launched while the
approved0af App/Kit source batch is still queued. Its plan is
wp-c3-diff-control.md on that isolated branch; current package proof remains owed.

The fa1 four-crate Clippy reached the updated Kit and returned four concrete
issues: a partial doc-code span, redundant Render qualification and two test API
spellings (RopeSlice has no is_empty; readonly is queried through presentation).
They are corrected in both package and isolated control bytes without lint
suppression or an engine API change. No window test ran in that Clippy check.

## Reviewed canonical comparison values consumed provisionally

V3 row196 passed exact194 plus Markdown-only3fde on the announced82ae staged
prerequisite; the integrator has queued it in batch12 and owns the missing TS
regeneration before gate49. The app merged only that reviewed value leaf as
fa2dffdf, not a moving node runtime. GIT_PANEL remainsNone, and calls still
settle unsupported before any peer frame.

Committed/All now construct canonical CompareGit/CompareGitFile through G4's
same original selected admission and retain exact tickets. Uncommitted/fixed
Commit keep exact legacy calls. Ready moves the whole payload; NoBase and
TooLarge retain typed distinct states, and mismatch retains both base facts.
Refresh rediscovers authority but sends typed Refresh purpose; repaint/file
reads use View. No fetch, runtime admission or approximate legacyBase fallback
is introduced. New View/Link fixtures cover whole query/outcome values and
actual staged summary/file no-frame settlement. These current native changes
are uncompiled/unexecuted; older fa1/0af jobs are not relabeled as their proof.


The exact b011 all-target Clippy returned cargo101 on six new fixture API issues:
two status fixture arguments, three elided Context lifetimes and a test helper
requiring Element rather than IntoElement. Corrected with the existing empty
status fixture, explicit lifetimes and AnyElement conversion; no production
state, assertion or lint is weakened. Current correction needs its own check;
b011 is retained as red, and queued27194 tests are not relabeled.


Workflow correction: the local boundary script was mistakenly called part of
non-Cargo preflight; source inspection confirms it invokes cargo metadata.
No local Rust compile was started, but that metadata invocation still violates
the VM-only Cargo instruction. Future boundary/metadata checks run on the VM;
only standalone rustfmt, the pure file-size/token scans and text/import scans
remain local. Earlier boundary output is not labeled workflow compliance.


## Current target and local owner incarnation fences

[Decision Log]
- Existing asset: W12's checked portable17094/6f1a/0396 CURRENT-target query;
  adopt only those three leaves, not its moving resources branch. It is not
  an ABA/visit/lease/ticket proof, and our combined compilation is still owed.
- Problem: C3 checked only tab existence/visibility, so a retained mode callback
  or reply could run after that same tab became another target. Same-root
  Refresh also recreated an indistinguishable local mode callback owner.
- Choice: use the sole book's current target query before deferred opening,
  answer installation, cleanup and mode activation; retain the already-issued
  discovery RequestKey as the local entry incarnation after discovery settles.
  The callback captures that key plus exact typed PanelTarget, original physical
  context/repository/comparison. No independent layout or connection book.
- Effect/limits: changed targets and recreated entries are refused; old replies
  cannot evict a current replacement just because their old target differs.
  Unobserved A-to-other-to-A on an UNRECREATED owner still needs W12's actual
  synchronous owner-retirement hook. That producer is not ready, so no complete
  visit/ABA/authority/UI acceptance is claimed from the current-target query.
- Evidence prepared: actual layout retarget + direct owner callback on a test
  window with a detached bounded Link; assert no git command and old answer
  cannot retain its stale owner. Existing pure fixture checks new incarnation
  on identical root/context/comparison. No test or new source check has run yet;
  current bdeed/08b1 jobs predate this App-only source. Final package tests must
  include this union. No layout source beyond the reviewed leaves is edited.


## Exact bdeed Link compilation return and shared writer boundary

The bdeed four-native-crate Clippy returned cargo101 after checking Kit, Link
and View. Link's command handler became101/100 with the one GitPanel registration;
one own doc item lacked backticks; seven test-helper unwrap/panic diagnostics
came from the combined cfg(all(test, not(wasm))) module declaration. No tests or
full command pass is inferred from those intermediate Checking messages.

Own correction quotes selected_connection and uses separate cfg(test) plus
cfg(not(wasm)), matching the existing testing module. Rust's inclusion condition
is identical; Clippy now sees the existing test-only policy. No allow/expect,
assertion or production bound is changed. A proposed same-file SavePrefs body
extraction was NOT installed: the lead designated one shared author (W10b),
whose checked isolated extraction will land on the line first. Its SavePrefs
parameter is already boxed on that upcoming line. Wait for the announced green
head and keep only the GitPanel registration arm; no competing helper or raised
line budget. The next coherent App check includes this union and the current-
target fixes, rather than replaying the known over-budget old base.


## Actual list reborrow failure and summary path correction

The08b1 Kit window run executed13/3/0; two failures traced to the fixture/audit
calling VirtualList.width while GPUI List already mutably borrows its state to
render that same row. Current summary file_row also used that pattern. It now
uses the already available native text_ellipsis_middle + nowrap against actual
layout width, keeping the full typed path in the existing code tooltip. No
window-width estimate, second sizing controller or shared list API is added.
A real narrow VirtualList/Shell snapshot regression draws the long Hangul/space
path and checks that its row exists within the viewport. This test is written
but not yet compiled/run; it proves no node authority or broad visual parity.
The controlled full-window Kit fixture/example separately capture their known
root viewport width before entering their callback; no panel-width claim is
made for that full-window setup. Existing assertions stay; no vendor fix or
shared width API was installed. All three08b1 failures and20 test/example lint
diagnostics remain actual red evidence until changed-source selections execute.

## Toolbar source audit still owed in the full C3 consumer

Current reference whole actions-split-button.tsx + control-geometry.ts use
pointer26/touch32, with native44 still the documented ADR0036 departure. The
earlier compact28 note confused the ordinary pointer sm size with touch header
height; geometry here was not silently changed. The old Rust module comment
quoting28 needs the same documentation correction at its next source checkpoint.

Source observations are NOT an approved toolbar UI result: use-actions.tsx
isActionDisabled includes pending, and the primary disabled style applies0.6,
including its pending spinner. Current native Busy is nonactivating but does not
apply that opacity. The reference only draws a caret when secondary actions
exist and keeps its separate overflow trigger/menu(width220); the legacy toolbar
currently puts its remaining actions in one caret menu. These need the existing
shared control/pure Git policy integration, not an area-owned replacement menu.
No Busy opacity/shape/source callback change is installed by this audit. It is
not part of the isolated patch7/diff-style producer or its checks.

## Actual15a fixture compilation corrections

The four-crate15a strict check reaches App and returns101. The new fixtures
incorrectly imported size from kit::ui, and manufactured TabIds through an
undeclared serde_json dependency. They now import gpui::size and allocate
fixture TabIds through the existing Layout::open, with no new dependency or
raw counter. Remove two unused AppContext imports and a redundant qualified
ConnectionContext, add the requested unit callback semicolon, and collapse the
identical summary let/finish condition. No gate, assertion, caller admission
or target/data semantics is weakened. This successor still needs checking.

## Unstamped legacy write path is closed pending the real C5 producer

[Decision Log]
- Source fact: code_git_context exposes alias/root, not the original physical
  context of drawn summary/diff. W13 confirmed its current root receipt alone
  cannot upgrade already drawn data. The coordinator assigned that producer to
  W13, joined through W12 typed Explorer, and requires failclosed writes.
- Reuse: the existing one Changes controller, G4 selected typed call and Info
  toast host. No guessed DTO, alias resolver, current-Machine stamp, content/count
  authority, duplicate controller or fake Ready source is added here.
- Choice: the old GitWrite path has no place to carry that proof, so it no
  longer sends any alias-only write. It emits whole EN/KO Info guidance before
  allocating a key/reserving pending or sending. Future genuine scoped callers
  replace this path after the checked producer; do not re-enable this old send.
- Existing pending payload/intention is retained on a refused new request. This
  is a local state test, not physically sent-task/ABA/uncertain receipt proof.
  Legacy target-follow lifecycle is still not the full new owner lifecycle.
- The stash-and-switch confirmation is kept rather than consumed into a fake
  pending stash/follow-up. Legacy toolbar PR/create/merge callback entries also
  show Info without invoking forge operations or closing that menu. Standalone
  C4 facts/actions and its full physical-context cutover remain separate work.
- Tests: real isolated Shell + bounded detached Link, matching current alias
  context does not authorize a request; no command/key/pending starts; a prior
  payload/intention survives refusal. Whole FTL words tested in both languages.
  They are written/unexecuted until this successor's exact batch.

This intentionally suspends legacy writes; it is not functional C3 write parity
or a merge candidate. Reading old data stays readonly. Known-unavailable menu
presentation and eventual scoped write activation remain part of the final C3
consumer; no authority is inferred from a successful read or stamp alone.

## Deferred body reads retain the accepted summary generation

[Decision Log]
- Existing assets: PanelReads, ComparisonTicket, lifetime RequestKeys and the
  existing numbered diff rows. Paseo workspace-cache.tsx reuses actual source
  object identity, not equal path/count metadata, across model variants.
- Constraint: Ready previously retained data without its request key. A queued
  row callback could therefore survive identical refresh data or comparison
  A-to-B-to-A if it checked only repository/comparison/path.
- Choice: preserve the accepted summary ticket in the same read store. A
  FileReadTarget captures only a present file in Ready status/summary, and its
  currentness is rechecked before the owner issues a request. A body ticket
  retains its parent summary generation; refresh invalidates acceptance before
  the next summary arrives, even while previous content remains drawn.
- No second controller, parser, NodeId map, root resolver, capability or external
  dependency. Explicit lower-level/direct-file reads remain available; future
  drawn-row consumers use the captured-target boundary.
- Tests cover ready/missing file, identical refresh before and after settlement,
  late/duplicate body reply, mode round-trip, retirement/reopen, failed status
  and failed summary. They are written, not yet executed. This source is newer
  than f1ab's registered App checks and borrows none of their outcomes.
- Limits: local generation is not physical connection authority or an atomic
  repository snapshot across pages. Body mounting, offset echo/pagination and
  global payload/editor retention bounds still need their real consumers.

## Actual f1ab strict-check return

The exact f1ab four-crate check reaches all4 crates and returns101. Removing
unsafe toolbar dispatch exposed unused legacy PR opener/merge paths and the
old prefill constant, plus one Git import; new origin docs also lacked code
markup. Five fixture HashMap lookups and one ambiguous Default caused the
remaining test lints. No failure is relabeled as a pass.

The successor retains active PR callback entry points but puts the same Info
origin refusal inside each, before any form/read/merge operation. It removes
the unsafe alias-only form opener/prefill and merge-send implementation; no
dummy call, unreachable branch or lint exemption keeps them alive. A new real
Shell regression checks both callbacks queue no prefill/merge command and keep
a prior merge payload. This proves local receipt retention, not a physically
sent forge operation. Typed C4 physical-context opening still remains missing.
Fixture lookups use test indexing and explicit Rc default; assertions unchanged.
The existing f1ab test run remains prior-source and must not cover this change.

## Git consumes the conditional shared retirement mechanism

[Decision Log]
- Prerequisite: announced82ca and ONLY shared80c9a8b4, whose V1 bounded source
  clearance is conditional on a real owner in the same checked consumer union.
  It is not the W8 feature branch. Exactbc Scripts Clippy0 and PRIOR368 runtime
  22/0/0 establish their stated scopes, not a Git result or standalone80 pass.
- Startup: one Git registration from Shell::new after shortcuts::listen, before
  route arrival. Empty Owners::new stays empty. Duplicate registration retains
  its typed error, does not replace a hook, and keeps Git pane reads/commands
  unavailable through a whole EN/KO recovery message. Without-IO fixtures must
  deliberately call the same initializer for actual owner behavior.
- Callback: use the original Notice workspace/tab/target to drop only that
  pane's UI and read tickets synchronously after a committed mutation. Matching
  target equality on its own never serves as a visit/ABA fence. Reopening goes
  through the existing discovery and lifetime key issuer again.
- No layout reentry, IO, RPC, second table or task cancellation. The Link keeps
  its detached reads; global pending mutation payloads/intentions are retained.
  This does not establish a physically sent/uncertain write lifecycle.
- Tests: actual Git owner startup missing/duplicate, real retarget A-to-B-to-A
  with fresh discovery and late callback/answer, failed transaction rollback,
  retained pending payload, same TabId in another workspace, and failed versus
  confirmed close. All are WRITTEN/UNEXECUTED until this exact union's checks.
- Shared edits are exactly the six authorized common promotion files from80
  plus the startup call in shell.rs; registry keeps G1's existing renderer arms.
  No Scripts renderer/startup or canonical-preferences/resident promotion.
- C5's original drawn-context producer/helper is still unready, and the actual
  Changes Explorer is unmounted. This callback cannot invalidate C5's source
  until that exact delegated helper is consumed. Legacy writes remain refused;
  C4 PullRequest is also unmounted, so no forge ownership cutover is claimed.

## Exact owner-union check return and startup boundary

The713 union's four-crate Clippy stops in View with two unused borrowed results
in file_target; App checking was not reached. Replace the discarded ready/file
references with explicit existence predicates, preserving the accepted-generation
rule. No warning is suppressed and no result from713 is claimed as green.

The legacy Changes header now shares the existing owner refusal renderer before
scheduling its deferred follow. Follow/status/branches/stashes also refuse while
the Git owner is uninitialized or registration failed. This matters because an
old deferred follow could otherwise replace the target and clear an existing
pending payload even though pane admission is closed. The real negative-startup
fixture installs a target and pending payload, then attempts a different-root
follow and all three reads: target, payload and intention must remain unchanged,
and the detached Link must receive no command. These assertions are unexecuted
until the successor batch; normal Ready legacy follow remains an incomplete
lifecycle, and unstamped legacy writes remain refused. No C5 source is stamped.

Exact1541 View git tests completed142/0/0 with872 filtered and actual View
compilation, including the four accepted-summary generation regressions. This
is prior source, not a result for the new owner/startup union. Node34bb TS
generation stopped on the missing CommitAbbreviation classification; the
4be successor adds the display-only alias and a filter-matching regression.
Its one TS run is queued; no generated artifact or freshness is claimed yet.

## Bounded file-page lineage before document mounting

[Decision Log]
- Assets read: web use-changes.ts want/inflight/generation and diff-rows.test.ts
  stable hunk keys; existing PanelReads, ReadState, FileDiff, GitHunkOffset and
  Link selected typed calls. Keep the same transport and row projection.
- Choice: each accepted FilePages retains its complete assembled FileDiff and
  original FileTicket. A continuation activation captures the accepted summary
  generation AND previous body request key, and may start only from Ready data.
  Loading removes continuation admission immediately. Initial paint callbacks
  cannot supersede an in-flight read or restart ready data; failed data may
  restart from zero, and explicit lower-level reads remain explicit.
- FileTicket now echoes from_hunk to both legacy git.diff and canonical
  git.compare_file. Link no longer resets a requested continuation to zero.
  A refusal still carries that exact original ticket and physical context.
- Admission checks path, old path, status, mode, content kind, prefix hunk count
  and advancing contiguous more_from before appending. A malformed or changed
  prefix settles a typed PageMismatch cause, using a complete EN/KO mismatch/retry
  sentence. It never silently joins a changed file to an old prefix.
- Payload limits per assembled file: eight MiB of UTF-8 field bytes,32768 hunks
  and100000 typed lines. Saturating sums refuse overflow; they do not admit it.
  This bounds retained data, including empty-line/hunk overhead, not transport
  decoding, editor allocations, total process memory or a global cache. Existing
  node patch limit is four MiB raw. Global retention/admission remains required
  before a body renderer can be mounted; no parity or full memory bound claim.
- Preserve every FileDiff field and line cut/no-newline marker. Reusing the
  existing row projection produces stable hunk0/hunk1 keys after assembly.
  Local request lineage still does not promise an atomic working-tree snapshot
  across separate calls. The feature constant remains None on staged nodes.
- Tests written: full two-page payload/key parity, duplicate/malformed progress,
  metadata/content mismatch, equal-summary refresh, failed continuation/retry,
  per-page and combined budgets, complete EN/KO recovery, and a real framed
  original-client nonzero offset with echoed failure. They are UNEXECUTED until
  the next meaningful changed-source batch; no queued480 outcome covers them.

### Actual480 lint return retained

The480 strict run reaches View and Kit, then returns101 before App: the
question_mark lint prefers ? to the explicit readiness predicate, while a bare
borrowed result previously triggered unused-results. Bind the successful
borrow as _ready_status rather than discard it or suppress either lint.
The complete-sentence recovery fixture reached102 lines after Owner was added;
move the unchanged cases into a module constant, retaining every whole-sentence
assertion. No test input/output is weakened. The480 ordinary runtime remains
queued and prior source; the new page source borrows no unexecuted result.

### Actualad9 page-check return

The exactad9 Clippy stops in View on four diagnostics: two fixture mode literals
need separators, the hunk-count conversion needs its ignored numeric-limit cause
named, and the fixture's half-budget division must use checked arithmetic.
The conversion is already below the named hunk cap; overflow still refuses as
TooLarge instead of narrowing. No node/transport diagnostic is discarded.
The correction changes no fixture expectation or production page semantics.
App checking has not yet been reached in this run. The ad9 View/Link git tests
are already running; keep their original source assignment and do not cancel.

## Exactad9 file-page runtime result

Run wp-git-view-tests-20261115T180414Z-ad96564f completed native-fonts0 and
native-cargo0, no skips, actual View/Link compilation. Selected git libraries:
Link6/0/0 (90 filtered) and View194/0/0 (830 filtered), total200 passed. All nine
new page-model cases and the actual framed nonzero continuation case are named
ok in the raw log. The bounded byte/line, parent generation, duplicate, retry,
metadata/content and bilingual recovery assertions actually ran.

The separate exactad9 Clippy remains RED101. Its four diagnostics are corrected
in7bf553b18; that newer source does not inherit this exact runtime result. The
current App owner union still has no runtime/compile pass; its480 ordinary tests
remain queued. No unchanged whole rerun, body mount, global cache bound, real
node, authority, OS, captures or C3 completion is claimed. Full immutable source
and log digests are in the external file-pages-ad9-result.json evidence record.

## Empty-mode callbacks retain their accepted summary receipt

The existing comparison/root/context/entry guard cannot distinguish a comparison
A-to-B-to-A after an identical summary refresh in the same pane instance. Reuse
PanelReads' accepted ComparisonTicket as the data-dependent callback receipt;
summary_ticket is unavailable during status/summary refresh or failure. This is
not another authority cache. The empty-state drawing captures only a Ready
receipt and does not draw a stale action while retained data is refreshing.

ComparisonChange now checks that receipt as well as the existing complete pane,
instance, root and original connection guard. A real detached-Link Shell test
keeps those other facts equal, changes comparison away/back, accepts identical
empty data, and proves the old callback sends no command while a newly captured
one sends exactly its original-context comparison. Ready fixtures now accept
actual status and summary data instead of treating an Empty store as actionable.
This test is written/unexecuted, not a native input/capture or authority claim.
No destructive action or legacy write barrier is opened.
