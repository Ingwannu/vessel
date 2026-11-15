# Composer attachment parity

Status: implementation in progress; review pending.

## Reference and existing assets

Paseo 0.15.4 is the behavioral reference, read without copying code:

- `packages/app/src/components/attachment-pill.tsx:10-156,164-245`:
  48-point image/label bodies, wrapping trays, hover/touch remove control,
  and a clickable sent-image frame without a remove control.
- `packages/app/src/components/attachment-lightbox.tsx:34-128,152-194`:
  a window overlay, black at 90%, fitted image with 16 padding and
  960 by 640 presentation caps; Escape, close action and outside click
  dismiss. Image errors have a translated message.
- `packages/app/src/components/zoomable-viewport/geometry.ts:54-151` and
  `index.web.tsx:19-20,121-151`: aspect-preserving fit, bounded scale,
  focal-point zoom, clamped panning; 1.25/0.8 toolbar steps. Image minimum
  scale is 1 (`image.tsx:30`), viewport maximum is 8.
- `packages/app/src/composer/index.tsx:1748-1841`: image pick/paste,
  generic-file pick/upload and visible failure; `input/input.tsx:494-516`
  consumes image paste only when images are present.
- `packages/app/src/attachments/selected-file.ts:3-8,20-30` separates
  selected-file identity from asynchronously reading its bytes.

The app already has `composer::{attach,files}`, the kit's attachment,
menu and field paste hooks, and chunked upload in `link::attachments`.
The inherited sent-image draft adds the pure bounded picture cache,
`attachment.read` caller and conversation attachment rows. Existing node
calls implement the entire transport; no new wire method is needed.

## Decision log

- Purpose and intent: attach a file or image, send it, see it in the
  message, and open an image without leaving the conversation.
- Existing implementation and constraints: attachments belong to live
  sessions (ADR 0025). Conversation records carry references, not image
  bytes. The app's shared shell and link entry files may only register.
- Main alternatives: replace upload with a second protocol, preload all
  historical images, or reuse upload and read only displayed images.
  Standard GPUI clipboard/path prompts, image cache and existing kit
  controls cover platform integration; no additional library is needed.
- Choice: modify existing modules. Pure image-cache and viewport rules
  live in `view::composer`, bounded reads in `link::attachments`, and
  tray/sent/lightbox screens in `app::areas::composer`.
- Why: one attachment model and wire path already enforce node scope and
  limits. Visible-image reads keep a long conversation bounded; pure
  viewport tests can prove fit, zoom and panning without a GPU.
- Advantages, costs and effects: no dependency or wire changes; images
  evicted from the cache must be read again. Viewport overlay registration
  is one shell line. New failure states remain typed and translated.

## Deliberate differences and platform limits

- Files upload at Send, before the prompt references them. Paseo uploads
  generic files on selection and sends local images with the prompt.
  Vessel requires a live session and its chunked attachment contract.
- One waiting attachment prompt per session uses the existing tray;
  another Send waits until that upload settles. This avoids replacing
  the pending prompt. Node limits remain authoritative (ADR 0025).
- Historical images are fetched on drawing, two reads at a time, with
  the existing page's count/byte LRU bounds. Paseo keeps preview data in
  its local attachment service. Read failures remain explicit.
- GPUI's standard file prompt cannot filter MIME types. Both attach
  menu entries use it; normal attachment validation still applies.
- File-manager paste, system dialogs and external drops require real
  desktop proof: the capture display lacks a desktop portal and its
  clipboard does not deliver URI lists to GPUI. Clipboard image paste is
  independently exercisable. Source wiring or fake tests are not proof
  of these platform paths.

## Validation and resume point

First working skeleton is implemented: bounded image cache and reads,
tray/sent-frame click wiring, full-window fit/zoom/pan modal with focus
restoration, and ended-session resume-before-upload. File acquisition
runs inspection in the background and captures its destination before
waiting. Actual desktop input proof is still outstanding.

Narrow view test run initially reported 75 passed and one failure: fit
ratio rounding overshot a 960-point presentation cap. The implementation
now fixes the limiting edge exactly; the corrected run is still owed.
An earlier narrow invocation used the root toolchain by mistake and was
refused before compiling; native commands run inside their own workspace.
The native app narrow clippy is pending. No whole gate has run yet.

