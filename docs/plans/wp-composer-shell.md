# Composer shell and commands

Status: implementation in progress; independent models tested, native integration pending. Package base4 `3cd0d3fc` merged at `f6b50dd74`.

## Existing assets and decision

[Decision Log]
- Purpose and intent: carry the reference composer shell, commands, editing and sending semantics into the native client.
- Existing implementation and constraints: `view/composer/{slash,recall,drafts}.rs`, `view/session/delivery.rs`, `view/prefs/send.rs`, the existing TextField, autocomplete and attachment lightbox already provide parts of this flow. The current agent command contract has no skill kind or draft-list call. Plugin contributions and file-panel navigation are separate packages.
- Alternatives considered: a second editor; a new dependency; extending the current text engine and pure models.
- Choice: modify existing models and extend the current editor through one composer-shell area. No new dependency. Keep attachments and file mentions in their owner's modules.
- Why: preserving one mounted editor retains caret, selection, undo and IME state; pure transitions can be tested without a window or node.
- Advantages, costs and effects: shared controls remain consistent; command and file consumers depend on the upstream typed APIs rather than pretending an absent RPC exists.

## Reference evidence (Paseo 0.15.4)

Paths below are relative to the reference checkout's `packages/app/src/`.
- `composer/input-mode.ts:1-47`: chat exposes attachment/voice/autocomplete/controls; terminal exposes none and uses the code face.
- `composer/input/input.tsx:1205-1224`: fullscreen send requires sendable content; the fullscreen affordance requires compact, editable, no overlay, more than two rendered lines.
- `composer/index.tsx:200-207`: preserve-and-lock locks only while submitting; ordinary submits clear.
- `composer/input/state.ts:9-15,140-190`: pending permissions turn queue into interrupt; alternate while running flips queue/send; Shift+Enter is always newline.
- `composer/input/state.ts:254-269`: retained selection is clamped to the current text and replacement identity avoids resetting live edits.
- `client-slash-commands/index.ts:19-87`: `/exit` (quit/q), `/clear` (new), immediate, exact whole trimmed token, attachment-free only.
- `utils/agent-command-autocomplete.ts:33-139`: merge/rank name and aliases, nearest whitespace-delimited slash, inline entries only for provider skills, trailing space only at text end.
- `composer/recall.ts` and `stores/session-store.ts` sent-message selector: recall only loaded nonempty trimmed user messages, consecutive duplicates collapsed.
- `assistant-file-links/{parse,resolver}.ts`, `assistant-image/acquisition-cache.ts` and `assistant-selection-copy/content.web.ts`: file lookups/openers and retained image reads are shared with file panels; native copy retains semantic markdown/code instead of DOM conversion.

## Structure and data flow

- `view/composer_shell/`: typed surface mode, access, submission policy, fullscreen transitions and command ownership. No toolkit or I/O.
- Existing `view/composer`: reuse recall, ranking and drafts; do not duplicate attachment state.
- `app/areas/composer_shell/`: native shell orchestration and view projection. Existing composer input wrappers delegate here; attachment hooks remain installed.
- Existing link prompt, cancel, queue and board action boundaries are reused. New RPCs are requested from their node owners before wiring.
- Words live in the English and Korean Fluent composer-shell catalog.

## Deliberate differences and pending contracts

- Vessel's existing queue is node-owned and durable; Paseo's composer queue is client-only. Preserve the node contract and document the resulting cross-client visibility rather than adding a second queue.
- Native assistant selection copy will use source markdown/code; Paseo's web implementation reconstructs markdown from DOM selection.
- Provider skill metadata and draft command queries are pending under ADR 0085 on the node branch. Discovery uses the existing bounded handshake without a prompt; native-driver catalog ingress and app consumption are not complete.
- Plugin slash commands await their contribution owner's typed API.
- Assistant links/images await timeline and file navigation seams; attachment image lightbox stays shared.
- The current editor caps growth at twelve lines; the shell extension will replace this cap with reference viewport capacity when the existing-control seam is approved.

## Validation

Narrow checks through the shared build lane (ordinary while the short lane is saturated); whole native gate and captures once when the complete package is ready. Only fake nodes and an isolated display. The initial independent view tests passed 10/10 after fixing a const-method and borrowed-closure compile error. Expanded catalog tests and Clippy are pending. No whole gate or capture has run yet.

## Resume point

