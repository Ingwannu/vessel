# Composer shell and commands

Status: skeleton in progress; independent view transitions/parsers tested, integration pending. Base: `1d1f11df`; merge the announced app package base before final verification.

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
- Provider skill metadata and draft command queries are unavailable on this base: do not infer skills from names or start an agent to list commands.
- Plugin slash commands await their contribution owner's typed API.
- Assistant links/images await timeline and file navigation seams; attachment image lightbox stays shared.
- The current editor caps growth at twelve lines; the shell extension will replace this cap with reference viewport capacity when the existing-control seam is approved.

## Validation

Narrow checks through the shared build lane (ordinary while the short lane is saturated); whole native gate and captures once when the complete package is ready. Only fake nodes and an isolated display. The initial independent view tests passed 10/10 after fixing a const-method and borrowed-closure compile error. Expanded catalog tests and Clippy are pending. No whole gate or capture has run yet.

## Resume point

Done: read package and architecture rules, inspect reference and existing code, agree attachment/input ownership, create isolated worktree and target.
In progress: ordered client/plugin/provider catalog and send gesture projection added to the view skeleton; Clippy and expanded tests are still queued. The markdown callback/copy extension is a separate uncompiled draft. No native app integration has landed.
Next: native input integration, command source APIs and exact submit interception; then complete package checks, capture matrix and reviewer handoff. The announced package base is still awaited.

## Integration seams agreed

- Attachment owner keeps `composer/mod.rs`, attachment/@ modules and `session.rs`/its extracted submit module; this package owns `composer/slash.rs`. Preserve its hooked paste callback. File-key dispatch runs before slash handling.
- Add one early `composer_client_command` delegation before submit/queue handling only after the lifecycle callbacks compile. Lifecycle exposes `composer_exit` and `composer_fresh`; archive failures remain visible and do not navigate. This differs from the reference's optimistic close-before-archive and is the lifecycle owner's failure-preservation policy.
- Expanded editor renderer returns an optional element at overlay priority 2; lightbox remains priority 3, tooltip 4. The existing TextField engine is retained. Its drawing split must land before this package edits field presentation.
- Markdown host calls the backward-compatible `chat_markdown_with` options overload. Link/image callbacks preserve find keys and versioned state; source-copy policy is opt-in. The new callback extension and code-copy adapter are uncompiled.
- Keyboard owner supplies typed composer actions; this package exposes a dispatcher after the action API lands. Hardware audio is one shared link engine, with exclusive Dictation/Conversation lease; this package consumes dictation rather than creating another capture engine.
- File/image reads and opening delegate to the Files package's authorized WorkspaceRoot/RelPath navigation and paged reads. Absolute/home paths outside known roots and host-bearing file URLs receive a refusal. HTTP(S) is the only external link class currently parsed; other schemes need an explicit disposition rather than being sent to file lookup.
- Native clipboard currently exposes plain text only: source Markdown and code-aware text are planned; the reference web clipboard's HTML flavor is not exposed by the platform API.

Messaging identity is owned by the timeline protocol package (ADR 0084). Attachment submit ingress allocates its canonical client id once after client-command dispatch and before optimistic presentation or preparation. The shell retains that id; it does not create an independent receipt or message queue.