Next: finish narrow checks and UI regressions, exercise the real UI against isolated fake nodes,
then run one whole native gate and submit one package review. No live
node, real agent, or preview process is involved. A centrally integrated
base will be merged when supplied; no package-local node integration.

The native link directly names the existing RPC error type to retain a
read/resume cause; this adds no external dependency. Toolbar plus, minus
and maximize glyphs reuse the kit's current glyph set instead of adding
Paseo's zoom/scan drawings. The labels and tooltips retain the same
meaning; compact buttons keep the kit's 44-point touch minimum.

## Full attachment package

The package now also covers the reference survey's file/directory
mentions, every user/workspace attachment kind, local byte retention,
workspace-file add/drop, forge picker/URL attachment, and plugin sources.
Additional reference: `packages/app/src/attachments/types.ts:31-137`,
`workspace-file.ts:24-115`, `composer/attachments/submit.ts`,
`attachments/workspace-attachments-store.ts`, and
`utils/file-mention-autocomplete.ts:20-71`.

The next skeleton adds typed context attachment variants and per-draft
stores with suppression/replacement/submit settlement, a validated
workspace-file/line-range contract, and separate mention query and result
state. Context attachments become ordinary typed text content blocks;
image screenshots use the same image-upload path. This does not add a
wire call. Plugin/browser/review producers supply the context contract
from their owning area, rather than duplicating it in the composer.

Missing node contract: the method table has `forge.status` and
`pr.list/get/open/merge/feedback`, but no issue search/lookup. A watched
session's PR list cannot implement issue/PR search or URL lookup.
Picker states may be defined but real lookup waits for the node contract.
`file.find` returns files only; `file.list` can provide directories for a
requested parent, an explicit bounded alternative to a new wire method.
The integration owner has been asked to route these dependencies.

Composer shell/commands belong to another area. Attachment input retains
its existing paste callback; mentions expose a typed DraftKey-based key
handler and menu renderer for that area's wrapper to delegate to before
slash handling. Other areas can open the same lightbox from an already
decoded image, or from a typed node AttachmentRef, without owning modal
state. Local source acquisition remains separate from these open APIs.

Resume point: finish narrow compilation of the attachment/preview core,
then fill context and mention skeletons. The first narrow view run's
rounding failure and clippy corrections are retained in the logs; there
is no full gate or UI approval yet. The package is not complete merely
because image attachments work.

## Integration checkpoint

The preview/context draft is committed to make the required package-base
merge reviewable. Narrow compilation has not passed yet: the first runs
found inherited sort/clamp lints, missing context error documentation,
and a theme constant's misplaced doc comment; these were corrected. The
latest source also adds a typed file-mention state skeleton which still
owes its narrow tests. No full gate or runtime capture is claimed by this
checkpoint. External receipt/create/directory/forge/plugin interfaces are
being supplied by their owners; no duplicate protocol types are created.

### First integrated mention boundary

[Decision Log]
- Purpose: ship the @ path selector against real existing operations while ADR0083 is staged.
- Existing assets: `file.find`, `file.list`, `session.get`, typed `FindQuery`/`RelPath`, kit autocomplete, composer hooks and reveal.
- Alternatives: fake a recursive directory RPC, start an agent, or replace the kit list. None matches authority/ownership.
- Selected: reuse file.find for file ranking, file.list for directories in the requested parent, and session.get for authoritative registered workspace scope. New-chat lookup uses its existing selected project.
- Why: captured draft/node/root/token correlation prevents a delayed result or caret movement from replacing the wrong text. Two query tasks and one root task are bounded and cancel superseded work.
- Effects: until file subsystem's ADR0083 is consumer-ready, directory matching is parent-local substring matching rather than recursive fuzzy matching. Outside registered project/worktree roots suggestions remain hidden. Typed failures remain distinct from an empty successful result. A query waits 180ms; its cache expires after 15s; at most 50 suggestions and 32 cached queries are retained per menu.

### Creation and producer seams (first working skeleton)

`creation::Ticket` is creation area's value from reviewed narrow-green commit3823224d (cherry-picked30420800). The composer retains its own Starting record; `prepare_draft_input(ticket,message,expected_node,machine,text)` returns an opaque Copy `PreparedSubmission` with only `ticket()`. `started_draft_files(ticket,card,cx)` and `rejected_draft_files(ticket,cx)` act only on the matching ticket; a machine mismatch preserves the record rather than consuming it. creation area consumed the correlated helpers in staged merged972926c; the obsolete `with_draft_files` adapter and its private `files_start` wrapper are removed. Legacy `started_with_files` remains for existing Event::Started routing. The helper detects both byte and context trays and freezes them before creation. Canonical message identity is retained, while the identified RPC/receipt activation remains a separate checked dependency.