Done: read package and architecture rules, inspect reference and existing code, agree attachment/input ownership, create isolated worktree and target.
In progress: ordered client/plugin/provider catalog and send gesture projection added to the view skeleton; Clippy and expanded tests are still queued. The markdown callback/copy extension is a separate uncompiled draft. No native app integration has landed.
Next: native input integration, command source APIs and exact submit interception; then complete package checks, capture matrix and reviewer handoff. The announced package base is merged; shared field work awaits the W4 events/Editability base and E2's presentation split.

## Integration seams agreed

- Attachment owner keeps `composer/mod.rs`, attachment/@ modules and `session.rs`/its extracted submit module; this package owns `composer/slash.rs`. Preserve its hooked paste callback. File-key dispatch runs before slash handling.
- Add one early `composer_client_command` delegation before submit/queue handling only after the lifecycle callbacks compile. Lifecycle exposes `composer_exit` and `composer_fresh`; archive failures remain visible and do not navigate. This differs from the reference's optimistic close-before-archive and is the lifecycle owner's failure-preservation policy.
- Expanded editor renderer returns an optional element at overlay priority 2; lightbox remains priority 3, tooltip 4. The existing TextField engine is retained. Its drawing split must land before this package edits field presentation.
- Markdown host calls the backward-compatible `chat_markdown_with` options overload. Link/image callbacks preserve find keys and versioned state; source-copy policy is opt-in. The new callback extension and code-copy adapter are uncompiled.
- Keyboard owner supplies typed composer actions; this package exposes a dispatcher after the action API lands. Hardware audio is one shared link engine, with exclusive Dictation/Conversation lease; this package consumes dictation rather than creating another capture engine.
- File/image reads and opening delegate to the Files package's authorized WorkspaceRoot/RelPath navigation and paged reads. Absolute/home paths outside known roots and host-bearing file URLs receive a refusal. HTTP(S) is the only external link class currently parsed; other schemes need an explicit disposition rather than being sent to file lookup.
- Native clipboard currently exposes plain text only: source Markdown and code-aware text are planned; the reference web clipboard's HTML flavor is not exposed by the platform API.

Messaging identity is owned by the timeline protocol package (ADR 0084). Attachment submit ingress allocates its canonical client id once after client-command dispatch and before optimistic presentation or preparation. The shell retains that id; it does not create an independent receipt or message queue.

App state skeleton now registers `areas::composer_shell` by existing DraftKey and gates the existing slash menu through its surface policy. No alternate submit listener, fullscreen button or node RPC has been exposed yet. The editor split and lifecycle callbacks remain pending. Markdown callback/copy adapter remains unverified until its narrow kit check.

The first remote view Clippy on `9e8e2164` compiled the models and returned only missing field documentation on provider catalog entries. Those comments are fixed; a new narrow run is required. Earlier independent tests were 10/10; expanded catalog/sending/private-controller tests have not run yet. Package base `8d5513e4` merged without conflicts at `9e8e2164`; the editor split is still held until its owner reports narrow-green.

Strict file-fragment parsing now rejects repeated markers, signed line numbers and malformed column markers. The earlier view-test submission did not receive a run id during the shared submit-tool incident; it is not test evidence. New checks use an explicit commit hash after the tool repair.

Assistant image cache reference correction: `assistant-image/acquisition-cache.ts:70-86` enforces a soft capacity, stopping eviction if every entry has active consumers. The pure image cache reproduces that rule, one flight per key, unique retention tokens, idempotent release, failed-entry retry and late-result fencing. Payload fan-out is Arc-based; resource ownership drops on eviction. Added tests remain unrun. New English/Korean `composer-shell.ftl` words are prepared for the shell/menu integration.

The second view Clippy (`52648a3c`) found two collapsible conditionals in strict link-location validation; these are corrected without changing parsing behavior. Both remote failures remain recorded as failures; expanded view/cache tests are the next narrow check. W4 supplies the sole shared Editability type/setter, E2 supplies select-all and the presentation move, and this package supplies composer layout and ButtonOnly after that lineage is announced.

Expanded view tests passed 23/23; kit and view all-targets Clippy passed on `8403be97`. The first markdown helper test run passed one and failed one: its test range found the opening fence instead of the closing fence; the test now uses the last fence occurrence. The product parser did not need a change. Package base 2 (`6036eeb8`) merged at `b06fcbcc` after that test completed. E2's W4-based presentation lineage remains pending.

The coordinator approved this package as sole writer of an optional backward-compatible image activation callback through the pinned TextView/NodeContext. Both existing image render paths share the interaction helper, with no second renderer. Kit Actions exposes a typed ImageHref primary-activation handler; acquisition and lightbox ownership remain Files/W6. Regression tests and the vendor patch-list entry are prepared; the new seam has not yet been checked.

