# Composer shell and commands

Status (2026-11-15 13:06 UTC): incomplete B2 package. App integration is source-stage;
no whole gate, capture matrix or final reviewer approval exists. Packagebase7 d18bb794 (including the dd13 lock correction) is adopted. Node and native work remain separate.

## Decision and existing assets

[Decision Log]
- Purpose and intent: port the reference composer shell, command selection and assistant interactions while retaining the existing editing engine.
- Existing assets and constraints: `view/composer` owns drafts/recall/slash application, `view/session` and preferences own legacy delivery, kit TextField owns selection/IME/undo, and W6 owns attachments/mentions/lightbox. W12 owns the sole resident registry; W21 owns archive/fresh lifecycle; W23 owns keyed acceptance and atomic delivery.
- Alternatives: another editor, another message queue/identity store, per-feature whitespace matching, label-inferred native execution, or new UI dependencies.
- Choice: extend existing controls and typed values. Use the same field Entity and one submit subscription, the canonical matcher/alphabetical helpers, and owner-provided lifecycle/file/receipt contracts. No dependency is added.
- Why: text, selection, focus, IME and undo must survive presentation changes; stale callbacks must not act on another owner. Parsing and presentation cannot grant workspace or transport authority.
- Advantages, costs and effects: existing controls remain shared. Unavailable owner contracts remain explicit; several source-stage adapters cannot become product paths until coordinated cutover and validation.

## Paseo evidence

Reference paths below are relative to the Paseo 0.15.4 `packages/` tree.

| Source | Required behavior |
|---|---|
| `app/src/composer/input-mode.ts:1-47` | Chat enables attachments/voice/autocomplete/controls; terminal disables them and uses the code face. |
| `app/src/composer/input/input.tsx:216-220,1046-1049,1205-1224` | Desktop/compact height limits; measured-line expansion; editable compact only; sendable-content guard. |
| `app/src/composer/input/presentation.tsx:109-124,145-162` | Glyph-only fullscreen controls within the text surface, retained editor and surface1. |
| `app/src/composer/index.tsx:200-207` | Preserve-and-lock loading differs from ordinary clear behavior. |
| `app/src/composer/input/state.ts:9-15,140-190,254-269` | Permission override, alternate/default send, Shift newline, retained selection/replacement. |
| `app/src/client-slash-commands/index.ts:19-87` | Exact attachment-free `/exit` (quit/q), `/clear` (new), immediate selection. |
| `app/src/plugins/client-slash-commands/model.ts:1-65` | Built-in/plugin/provider precedence and one argument trim at invocation. |
| `app/src/hooks/use-agent-autocomplete.ts:231-244` | Inline skills come from provider entries directly, independent of root collisions. |
| `app/src/composer/recall.ts` | Loaded nonempty prompts, consecutive duplicate suppression and selection-aware recall. |
| `app/src/assistant-file-links/{parse,resolver}.ts` | Typed path/location classification, suffix lookup and owner navigation. |
| `app/src/assistant-image/acquisition-cache.ts:70-86` | Single flight, retained consumers, soft LRU capacity500; do not evict in-use entries. |
| `app/src/assistant-selection-copy/content.web.ts:155-199` | Inside-code drags copy raw code; crossing semantic boundaries retains Markdown; preserve indentation and remove trailing line breaks. |
| `server/src/server/agent/providers/codex-app-server-agent.ts:226-242,4125-4141,5143-5159,5166-5287` | Native root parsing, whole goal arguments, command catalog and out-of-band acceptance. |

## Modules and ownership

| Layer | Responsibility |
|---|---|
| `view/composer_shell` | Pure surface policy, ordered command catalog, gestures, validated file references/recognition, retained image-cache model. |
| Existing `view/composer` | Draft/recall/replacement rules; attachment state stays with W6. |
| `kit/field/composer_layout.rs`, `send.rs`, `submit_press.rs` | Window-aware presentation over the existing field, measured wraps, compact ButtonOnly and synchronous press metadata. Readonly remains W4's setter. |
| `kit/chat_markdown` and the existing vendored TextView | Backward-compatible options/actions over the same renderer, source positions, find state and selection aggregator. |
| `app/areas/composer_shell` | Current surface orchestration; staged resident policy/retirement adapter. |
| `app/areas/composer/slash` | Existing hook wrappers, file-key-before-slash ordering, typed menus, captured-owner checks and early client-command dispatch. |
| `app/areas/session/resource.rs` | SessionView construction accepts one retained field; parent session.rs shrinks604→570. Open/Pending/lifecycle/submit bodies are unchanged. |
| Root `proto::composer_commands` / `native_commands` | Catalog values, optional kind/native metadata, protocol projection, closed native intents/status. Runtime catalog/receipt/transport stay separate owners. |
| Fluent EN/KO | Shell/menu text and accessible fullscreen labels. |

Rust/GPUI, the existing kit, Fluent, Tokio and vessel-client remain the stack.
No alternate renderer, input engine, message id allocator, queue, identity registry
or hardware microphone engine is introduced.