`composer_attach_context(target,scope,item,window,cx)` checks the destination's registered root and the file pointer's own scope before changing the draft, then focuses that composer. Producer-owned registries remain external: file subsystem owns file actions and root tracking; forge subsystem owns forge identities and lookup; plugin subsystem owns plugin addresses/items/search. The context area renders the existing attachment pills and dialog, keeping focus for close. Forge/plugin safe HTTP(S) links open using the existing app mechanism; other context opens a local text preview. Browser uses the existing Monitor glyph until the shared owner publishes a browser glyph. Review-source reset callbacks and canonical producer adapters are still pending; a generic snapshot does not establish those integrations.

Source status: the30420800 remote narrow app clippy run stopped at a missing must_use on the new view Menu::rows, now corrected. No runtime captures or whole native gate have run. The typed file source is tested through an in-memory fake with parent confinement, empty-vs-failure and result bounds; those new tests are not yet executed. The input key entry is exposed for composer shell area to delegate before slash; it must be consumed before claiming the @ input flow complete.

### Background creation preparation

`Starting` now freezes the exact byte tray at prepare time. A matching completion transfers that snapshot and removes only its captured keys from the new draft; files arriving later remain with the new draft. A stale ticket or different machine cannot take the snapshot. The ticketed path does not call legacy follow/navigation.

The node already exposes `session.conversation`, including `abilities` and `limits`. `link::attachments::capabilities` reads that exact created session independently of selection, with two reader permits, a 30-second overall deadline (including permit/handshake wait), and 250ms handshake checks. The reply retains the creation ticket, node and session. The composer installs it only for the expected pending creation, then validates/uploads that session's tray. Failure retains files and exposes retry on their pills; no capability is inferred from an unrelated timeline. The same bounded pending-submission capacity reserves room for an in-flight creation. These regressions are written against the isolated shell and remain unexecuted pending narrow compilation.

History text now uses the existing aggregate draft budget (1MiB per complete formatted snapshot/submission), rather than the128KiB typed-prompt budget. This permits Paseo's100000-character recent-history wrapper with multibyte Unicode. history import area still owns selection/truncation and the wrapper; `ChatHistory(Payload)` records the semantic kind without requiring a destination root before creation area has selected placement. `composer_seed_history` accepts only the new draft; other producers use the scoped intake. URLs retain a separate8192-byte metadata bound.

Narrow-check correction checkpoint: local view/link Clippy reached the new tests and refused three incorrect RelPath::try_from(String) test constructors plus a nonminimal_bool expression; use the existing FromStr API and is_none_or. Remote aa8 view/link/app Clippy additionally found the new hub missing from the machines test fixture, the common command handler over100lines, wildcard imports and unnamed ignored validation/timeout errors. The mention feature now owns an exhaustive Ask enum and its dispatch behind one common-handler arm; the existing State fixture registers the hub. Workspace-file pill clicks now remain inert as Paseo specifies. These fixes await the next exact-source narrow replay; the old runs remain failed.

The combined creation consumer exposed two further test-only checks: immediate in-memory trait replies now return std::future::ready rather than declaring an await-free async implementation, and the framed peer explicitly retains read_exact's returned byte count. This keeps the fake boundary synchronous in meaning without suppressing Clippy. The c688 app build reached the new rendering for the first time and exposed GPUI import/API/type mismatches; fixes are separate from this small peer-test correction.


The first app compilation now has concrete corrections: import GPUI's image and stateful-input traits, type the pointer event, use the current on_scroll API, propagate optional frame instrumentation, constrain the menu renderer to the enclosing composer module, coerce callback function items through explicit types, and name the20-point pointer control instead of referring to a nonexistent spacing step. Tests retain exact clamp values through bit comparisons and explicitly consume transition/peer results. These are build corrections, not runtime proof.

### Pin the admitted attachment destination

