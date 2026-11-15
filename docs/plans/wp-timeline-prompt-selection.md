# User prompt selection preparation

Status: source draft only. No strict/runtime/UI or adoption pass; the new caller
is held outside the main A7 tree until the shared foreground-range requirement
is resolved. No node, network action or new host capability is added.

[Decision Log — 수정 사용]
- Purpose: the user prompt currently uses StyledText, which paints its existing
  find ranges but does not participate in the shared text-selection engine.
- Existing assets: kit plain_text already uses the same retained TextView/global
  selection as Markdown with escaped literal content. FindMarks, its byte-range
  adapter, the existing VirtualList reveal and theme palette already exist.
  SelectableText exists too, but has no public find-range input; adding a separate
  highlight policy/renderer there would duplicate the current selection/find path.
- Choice: extend the existing literal renderer with optional existing FindMarks,
  retaining its state, literal source mapping and selection format. B2 approved
  exactly fn mark -> pub(crate) fn mark with the same signature/body, so the
  existing range implementation is reused once. No parser, Actions, selection
  manager or other engine callback is copied or added.
- Why: the same presented_prompt slice drives the bubble, local find and explicit
  copy/rewind text; canonical Entry identity/content stays unchanged. Rendering
  needs App only, not Shell/network authority. Existing borrowed row presentation
  scopes managed widget caches by display source; W12's actual resident renderer
  must replace legacy route/session scope with canonical ResidentToken scope.
- Effects and limits: the existing HTML paragraph wrapper normalizes HTML line
  endings and plain selection retains its structural paragraph separator. The
  dedicated copy button still uses the exact displayed string. Scheme/query
  cache changes clear/rebuild the same marks, without clearing text selection.

Remaining concrete dependencies: gpui-base RangeHighlight currently carries only
background color, and TextViewState::set_range_highlights/reveal_range reject the
HTML format as Unsupported. The old user StyledText range uses FindPalette.current_ink;
copying the Markdown background-only path would lose that current-match ink in
dark mode. B2 independently confirmed both limitations and will coordinate one
named owner/contract for an opt-in rendered-range/ink seam in the SAME TextView.
The old mark helper's cache-key Some is not evidence that a range was accepted:
it intentionally ignores a refused range on its Markdown-only product path.
The draft fixture therefore requires actual accepted highlights, not just that
cache flag, and is known to need the missing capability before it can pass.
There is no duplicate overlay renderer or
alternate mark helper here. The caller must remain unaccepted until this is
resolved; this draft is not a claim of color/accessibility parity.

Source fixtures exercise the shared literal engine with marks, scheme change and
find close while retaining selection payload; a headless user-bubble draw uses
the actual optional-mark caller without constructing a managed session. Those
new fixtures have not run. Full A7 captures and package review remain separate.