## Current source behavior

The compact host uses the same TextField Entity in exactly one presentation slot.
It expands above two measured wrapped lines, applies viewport-aware growth, uses
ButtonOnly on compact layouts and restores the configured desktop shortcut on
window-aware rendering. Escape passes through the IME-aware hooks before collapse;
menus are suppressed in fullscreen. The glyph is over the input instead of adding
a row. Fullscreen is deferred priority2 below W6 images at3 and tooltips at4.
The shared form-style Enter bubble stop prevents native fallback inserting a second
newline after the sole engine/listener has already submitted or inserted one.

Early client dispatch precedes optimistic echo and attachment preparation. Any
files or retained context suppress client execution. W21's callbacks wait for an
archive acknowledgement before close/fresh; failures retain the owner and draft.
Menu clicks verify the drawn target/text/caret/name before acting. Root names and
aliases are reserved before plugins/providers; draft menus hide client rows.
Only explicit provider Skill metadata is eligible inline.

Client/plugin parsing and token boundaries reuse the existing
`matcher::whitespace` predicate (one agreed crate-only visibility change). Its
body/default matching is unchanged: JavaScript includes BOM and excludes NEL.
The name tie uses G3's canonical alphabetical comparator. No second helper is added.

Submit factories now accept a captured immutable Fn boxed once in the existing
Handlers. Base7’s canonical Callbacks::Identified/FieldId/with_source contract remains unchanged; capture widening applies only to Callbacks::Plain. SubmitPress is available only during that field's synchronous callback,
with nested scope restoration and clearing afterward. Buttons/asynchronous work
must not read later global modifiers. K's Legacy/Windowed and with_events union
must preserve this same invocation; there is no second subscription.

## Resident preparation: test-only

W12's exact checked staged producer1d93 is adopted with its checked W1 search
constructor prerequisite. The literal cfg(test) registry/resource boundaries remain.
`SessionView::new_scoped` also remains literal cfg(test): it captures one context,
refreshes only the same resident+claim for local submit, and requires current
Visible+Focused. It never resolves a selected route or refreshes network authority.
The existing product global resources remain the sole product path.

The staged bundle owns ResidentComposer, not another map. It records W12-issued
binding/ticket pairs under the canonical64-operation bound. W6 retains message ids,
payloads and optimistic rows. Retirement marks only matching composer operations
Uncertain; it never clears text or retries. Settlement belongs inside the sole
registry's exact ticket consumption. Unknown-effect reconciliation still requires
W23/W6; an old ticket is not readmitted.

Clean snapshots refuse dirty text including whitespace, selection, IME, active
focus, fullscreen/audio overlays and pending/unknown effects. Only empty inactive
input can transfer its Surface policy. Undo history of a clean inactive empty
editor is not serialized. This does not claim live hydration, follow admission,
IME handling, permissions forms or transport safety.

## Markdown and assistant interactions

`chat_markdown_with` preserves the old entry point and keyed/versioned state.
Options carry find marks, typed authored/inferred links, authoritative image
resolution, optional image clicks and opt-in copy policy. It calls the existing
Base renderer directly and refines the Component-installed theme, rather than
creating another control. Authored text links win over inferred recognition. An installed image-activation callback consumes the image click, including linked images; without it the existing enclosing-link behavior remains.

Opt-in recognition validates UTF8 byte ranges and skips authored links/fences.
Recognized code/emphasis/link marks reuse the existing highlight sweep so source
copy does not emit overlapping bytes twice. Text/source segments and find offsets
remain the engine's. Source copy ignores inferred-link syntax.

Code-aware copy queries the existing parsed selection instead of scanning fences
again. Mixed prose and multiple regions retain Markdown; whole-view Select All
retains message boundaries. Inside-code drag text preserves indentation and strips
trailing line breaks. A single participant never replaces the window aggregator's
multi-message selection. The obsolete duplicate source cache is removed.

The parser distinguishes authored href decoding from literal inferred percent
bytes and validates line suffixes, file hosts and external HTTP(S). These are
values, not authority. Actual file opening, suffix lookup and paged image reads
await W13's authorized root/opener. W6 owns the decoded/sent lightbox. The pure
image cache is single-flight, retains consumers, fences stale completion and uses
the reference soft500-entry bound; it is not yet an app acquisition pipeline.
Native clipboard exposes text/image/files but no text/html flavor, so HTML copy
parity is explicitly unavailable.

## Remaining work and deliberate differences