[Decision Log]
- Purpose: a queued creation capability/read/upload must never follow a reused machine-book label to another node.
- Existing assets: the shared `Machine.node` is the authenticated welcome NodeId; the node client retains that same identity. No alias registry or connection epoch is introduced here.
- Alternatives: trust the label when dispatching, or silently choose another node after lookup failure. Both can redirect retained bytes.
- Choice: `prepare_draft_input(ticket,message,expected_node,machine,text)` retains the admitted node beside the typed Name in Starting/Waiting. Capability, upload, resume and final attachment prompt commands carry that same NodeId and check the captured NodeClient before effects. Wrong-node peer tests assert that no request is sent.
- Why and effects: a missing/rebound address refuses while preserving the retained payload. The creation consumer adopts the explicit `_on` seam and handles typed Target failures; the obsolete label-only wrapper is removed. Successful rename/release adapters and resident connection-lease fencing remain separate pending integration, so a NodeId check is not claimed to prove a current resident/transport epoch.

The native attachment menu now exposes Paste image beside Add image and Upload file, following `composer/index.tsx:1757-1775,2162-2181`. It reads the current clipboard once, uses the same image intake as key paste, captures the target draft, and restores that field's focus. An empty clipboard produces the translated no-image error rather than pasting text. Raster formats unsupported by the current wire image set are explicitly refused pending normalization; they are never mislabeled as PNG. Headless menu tests use a generated2x2 PNG fixture and an ordinary-text clipboard; they do not establish OS clipboard integration.

Suggestions now use the attachment feature's existing Files command and Answer envelopes. Root bus dispatch retains one registration per feature; the common handler stays within its function budget as correlated creation is added. The own attachment dispatcher exhaustively routes root/suggestion, capability, upload, wake and image-read responses. No extra manager or wire operation is introduced.

### Shared file intake and routing

The file area's narrow-checked scope/search adapter is merged from0df7abd9. Context intake first uses its cached exact machine/session workspace scope, then the composer's correlated root lookup. `composer_destination` is the single explicit route-to-draft bridge until the resident owner publishes scoped contexts. Native composer cards accept typed `WorkspaceFile` drops as well as external file paths; drop handlers retain the destination captured while drawing. The file area still owns drag sources and Add to chat actions. Intake refusal uses one exhaustive translated presentation; success alone focuses the composer. The slash owner's input wrapper extraction is adopted for the file budget, preserving mention-before-slash and clipboard hook order.

### Context-only creation and retained delivery

[Decision Log]
- Purpose: history/review/file context without byte uploads must enter the same first-input pipeline and survive background creation or a missing delivery reply.
- Existing assets: creation tickets, ContextTray revision snapshots, the existing session.prompt operation, shared Files envelopes and the existing typed Delivered result. The canonical keyed timeline operation is still pending its owner's checked seam; it is not called here.
- Alternatives: append mutable live tray text at completion, send context separately, or clear the pending owner before RPC. Each can lose, substitute or duplicate context.
- Selected: prepare locks one immutable context Submission alongside the byte snapshot. A matching background completion moves only captured revisions into the new session; newer draft revisions stay behind. Rejection restores suppression only for the matching ticket. Existing send/render guards now treat context as an attachment, so context-only input is Deferred.
- Why and effects: draft_has_payload checks exact text bytes (including whitespace), persisted New text, any retained context including suppressed entries, bytes, and pending creation. Seed intake refuses rather than overwriting those resources. The immutable final WithFiles is retained by Arc through dispatch and response. Arc pointer identity is only local operation ownership, never a wire idempotency key. Pre-transport refusal and transport uncertainty remain typed. A timed-out/failed call stays retained and cannot automatically reupload or resend; adopting the canonical ClientMessageId later will preserve that same payload. Browser demo explicitly refuses this native path rather than pretending to upload context/files.
- Current limit: legacy uncertain delivery has no safe retry/reconciliation UI yet, so the retained composer remains locked and reports the condition. This is honest preservation, not canonical receipt parity. Resident Hold/connection lease integration, managed local-byte persistence and source-specific reset callbacks remain pending.

### Managed local byte-store foundation