The first image-seam check found the existing keyed image element is `Stateful<Img>` rather than `Img`; the interaction helper now preserves that exact type, and unused imports are removed. A further coordinator-approved inferred-link seam decorates existing paragraph marks with validated byte ranges rather than custom atoms. Existing authored links/fenced blocks are skipped; source copy ignores inferred link markup, and the typed activation preserves authored/inferred provenance so literal percent filenames are not decoded accidentally. Source selection/find regression tests are prepared but unrun.

Recognition grounding: a disposable Node oracle ran the original reference `FILE_PATTERN` for prefix, Unicode, sentence, domain-shaped and invalid-location fixtures. Prose recognition intentionally follows syntactic matching before click-time validation; complete inline code uses strict path classification. Dotfiles and explicit relative executable paths follow `parse.ts:598-646`. This work adds no regex dependency. Raw invalid colon/query locations are refused while percent-encoded filename colons decode only after suffix parsing. The new tests have not yet passed a narrow run.

Field handoff: E2's `9a173ca0` is merged (`23bdb8f3`), with its actual field engine tests subsequently reported green20/0. Package base4 `3cd0d3fc` is merged at `f6b50dd74`. This package now owns `field/composer_layout.rs` and `field/send.rs`: one renderer over the same Entity for bounded/full-height chat/terminal, measured wrapped rows via display_map, and ButtonOnly through the existing Enter listener. Readonly stays solely W4's setter; the layout never resets it. Existing call signatures retain their old12-row rendering until a host passes window-aware layout. Row fits use a bounded16-comparison integer search (exact u16-to-f32 conversion) instead of unchecked float casts or another numeric dependency. A retained row request avoids notify loops. New geometry/actual-enter/selection-retention tests are unrun.

W6 explicitly requested a staged consumer to break its standalone @ handler cycle. Commit `1dddb40d6` calls its typed file-key API before slash/recall and preserves paste; this requires B1 and is not standalone app-green. Lifecycle callbacks are narrow-green at W21 `963ca595`, but their consumer guard/fullscreen registration remain to be implemented. Retained context must suppress all client commands.

The expanded native view run passed26 tests and failed uppercase HTTP scheme parsing;
`4ae17d69e` normalizes only the scheme before the canonical typed URL parser. Rerun pending.
The next kit compile found that Component's compatibility TextView does not expose
the new Base callbacks. The kit now uses that same Base renderer and retained state
directly, refining the Component-installed TextViewDefaults style rather than
duplicating its theme adapter. Existing selection/find/versioning remain in place.
Two test-only import/accessor errors are corrected; current narrow checks remain pending.
Node command unit tests passed14/0 and fake Codex/options integration passed29/0;
these are scoped checks, not whole-package or UI approval.

Client commands now delegate from the common submit ingress before optimistic
echo or attachment preparation (`099ef4498`). Exact tokens with any files or retained
context stay provider prompts. The guard reuses W21's acknowledgement-gated archive
and fresh lifecycle and does not clear the input itself.

The compact fullscreen host mounts the same TextField Entity once; the dock draws
an empty slot while it is expanded. The input wrappers now receive their existing
Window so inline growth uses measured viewport height, and captured DraftKey actions
refuse a different selected owner. Existing composer hooks retain IME admission and
paste dispatch; Escape collapses before file/slash/recall, and fullscreen suppresses
those menus. Shared integration is limited to two input call sites and one priority2
overlay registration, below W6's priority3 image lightbox. Actual GPUI retention,
button-only Enter, empty-send and readonly-collapse tests are prepared, not passed.
The temporary selected-resource adapter must be replaced by W12's sole resident
context after its checked producer arrives; it creates no additional registry.
Fullscreen send availability reuses current attachment/create admission. W6's newer
context-only admission must be consumed before claiming context-only send parity.

The existing autocomplete now projects typed client/provider rows from the shared
catalog. Client rows execute immediately only through the attachment-free guard;
drafts hide their rows while reserving their names and aliases. Inline entries start
from provider entries directly (reference use-agent-autocomplete.ts:231-244), so a
future explicitly typed skill may reuse a root-only built-in name. The current old
wire has no kind metadata and remains root-only until the canonical producer merge.
Mouse selection checks the drawn DraftKey, text, caret and row label before acting,
preventing a stale menu from archiving a replacement selected session. Presentation
was extracted to its own module to keep the root below the500-line cap. New menu,
context-suppression and theme-preservation tests are prepared, still unrun.

