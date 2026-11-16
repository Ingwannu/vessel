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
read/resume cause; this adds no external dependency. The first toolbar
reused the kit's plus, minus and maximize glyphs with tooltips; it was
returned and replaced, see "Lightbox toolbar correction" below.

## Lightbox toolbar correction (2026-11-16)

[Decision Log]
- Purpose: the coordinator's capture comparison returned the lightbox toolbar
  only: boxed white `+ − □ ×` squares at the window's top right, over and
  looking like the window controls.
- Paseo (pinned): `zoomable-viewport/toolbar.tsx` 28-136 and
  `attachment-lightbox.tsx` 59-118. A cluster absolutely placed in the image
  viewport, `top`/`right` spacing[2], `gap` spacing[1]; each action is the
  small icon chrome (20 frame, 32 compact; 14/18 glyph) on `surface2` with a
  `foregroundMuted` icon that turns `foreground` on hover; ZoomIn (disabled at
  the maximum), ZoomOut (at the minimum), Scan (reset, at fit) and the
  lightbox's X; shown on hover, focus within or touch input on the web and
  always on the native app; hidden actions have opacity 0 and no pointer
  events. No tooltips. The modal is a `WindowChromeRootRegion`.
- Existing constraints: the overlay was one full-window occluding element, so
  any top-right cluster landed on the app-drawn window controls and the
  controls could not be used while it was open.
- Alternatives: keep the full-window overlay and move only the cluster down
  (the controls stay covered and unusable); start the whole overlay below the
  title bar (the bar's sidebar, search and update actions would stay live under
  a modal and could move focus out of it); a lightbox-local button style
  (rejected by the return).
- Choice: the overlay root keeps focus and keys only. Its viewport starts at
  `kit::chrome::bar_height()` and carries the pointer handlers, the image and
  the toolbar; a shaded, occluding strip covers the bar left of
  `kit::chrome::controls_corner()` and dismisses on a press like Paseo's
  backdrop; the controls' corner is not covered. The toolbar is
  `ToolbarButton::viewport` (`icon_chrome::Profile::Viewport`) with the new
  ZoomIn, ZoomOut and Scan glyphs and the existing X. Presence: shown while
  the viewport is hovered (`on_hover`, which also fires for layout under a
  still pointer), after Tab or a zoom key until the image closes (Vessel's
  keyboard path to the same actions; Paseo shows the cluster while focus is
  within it), and always in the compact layout (Paseo's native form).
  Each action's slot stops a press from starting a drag or dismissing.
- Why: it is the source chrome and glyph set, the controls stay usable as in
  Paseo's own window-chrome root, and nothing under the modal can take input.
- Differences that remain: the image fits the viewport below the bar rather
  than the whole window; the undimmed controls' corner shows the bar's own
  colour; keyboard `+`/`-`/`0` stay Vessel additions. Paseo's `hitSlop` (4,
  6 compact) is not reproduced: the cluster's spacing-1 gap is narrower than two
  slops, so neighbouring margins overlap. The DOM gives an overlapping press to
  one element; GPUI sends it to every Normal hitbox under the pointer (both
  neighbours would act), and an occluding margin would hide the viewport's own
  hover, which is what shows the cluster (it would flicker off over a button).
  A single-target margin needs a cluster-level hit owner; recorded, not built.
  Paseo's web build ignores `hitSlop` too (react-native-web `Pressable` has
  none), so only its native compact 6 px differs.
- App review round 2: the cluster now sits in Paseo's 200 x 48 toolbar box at
  the viewport's top right (`toolbarDomStyle`, index.web.tsx:303-320, 343-350).
  A press, release or wheel anywhere in it (a gap, beside the cluster, a hidden
  action) stops there, so it never pans, zooms or dismisses; a release there
  also ends any drag. The box does not occlude, so the viewport hover that
  drives presence stays on. Test: presses and wheel in the toolbar box neither
  dismiss nor zoom, while a press outside it beside the image dismisses.
- Coordinator verdict (same day): look PASS. A minimum-scale change to 0.25
  was requested and then withdrawn: Paseo's lightbox renders `ZoomableImage`
  without `minScale`, whose default is 1 (image.tsx:30); 0.25 is only
  `ZoomableViewport`'s own default. The minimum stays fit (1), the maximum 8,
  steps 1.25/0.8; at fit ZoomOut and Scan are disabled. A `viewer` test holds
  the steps, the fit floor and the 8 ceiling.