[Decision Log]
- Purpose: copying bytes before draft intake separates the attachment snapshot from later edits/deletion of a picked source, and lets drafts survive restart without serializing large blobs.
- Existing assets: app state path resolution, owner-only directory/file creation in link::prefs, std streaming I/O, validated protocol filename/MIME, serde and the existing draft key. No dependency or parallel node attachment identity is added.
- Alternatives: retain source paths indefinitely, keep every pasted image only in RAM, add IndexedDB to the native process, or delete by filename/age alone. These lose immutable bytes or cannot protect active consumers.
- Selected foundation: a confined LocalKey, nonzero LocalSize bounded by the node's generic-file limit, LocalRecord without arbitrary persisted paths, and a private native Store. Writes and GC serialize through one shared barrier; create_new prevents overwrite, completed writes sync, copies check source size/mtime and exact byte count. A bounded validated atomic manifest and live Arc leases protect bytes from GC, with a five-minute grace for unreferenced saves. Total files/bytes are bounded at512/512MiB. Unknown manifests refuse both overwrite and collection.
- Why and effects: 0700 directories and0600 new files preserve the actual user's access boundary; existing permissive roots/symlink substitutions refuse instead of broad chmod. Explicit test roots avoid reading operator state or changing environment. No browser byte-store parity is claimed by this native foundation. Draft intake/restore/persistence and lightbox lease wiring follow separately; source-only storage code is not marked built or passed before its narrow checks execute.

The native startup now restores byte metadata through an explicit composer hook; isolated `with_drafts` tests do not resolve default storage. Pick/drop inspection copies a bounded immutable managed file on the background executor; paste saves its bytes there before adding the pill. Pending acquisition blocks session submission and creation preparation. Mutations debounce manifest writes by200ms, with a dirty-during-write transition that prevents older snapshots overwriting newer ones. Restored unconfirmed files block fresh submission rather than pretending to be unsent; no automatic replay occurs. This is not an atomic cross-process receipt guarantee: canonical message IDs/receipts and the resident registry remain the shared owners of that integration.

An open local-image lightbox holds its own Arc lease, so removing its original pill cannot make GC unlink the image while the modal still uses its path. Pending sends also retain local-file leases independently of tray LRU presentation. The persistent manifest marks pending byte owners as unconfirmed; successfully delivered byte copies can be collected after grace because authoritative queued/timeline attachments are then re-readable from the node. That server-backed retention differs from Paseo's client-only queue byte retention. Recovery of uncertain final text/context/refs still awaits the canonical receipt persistence seam; byte restore never initiates transport.

### Canonical message identity before attachment preparation

The separately checked value-only timeline commits96ed8b0b/0be6f4b1 provide ClientMessageId::random without activating any wire method. Starting, Waiting and immutable WithFiles now retain that exact canonical type. The additive prepare_draft_input(ticket,message,node,machine,text) lets creation allocate its first-input identity once; the creation consumer now calls it directly, and the old implicit-identity wrapper is removed. Existing-session ingress allocates before any attachment preparation and retains the id across upload/final-payload retry. This is preparation ownership only: legacy session.prompt still carries no wire key and no canonical receipt is fabricated from Delivered. The exact-ID timeline and identified plain-prompt ingress cut over together only after the shared runtime/echo/NodeClient seam is checked.

The standalone local-store strict link check on5f714fbc reached both lib and test targets and refused one Rust1.98 readability lint: Duration::from_secs(5*60). The store now uses Duration::from_mins(5). No storage runtime test was executed by that Clippy run; replay on the changed source remains required.

### Read-only mention-key admission

The global shortcut owner can query composer_file_menu_owns_key(target,key,cx) without moving the selection, dismissing a menu or editing input. The same predicate gates the actual file-key handler after observation, so Escape/Tab are claimed only for the current draft/token/scope (including loading/root-error Escape). A scope change clears previous query rows rather than showing another workspace's cached candidates, and an unscoped invalid token stays hidden. The creation owner's one-file sidebar lifetime fix8c932dc10 is adopted, and the now-unused PreparedSubmission reexport is removed; the opaque return value remains owned by its module.

The first exact app4bd strict replay reached all new app code and reported17 lints/errors (not a passing run). Corrections narrow file inspection visibility, borrow capability answers, collapse scope/link conditions, move menu constants before statements, split suggestion listing from drawing, and move file-envelope refusal into its own leaf. Wheel sign conversion now belongs to the existing pure viewport geometry. Method callbacks, explicit CardDetail default and test statement terminators follow the current strict toolchain. Two creation-owner style fixes are requested separately; no size allowance or lint suppression is raised.

The mention regressions cover scope-switch candidate isolation, invalid input outside any registered scope, and side-effect-free Escape/Tab admission (unchanged token, selection, phase and root lookup). The creation owner's Arc-backed identity fix86956f78e is adopted without minting another message ID. The isolated context bridge20ea88e2 has strict view all-target Clippy green; its focused context tests are separate pending evidence. Missing local build fonts were supplied from the existing pinned geist1.9.2 package into ignored assets with hashes recorded locally; no kit source or shared binary is modified.