Source inspection found a real overlap risk: inline code and inferred link marks
can cover the same bytes, while source copy walks marks sequentially. Opt-in
recognized runs now reuse the existing highlight sweep to combine those ranges
before copying; code/emphasis and inferred links still use the same text and
source segments. The unchanged sweep body is shared generically, rather than
introducing another renderer or parser. GPUI selection fixtures now also cover
recognized links within bold prose and bold inline code, asserting one copied
occurrence. This correction is source-only until the new test head is checked.

Code-aware selection now queries existing parsed code blocks/inline marks through
TextViewState::selected_code_text rather than scanning fences independently. The
query refuses mixed prose, multiple independent regions and stale replacement
documents; it preserves leading indentation and strips trailing line breaks. The
existing window selection aggregator still prevents a single-message override of
a multi-message selection. This removes a second parser with different nested
fence/inline-code behavior. Actual GPUI query fixtures are added, not yet passed.

Exact3e kit tests137/2 failed: the inferred inline-code fixture reproduced
duplicate source bytes (fixed by0293), and compact Enter inserted two newlines.
The composer layout now retains the shared form renderer's bubble-phase Enter
stop: the sole engine/listener has already inserted or submitted; allowing native
fallback inserted a second newline. No second submit listener or key interceptor
was added. Existing IME/menu captures still run before the engine. This correction
awaits actual rerun; the earlier failed result remains evidence, not a pass.

Fullscreen controls now use the shared glyph-only pill and translated accessible
labels, with the inline control positioned over the text surface rather than
adding another row (presentation.tsx:109-124,145-162). Pointer-down preserves
editor focus; compact targets retain the shared44px hit area. Fullscreen uses
surface1 as the reference. Final geometry/captures remain unverified.

The draft command adapter cannot construct an authorized DraftCommandConfig yet:
W3 confirms no checked root/config snapshot accessor (scope intentionallyNone),
and W11 authoritative WorkspaceLookup is not live. COMPOSER_COMMANDS remainsNone.
No cwd-derived authority or Workspace-to-New fallback is used. Native clipboard
supports text/image/files but no HTML representation, so copy currently supplies
semantic Markdown/raw code as text; rich text/html clipboard parity is unavailable.

Typed SubmitPress metadata is scoped around the existing synchronous PressEnter
callback and shared by clones of the same field. No second subscription, submit
callback or global modifier read is used. A guard restores nested scope and clears
metadata after callback return. Buttons/asynchronous work see None; adapters must
copy it into their own typed events. This is control preparation for alternate
send and scoped callbacks, not activation of another delivery policy. K's later
Legacy/Windowed notification arms and W13 captured-submit widening must preserve
the same one guarded invocation. Real-engine modifier/newline and nested guard
fixtures are prepared; compilation remains pending.

Client/plugin parsing and inline token boundaries reuse the existing shared
matcher::whitespace predicate, exposed crate-only with F4 owner agreement. Its
body/ranking are unchanged. JavaScript trim/\s includes BOM and excludes NEL;
Rust trim would otherwise execute a literal /exit+NEL as an archive. Adversarial
BOM/NEL fixtures cover client refusal, plugin arguments and inline boundaries.
References: client-slash-commands/index.ts:49-69 and
plugins/client-slash-commands/model.ts:51-62. No second whitespace helper added.

Copy boundary refinement from the primary source
assistant-selection-copy/content.web.ts:155-199: code-only drags retain raw code
even when every character is selected; a selection crossing the semantic boundary
retains Markdown. TextView Select All has whole-message semantics, so it must not
be reinterpreted as an inside-code drag. The readonly query now refuses Select All
and uses only actual retained per-node selections. GPUI fixtures check full-view
Markdown clipboard first, then a real inside-code drag/raw clipboard. This removes
unneeded duplicate all-document AST reconstruction. New source awaits rerun.

The obsolete retained source clone and private finish source argument are removed:
the same versioned TextViewState already owns source and parsed selection. Find
marks, revision updates and the public Options/Actions API remain unchanged.

With W13's explicit staged-consumer approval, only267d's captured on_submit
widening is consumed: existing factories accept immutable Fn and box it once in
the existing Handlers. No older factory/style/root extraction is restored. The
same listener and SubmitPress guard own invocation; edit/focus/blur stay unchanged.
W13's real-engine changed-selection/captured-opening fixture is retained verbatim.
This permits a scoped resident constructor without an extra callback model/entity
or reentrant weak-Shell update. K's future Legacy/Windowed integration must retain
this boxed submit and the one guarded invocation. Combined compile is pending.