- Same round, from the app reviewer and the coordinator's ruling: disabled
  actions get no hover/press highlight and no ink change (Paseo's disabled
  `Pressable` has neither), and the kit copies Paseo's pinned style order
  (icon-button-chrome.ts:79-82: hidden, then highlighted, then disabled), so a
  hidden disabled action shows at 0.5 and still takes no pointer: at fit and at
  rest ZoomOut and Scan stay faintly visible.
  Checks on bc65b03a (format-only successor bf4ff7bc2): lightbox, viewer and
  viewport-chrome tests lightbox-toolbar-narrow-20261116T070339Z-bc65b03a 5/0,
  5/0 and 11/0; strict lightbox-toolbar-20261116T070454Z-bf4ff7bc 0; export
  lightbox-toolbar-export-20261116T070341Z-bc65b03a (base12). The withdrawn 0.25
  head's lightbox wheel test failed as expected (it never reached 0.25 in 12
  steps); that head is superseded. Recaptured at 1280 dark: fit at rest (only the
  faint disabled ZoomOut and Scan), fit hovered, and the pointer on the disabled
  Scan (no highlight, no ink change).
- Impact: tooltips, their per-action places and the old `pill_parts` buttons
  are gone from the lightbox state. Pointer positions are mapped into the
  viewport's own space. A new test covers hidden/shown presence, the shaded
  bar's dismissal and Tab staying in the modal.

Checks (build VM, receiving tree = the input packet on 14.2 + the viewport chrome
leaf + this correction): strict lightbox-toolbar-20261116T064040Z-80e0bb24 failed
native-clippy (one unused import, three Pixels subtractions under
`arithmetic_side_effects`); after the fix, lightbox-toolbar-20261116T064501Z-fdbadf78
native-fmt/quality/clippy all 0. Tests lightbox-toolbar-20261116T064712Z-fdbadf78
(`test --locked -p vessel-app -- areas::composer::lightbox`): 5 passed, 0 failed,
the new presence/bar test included. Export lightbox-toolbar-export-20261116T064715Z-fdbadf78
(native-build, app only, shared root fleet base12). No whole native gate ran on
this tree; the input packet's own runtime remains as its resume note records.

Captures (fake fleet, private display; 12 images, all opened): dark and light at
1280 (fit with the pointer over the viewport, fit at rest with the toolbar hidden,
a hovered action, zoomed) and 390 (fit and zoomed, toolbar always shown), plus the
window's maximise control hovered while the lightbox is open at 390 light.

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

`creation::Ticket` is creation area's value from reviewed narrow-green commit3823224d (cherry-picked30420800). The composer retains its own Starting record; `prepare_draft_files(ticket,machine,text)` returns an opaque Copy `PreparedSubmission` with only `ticket()`. `started_draft_files(ticket,card,cx)` and `rejected_draft_files(ticket,cx)` act only on the matching ticket; a machine mismatch preserves the record rather than consuming it. creation area consumed the correlated helpers in staged merged972926c; the obsolete `with_draft_files` adapter and its private `files_start` wrapper are removed. Legacy `started_with_files` remains for existing Event::Started routing. This initial helper detects the byte tray; context preparation/receipt retention follows the canonical message contract.

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
- Choice: `prepare_draft_files_on(ticket,expected_node,machine,text)` retains the admitted node beside the typed Name in Starting/Waiting. Capability, upload, resume and final attachment prompt commands carry that same NodeId and check the captured NodeClient before effects. Wrong-node peer tests assert that no request is sent.
- Why and effects: a missing/rebound address refuses while preserving the retained payload. The creation consumer adopts the explicit `_on` seam and handles typed Target failures; the obsolete label-only wrapper is removed. Successful rename/release adapters and resident connection-lease fencing remain separate pending integration, so a NodeId check is not claimed to prove a current resident/transport epoch.

The native attachment menu now exposes Paste image beside Add image and Upload file, following `composer/index.tsx:1757-1775,2162-2181`. It reads the current clipboard once, uses the same image intake as key paste, captures the target draft, and restores that field's focus. An empty clipboard produces the translated no-image error rather than pasting text. Raster formats unsupported by the current wire image set are explicitly refused pending normalization; they are never mislabeled as PNG. Headless menu tests use a generated2x2 PNG fixture and an ordinary-text clipboard; they do not establish OS clipboard integration.

Suggestions now use the attachment feature's existing Files command and Answer envelopes. Root bus dispatch retains one registration per feature; the common handler stays within its function budget as correlated creation is added. The own attachment dispatcher exhaustively routes root/suggestion, capability, upload, wake and image-read responses. No extra manager or wire operation is introduced.