The current package line2e95b7d31 (including the shared connection preflight) is merged at a natural source checkpoint; no node head is merged directly. Sidebar row labels now accept owned SharedString through the creation owner's57825b3a1 fix. The post-font local check on02d1 failed on the old Arc identity moves, sidebar argument type and test CardDetail path; those are corrected before VM-only replay. Native assets are supplied from pinned Geist1.9.2 without rebuilding any shared root binaries.

Local manifests now reject more distinct drafts than the actual tray capacity. Until the resident owner provides live protected resource access, adding a new tray at full capacity while any submission is pending refuses visibly instead of evicting upload state. This conservative admission keeps the existing owner and bound; it does not introduce another resident manager.


### Workspace draft attachment cutover (staged)

Canonical DraftKey now includes node/workspace/layout DraftId. The area reuses W3's exact field and scope accessors, preserves per-key context/managed-byte manifests, and retains creation payloads by key plus local Ticket. Captured pill and autocomplete callbacks extend existing kit controls rather than duplicating their renderers. The scoped seed refuses occupied fields, suppressed context and pending acquisition. The registry's root remains unavailable until its owner resolves it; no cwd or global-session substitute is used.

Verification: isolated context value head20ea88e2 passed strict view Clippy and7 focused tests on the VM. Full source59a22f16 failed9 strict style/test diagnostics; owner fixes are committed and a new immutable combined check is queued. Workspace selection, creation consumer, full package gate, captures, OS file dialogs/paste/drop and review remain unverified. Boundaries checks must run on the VM because they invoke Cargo metadata; no new local Cargo is used.

### Canonical plugin context values

The retained context variant now directly contains the reviewed protocol plugin ContextAttachment. Its opaque logical key is compared through the producer's key() method, retaining source node/plugin/contribution/item and the captured installation generation; display labels never stand in for identity. A helper restart does not replace a selected snapshot or authorize a new invoke. Intake checks the captured source node against the explicit workspace node or verified connected machine before insertion. Sending uses the validated retained text only. The generic PluginResource placeholder is removed. Plugin picker/search/runtime calls remain unavailable until their owner activates the canonical producer; this is not an invented source registry or wire call.

### Bounded image intake and preview normalization

[Decision Log]
- Purpose: native clipboard and selected images include BMP/TIFF, and tiny compressed or animated images must not reach the UI decoder with unrestricted dimensions/frames.
- Existing assets: GPUI and the kit already link image0.25.10 with PNG/JPEG/GIF/WebP/BMP/TIFF codecs. Paseo hooks/picked-image-normalizer.ts preserves JPEG/PNG and exports other picked formats as PNG. No new library/version or protocol is needed.
- Choice: a byte-only kit helper inspects the actual signature, applies strict4096 side limits and a64MiB decoder allocation budget, preserves static PNG/JPEG, and exports the first frame of other supported rasters/APNG as RGBA PNG. Its encoder writer cannot exceed the node image byte limit. App acquisition runs this on the background executor before retaining the private managed copy and publishing its pill. Selected image paths are read only from the already frozen managed source in production. Remote image previews remain Reading until the same conversion completes, preserving the existing two-reader budget.
- Tradeoffs: animation becomes a static preview and images over the dimension budget are visibly refused; no silent resize or arbitrary URL decoding is introduced. Codec allocation limits include a documented best-effort component; strict side limits remain separately enforced. This is not proof of a global GPU/decoded-cache memory bound, and W13's separate file renderer has not adopted it yet.

Tests cover static PNG byte preservation, BMP conversion, invalid/oversized input, encoder output overflow and dimension refusal. The actual clipboard regression now waits for background validation and checks Send remains blocked until intake ends. VM execution and OS clipboard/dialog/drop captures are still owed.


### Original admission across attachment effects