- W12's coordinated product resource cutover, W3 scoped field/mode getters and W6 Workspace-keyed card/intake must be consumed without Workspace-to-New or cwd authority fallback. W3 confirms Chat→Chat and BlankTerminal/Profile→Terminal, including readonly resolved profiles.
- Terminal mode/font and preserve-and-lock values exist; their new workspace hosts are not fully wired.
- W3 confirms no checked authorized DraftCommandConfig root/config snapshot. The draft RPC stays gatedNone; no agent is started just to fabricate a catalog.
- The real protocol still masks command metadata under COMPOSER_COMMANDS=None. Native metadata is node-side staged source; actual current-run W22 offers and W23 acceptance must both be checked before advertisement. Persisted command reports/cache never authorize native execution.
- W23 owns keyed SubmitPrompt/CommandAccepted and atomic SteerOrInterrupt. No new UI read-then-act fallback or separate command queue is added. Vessel's existing queue remains node-owned/durable, unlike Paseo's client queue.
- Modified-Enter metadata exists; actual alternate-send/keyed-delivery integration remains pending. Fullscreen collapse must also cover all shared submit entry points, not only its own button/client guard.
- Plugin descriptors/invocation await W16; shortcut/command-center dispatch and source contributions await W22b. No second listener/palette is installed.
- Timeline's one Markdown host hook, authorized file/image acquisition and context-only fullscreen sending await their checked consumers.
- B3 dictation waits for node wiring and W19's one CapturePurpose lease; no second capture engine is added.

## Evidence and next validation

Evidence is scoped to the exact source cited; it is not a package/UI pass.

| Source/check | Actual result |
|---|---|
| Old840 native kit+view strict Clippy; view models | Clippy0, view23/0. |
| Later unchanged parser/cache/sending/surface view slice | Actual27/0; later BOM/NEL changes unverified. |
| Node d9 combined catalog/probes | Strict5crates0; cache tests4/0. Earlier command units14/0 and fake Codex/options29/0. |
| Node67 gate44 union | Strict5crates0, run `wp-composer-shell-node-narrow-20261115T114725Z-67c5eb77`. Later native metadata is not covered. |
| Native3e all-kit |137passed/2failed: duplicate inferred-code source and compact double newline. Both source corrections are prepared; failed evidence is retained. |
| Native519 | Lock-file refusal before compilation; exact lead dd13 fix adopted. |
| Nativec151 strict / kit | Strict failed lifecycle/base6 union constructor fields; kit compile failed missing App argument to focus. Corrected atd4ca; no pass inferred. |
| Independent native valuesdd06 +TS |4/0/0; strict failed doc, integer-division fixture and generated-TS name collision. Corrected b7b4 successor check queued; JSON unchanged. |
| Node67 TS generation | Compile failed missing web Method arm before generation; empty artifacts. Corrected policy and metadata rerun queued. |

Current d4ca strict4crate and kit+app library tests are registered on the build VM.
Later shared-value carriers have their own pending checks. No local Cargo/Rustc is
started. Static size/boundary/token checks pass for the source prepared so far.

After required adapters are complete: one whole native gate, root gate for root
changes, regenerated TS committed from VM artifacts, isolated fake-fleet captures
at1280x900/390x844 (800x600 where layout matters), dark/light EN/KO, then the agreed
V3/V4 review. Never use live agents, live preview ports, credentials or real browser
openers in tests. Capture and review evidence remain outstanding.

The additive `composer_command_menu_owns_key(DraftKey, ComposerKey, &App)` query is a read-only local-key ownership seam for the sole shortcut interceptor. It recomputes the actual menu for the exact current field; empty results, other draft/session keys, read-only fields and covered surfaces do not claim keys. Its shared `Menu::takes_key` predicate is also used by the actual slash handler, so inline Escape passes while root-menu Escape closes the menu. It grants no active-composer identity or network authority and does not cover W6’s file menu or audio. A real GPUI regression checks repeated queries preserve text/selection and the subsequent Escape agrees with the handler. The new source remains VM-unverified; existing d4ca checks cover the preceding source only.

Assistant file-token normalization also reuses the existing matcher’s exact JavaScript whitespace predicate (`assistant-file-links/parse.ts:58-68,106-119`). BOM is stripped as in the reference; NEL remains a control and is refused rather than erased into a valid file target. Classification and source-line path suffixes share this normalization, without introducing a second whitespace helper, parser, filesystem read or dependency. The regression covers authored/inferred paths and uppercase HTTP scheme normalization; the successor source awaits VM checks.

Code copy now shares the engine’s pure `trim_code_line_breaks` helper between code-only retained selection and the existing code-copy button. It matches Paseo `assistant-selection-copy/markup.ts:12-19` and `components/highlighted-code-block.tsx:95`, removing final LF/CRLF plus following ASCII indentation while preserving the last code line’s own spaces, leading indentation, interior newlines, NBSP and lone CR. The prior query removed only CR/LF and could leave a selected newline followed by spaces. Table regressions cover those boundaries; actual existing Copy handling and the window selection aggregator are retained. New source still awaits VM checks.

Base7 union preserves the shared FieldId producer and its unchanged identity regression. The one existing `Callbacks::submit` invocation is wrapped by the same field-local SubmitPress guard. Plain submit factories retain captured Fn support; identified callbacks retain their exact signatures, engine ids and source checks. No second listener is installed. Existing queued d4ca checks remain prior-source checks, and no new base7 union compilation result is claimed.
