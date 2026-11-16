# Shared sidebar FooterIconButton

[Decision Log]
- Purpose: sidebar footer actions need the source28/44 hit area, glyph16/20 and
  matching hover/pressed feedback, plus a real caption/shortcut tooltip.
- Existing assets: announced base14.1 supplies W12's single icon_chrome renderer
  and configured Tooltips controller. command::key_badges already paints the
  source shortcut badge. GPUI's own focused Div maps unmodified Enter/Space
  down/up to ClickEvent::Keyboard with focus-generation admission.
- Alternatives: ordinary Medium32, Toolbar20/32, a copied frame, arbitrary-body
  tooltip or a custom keyboard listener would duplicate or change source policy.
- Choice: one FooterIconButton wrapper delegates chrome to SidebarFooter and
  exposes only owner callback/focus/tooltip seams. GPUI retains native focus and
  activation. A closed caption+display-chords content mode extends the SAME
  Tooltips controller/surface, not a second tooltip, badge renderer or timer.
- Why: source geometry and input bounds remain one element. Caller-prepared
  display chords carry no defaults, registry, route or operation authority.
- Impact: Text/Code/default configured tips and all Toolbar/icon_chrome source
  remain unchanged. Owner callbacks still validate original captured targets.
  Native framework tests do not establish OS screen-reader or effect authority.

Pinned Paseo1c00 references:
- components/left-sidebar.tsx:282–330,387–403,910–927: square frame, accessible
  button name, tooltip top/center8 after300ms, caption14 and optional keys gap8.
- styles/density.ts:15,18–28,39–44: desktop28/compact44, md icon16/20, padding4.
- components/sidebar/sidebar-footer-icon-highlight.tsx:1–25: transparent rest,
  glyph-only hover, pressed pointer-transparent INNER round interactionHighlight.
  The outer target is square so its corners remain clickable.
- components/ui/shortcut.tsx whole: one badge per display chord, p4x2/r6,
  surface3 at.5, normal Meta12, multi-chord wrap/gap4. Omit the whole key child
  when unavailable/unassigned/empty so the caption never gets a phantom gap.

Ownership: coordinator394 assigns the named Footer wrapper to S2. W12 confirms
no pending key-caption equivalent and clears the narrow content seam; V1 records
the existing-Footer reservation. No Toolbar/icon_chrome writer expansion,
arbitrary content callback, new overlay/focus owner or global default change.

Start on announced b54abbdc; all Cargo remains build-machine-only and strictly
serialized after the existing placement check. This plan claims no source,
runtime, first-use, keyboard, accessibility or UI approval. Exact source tests
must cover shared bounds/corners, native pointer/keyboard callback routing,
owner refusal, tooltip timing/touch suppression, empty keys, bounded chord wrap
and stationary same-Place pending/shown caption+keys refresh without restarting
the existing deadline. Existing tooltip ABA/leave/press/drop tests stay intact.
Final bounded review, lead receiving gate and real consumer images remain owed.

Source checkpoint: FooterIconButton delegates directly to the announced
SidebarFooter profile and exposes no frame/color styling. The same element
gets GPUI focusability/tab-stop or the owner's existing FocusHandle. Native
Enter/Space activation remains GPUI's existing ClickEvent path; there is no
new keyboard listener or focus state. Pointer/keyboard callbacks retain the
owner's original effect-admission closure, without a fabricated capability.

Tooltips gains closed KeyCaption/DisplayChords content and on_caption only.
Existing Text/Code append their previous line children unchanged. Caption
uses the same surface and existing key_badges, with bounded wrap in this
content composition; unavailable/unassigned/empty keys omit the entire child.
Configuration, pending state, weak-trigger ownership/deadline logic, all old
profile tests, Toolbar and icon_chrome files are unchanged. Native tests for
caption geometry/wrap, stationary pending/shown props and Footer hit/key/timing
are installed, not executed. Framework/OS accessibility and real consumer
first-use pixels are unverified; primitive254 is not this wrapper's result.

## Quota-stop resume checkpoint

Committed WIP source331a87cb43a3caeb2c743ffb1e06a5be1897cb49 is based on
announced b54abbdc. No Cargo run was submitted for it. All compilation, strict,
runtime, framework/OS accessibility, App dispatcher integration, first-use
pixels and V1/lead receiving approval remain UNVERIFIED. No Usage/Settings
registration exists. W5/G2 must keep waiting; do not consume this as passed.

The only outstanding S2 run is placement025223/3958, not this control. Before
any next-session run inspect that existing state and the Main track resume
file docs/plans/wp-f4-sidebar-resume.md on backend/m12-wp-sidebar. Main receiving
strict and its one runtime replacement remain ahead. Latest announced14.4 is
intentionally unmerged during the stop. Start no new merge, control or Cargo
until resumed. All current source is committed; no local app/fleet/display ran.