[Decision Log]
- Purpose: a verified node identity alone cannot distinguish a same-node reconnect or changed credentials under the same book name.
- Existing assets: G4 owns ConnectionContext, the admission allocator, SelectedConnection and its before-poll/await/post-result fences. B1 must consume that boundary rather than add a connection registry. G4 permitted derived Hash only on the three immutable context leaves; a set regression distinguishes reconnects and aliases without assuming different numeric hashes.
- Alternatives: re-resolve a book alias at each upload chunk, pin only NodeId, expose a raw NodeClient, or invent another epoch. Each can substitute authority after preparation.
- Choice: Starting/Waiting, upload/final-prompt/resume/capability envelopes, sent-preview keys and file-suggestion scopes retain the original canonical ConnectionContext. Typed existing Unary requests run only through SelectedConnection. Creation can pass its already captured context through prepare_draft_input_on; the compatibility wrapper captures synchronously before freezing input. WorkspaceInput takes the same context instead of duplicate node/name inputs.
- Why and effects: stale and same-node replacement admissions cannot issue requests or populate the replacement preview/cache slot. B1 projects RequestFailure once into a closed RequestProblem; peer diagnostics never become UI/log strings. Resume uncertainty retains the input and does not automatically repeat the effect. Capability reads and all upload chunks keep the same context instead of hydrating fresh authority on completion.
- Limits: this is a consumer of the shared guarded Unary foundation, not a resident/follow/Delivery authority proof. The global selected timeline remains the product resource owner until the coordinated registry cutover. Legacy session.prompt remains unkeyed; uncertain final payloads stay locked, and no canonical receipt is fabricated. File/directory suggestions still use existing file.find/file.list pending the canonical directory-suggestion producer.
- Verification owed: changed-source strict Clippy and targeted tests on the build VM, then the package whole native gate and inspected captures. Tests require zero peer frames for retired and same-node replacement admissions, preserve live transport while observing a guarded read, and check that synthetic peer diagnostics cannot escape projection. Source tests are not runtime evidence until they execute.

### Late upload/resume reply correlation

Upload and resume envelopes now carry the same existing canonical ClientMessageId allocated before preparation. The link echoes it on every progress/final/unsent/demo reply; no new identifier or wire method is introduced. The owner accepts progress only for that pending submission in Preparing phase and an actively uploading item. A reused tray ItemKey cannot let an old task fail or complete a newer prompt, and a late resume refusal cannot remove its pending owner. The regression sends an old id against a new uploading file and resume, then requires the matching id alone to change the file. This local correlation is not a substitute for the future resident OperationTicket/connection lease or the inactive keyed server receipt contract.


Image normalization evidence: fixed-source run wp-composer-image-kit-20261115T121330Z-6987bc0f executed both codec/policy tests: 2 passed, 0 failed, 0 ignored. The matching strict union run wp-composer-image-union-20261115T121331Z-6987bc0f stopped in kit on wildcard_enum_match_arm before proving App: preserved-format policy now uses explicit equality checks after the existing six-codec admission gate. The changed source still needs strict verification; these two tests do not establish OS paste/drop or global decoder memory behavior.


Capability results remain bound to the same original admission even after becoming Ready; the pending owner must match both its creation ticket and ConnectionContext. Successful settlement removes that capability entry so a later send cannot inherit an obsolete creation read. Read-only shortcut admission and arriving @ rows also reject a same-node reconnect before rendering/consuming its old popup. This is local cache/operation correlation, not additional transport authority.

Creation now calls prepare_draft_input_on with its original ConnectionContext directly. The name/node compatibility wrapper is literal cfg(test) for the two recovered-intake refusal fixtures; no product caller can rehydrate an admission through it.


## Current candidate and verification boundary

The native candidate now consumes package base8 and the creation owner's real scoped Chat renderer, captured ConnectionContext first-input caller, canonical Chat/Terminal mode and scoped edit/seed persistence callbacks. The B1-owned image/file intake, private managed store, retained upload/final-payload pipeline, context tray, sent-image viewer and file/directory fallback suggestions are wired. It is a candidate, not a parity-pass claim.

Fixed-source VM evidence already executed: isolated context values7/0, image codec/policy2/0, and attachment link tests16/0. The latter includes actual-user private storage/reopen/GC-manifest and in-memory preview-peer tests on0ece77fdd; it predates the ConnectionContext migration and cannot prove that newer boundary. Full candidate strict/whole native checks, inspected captures and V4 review remain owed.

Dependency gaps remain explicit: canonical directory suggestions (parent-local file.list fallback is intentional); forge issue/PR lookup, picker and URL auto-attach; plugin source search/runtime picker; source-owner add-to-chat/browser/review adapters; B2's checked fullscreen/client-command consumer; W23 runtime/echo activation and exact-ID timeline ingress; live resident/follow resources and queued-result Delivery integration. B1 does not invent these calls, a second registry, successful receipts or a Workspace-to-New fallback. Uncertain final payload recovery retains bytes but does not yet durably restore all text/context/upload references after restart.

