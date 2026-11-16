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
