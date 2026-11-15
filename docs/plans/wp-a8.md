# Attachments in the native conversation

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