OS file dialogs, external file paste and native drops under Xvfb are specifically unproven; clipboard-image, attachment send/read, zoom/focus and responsive captures must be inspected before being reported. Root fixture binaries stay read-only and protocol23; captures use only a private fake fleet and authenticated display. No service permissions or production state are changed.

The isolated shared input check wp-composer-shared-input-20261115T132829Z-d5200d04 reached Link and refused one redundant closure and three test-fixture unwrap lints. The fixture module is literal cfg(test); its single constructor now records why the fixed parsed address, fresh allocator and just-inserted admission are infallible for that test. No product panic or crate-wide lint exception is added. App strict compilation remains unproved until the corrected packet runs.

The earlier composer-test source0ece77fdd did not execute tests: it stopped on missing creation fixture/translation prerequisites and an older context preview extraction. Those prerequisites and extraction are present in the current candidate. Its unused attachment import, duplicate HashMap/WorkspaceInput qualifications and clipboard-test context name are now also corrected; the recorded failed source remains failed.

Corrected shared-input run wp-composer-shared-input-20261115T133737Z-c71bd770 reached App and failed on existing callback style/import/fixture diagnostics rather than proving the union. Remaining B1 callback semicolons, local helper qualification, simple empty-state predicates and one fulfilled reexport lint expectation are corrected without changing submission semantics. Portable baseline also needs the earlier scoped callback coercion, local-store style fixes and explicit fixture epoch fields already present in the full candidate. W3 owns its separate form/workspace documentation/control-flow corrections. Further checks use the ordinary/short lanes, not another priority exception.

The candidate adopts the creation owner's exact4331a7e95 scoped-workspace documentation/history-if and fixture invariant changes. Its unrelated form controls and absent top-level seed test file are not introduced merely to consume lint fixes. The input remains owned by the same scoped editor and shared persistence API.

The creation owner's actual f0b2822a6 strict run found its four newly added fixture expectations unfulfilled; the candidate takes its bd908d639 removal only for the existing scoped-workspace test file. No absent seed test file is resurrected. The remaining B1 test/import diagnostics are fixed without relaxing product lints; the old failed runs remain failed.

The candidate adopts the creation owner's conservative ddc62f925 legacy failure boundary without unrelated form/seed ancestors: generic Unavailable and SpawnFailed can occur after a child process launched, so both registration and first-input stages retain OutcomeUnknown. The existing duplex matrix now covers all three uncertain generic codes (including Internal), keeping payload/id rather than offering a duplicate create/prompt. This changes no public API or wire schema and does not establish a durable receipt.

Full-candidate strict run wp-composer-20261115T140056Z-9505ebb5 reached App and failed on a misplaced plugin-open branch in the context close handler. Closing now only releases the preview and restores its previous editor focus; opening retains the sole validated external-link action. A GPUI regression keeps the retained source context while dismissing the separate preview, without invoking a real URL opener. The pending-preview callback also uses the established unit-return statement style. No whole native/export is started from the failed candidate.

The creation owner's actual shared309 execution reached App117 passed/1 failed/2 ignored; it stopped before Link tests. The failure was its NewWorkspace navigation row retaining the old Draft-only sidebar focus condition. This candidate carries that NAV change and takes the owner's exact8d8a4b623 focus-anchor/comment correction, preserving the same handle and assertions. That partial App result is not a whole/native/Link execution pass.


The candidate strict run wp-composer-20261115T150635Z-e1e3c729 reached App and rejected its unused typed image-error payload. Intake now consumes the error kind: encoded size refusal uses the existing size message, while unsupported or undecodable images use the existing format message. This preserves the typed failure instead of discarding its reason or adding a lint exception. The next candidate includes the announced native-green package base c3429952; the draft merge preserves the correlated creation and extracted renderer rather than resurrecting the removed uncorrelated sender. No whole gate or capture had started before this base update.

The c3429952 preference-command change boxes Prefs. Three extracted draft/file-tree call sites do not exist on that base's original paths, so the union wraps their same existing preference snapshots in Box::new. This is a transport representation migration only; persistence ownership, values and scheduling stay with the existing owners.
