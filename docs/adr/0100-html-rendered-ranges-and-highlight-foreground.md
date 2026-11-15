# ADR 0100: HTML rendered ranges and optional highlight foreground in the shared text engine

Status: proposed contract; coordinator-owned scope approved, V1 direction pending.
Owner: B2, the existing text-engine seam owner. A7 owns only its prompt caller.
Receiving native baseline: announced package base10, 3e7820e3.

## Decision Log

- Purpose and intent: let literal user-prompt text use the existing selectable TextView while preserving find ranges, current-match contrast, reveal and plain copy.
- Existing assets and constraints: A7 already renders escaped literal text through the HTML TextView. The one engine owns parsed documents, rendered UTF-8 text, selection, range highlights and reveal. HTML currently returns Unsupported for setting/revealing ranges. RangeHighlight carries a background only. HTML text leaves have no Markdown source spans, so merely removing the format guard cannot create addressable leaves. The kit mark helper currently ignores a refused result and caches an applied key.
- Alternatives considered: another prompt renderer/parser/selection controller; fabricated Markdown source positions; escaping the prompt into Markdown/code blocks; accepting a cached mark key as proof; copying mark into a literal wrapper. These lose literal or whitespace behavior, invent mapping, add competing state, or hide rejected effects.
- Chosen approach: one opt-in extension of the existing TextView range/index path, one typed rendered-only leaf identity, optional foreground on RangeHighlight, and the same selection/reveal path. Rejected updates are never cached as applied.
- Reason: rendered text already supplies the correct search/copy coordinate space; source-map absence must remain explicit. Presentation changes must not replace the retained view or change user text.
- Advantages, costs and effects: existing controls and selection/find mapping remain shared. The engine gains HTML leaf identity and foreground range refinement; HTML range state is intentionally invalidated on reparse rather than claiming Markdown-style source retention. Background-only/default HTML behavior remains compatible.

## Source evidence and limits

Existing engine paths are relative to spikes/release/patches/gpui-base/src/text:

- state.rs set_range_highlights and reveal_range reject non-Markdown format.
- range_highlight.rs RangeHighlight stores range/background; RenderedIndex indexes the same ParsedDocument used by plain copy.
- stream_fade.rs TextLeafKey/text_leaves and range_highlight.rs IndexBuilder currently derive addressable leaves from real source spans.
- node.rs draws the text/highlights/reveal using those same leaves.

A7's held prompt-selection source508ace2bd is consumer context, not an approved implementation. The current product prompt remains StyledText until the engine and caller pass their independent checks. The existing plain-text wrapper escapes HTML and uses Plain selection; no Markdown parsing or source-copy claim is introduced.

## Typed opt-in and compatibility

Proposed public shape for V1 direction (names may be refined before source):

- RenderedRangeSupport::{MarkdownOnly, HtmlAndMarkdown}; existing default is MarkdownOnly.
- A TextViewState construction builder selects HtmlAndMarkdown for the literal prompt. It creates no new Entity, subscription, renderer, parser or controller.
- RangeHighlight::new(range, background) is unchanged. An additive with_foreground(Option<Hsla>) supplies an optional ink override; None is the existing behavior.
- Existing set_range_highlights/reveal_range continue to use validated UTF-8 byte ranges in the current rendered_text snapshot. They remain Unsupported for HTML without the opt-in.

Foreground is a color refinement only: no font, metrics, layout, text, source or selection change. On overlapping ranges the last supplied foreground wins; a background-only range adds no foreground override. Existing background ordering, selection behavior, authored styles and copy remain unchanged except where an explicit foreground refines affected glyph ink. Current-match ink must remain readable over the existing current-match fill.

## One identity and coordinate path

The existing parsed leaves and rendered index gain a private typed rendered-only key for HTML leaves. It is distinct from the existing source-start/table-cell identity. A rendered ordinal is never inserted into a Markdown SourceSpan or returned as a source position. Ordinals are assigned from the existing parsed document traversal and used by its normal node paint/reveal path; no second identity registry or parallel index is added.

Markdown identities, source maps, inference/find offsets and source selection remain unchanged. HTML source-range conversion remains unavailable when the parser recorded no source mapping. This extension addresses rendered text only. Existing non-text objects/separators remain unpainted as in the current range API; no image/custom-block selection promise is added.

The view owner and committed revision already fence rendered_text snapshots. HTML leaf identities are scoped to that parse. A reparse invalidates HTML highlights and pending reveal rather than moving them by a guessed source position. The caller rescans the current committed rendered_text and applies its current ranges in the same state update. A pending old reveal must not scroll a replacement prompt.

## Mutation and result contract

- Validate every range before accepting the set: reversed, out-of-bounds or non-UTF-8-boundary ranges refuse atomically and preserve the previously accepted set.
- An empty accepted highlight set clears paint. Reparse invalidation is separate from invalid-range refusal.
- On an accepted HTML text replacement, clear obsolete highlights/reveal; clear again when its new parse commits so interim scans cannot retain old offsets.
- reveal_range Ok means a valid request was accepted, not that scrolling already happened. Preserve the existing best-effort/list-reveal contract and its bound.
- Kit caches a mark key only after range acceptance. A rejected range/unsupported format remains un-applied and may be retried on a later current revision; it is not reported as successful find ink.
- Keep the single existing mark implementation and the existing window selection aggregator. A7 may consume a crate-visible mark after this producer; it must not copy it.

## Tests before implementation

Write behavior assertions first and preserve them through implementation. Baseline runs use the current engine and must exhibit the actual Unsupported/missing-ink defects; no knowingly-red build caused only by nonexistent API names is submitted. Small fixture helpers may later add the explicit opt-in/foreground builder calls; result, paint, copy and reveal assertions cannot be weakened to a cached key or synthetic marker.

1. Escaped literal UTF-8 text (Korean, combining/non-BMP characters, <>&, repeated spaces, tabs and line breaks) keeps rendered_text and Plain copy unchanged. Valid searched rendered ranges are actually accepted in the opted-in HTML view.
2. Real rendered current-match glyph ink uses current_ink over current fill; ordinary matches keep existing ink. Compare actual glyph/run or rendered paint evidence, not only the stored option. Bounds remain unchanged.
3. Empty accepted ranges remove highlights. Invalid multi-range input refuses atomically and leaves earlier accepted paint intact; interior UTF-8 byte offsets are refused.
4. A reveal over a real retained HTML TextView is accepted and reaches the existing list/reveal bridge. Text replacement clears pending reveal/highlights; a new committed revision must be rescanned before applying new positions.
5. Real partial drag and multi-block Plain copy preserve exact literal whitespace and shared selection mapping before/after highlight/reveal. No second selection participant replaces the window aggregator.
6. Existing Markdown highlight/reveal/source/copy tests remain unchanged and pass. Default HTML remains Unsupported. The kit must not cache a rejected range as applied.

## Qualification and ownership

V1 direction comes before implementation. Then run the failing-first actual engine fixtures, implement only the approved seam, update the vendored patch list with its Why, and run exact Kit strict/engine tests on the final producer. The clean producer has no A7 timeline/host/root-opening change. A7 wires its one prompt caller only after the checked hash and performs its own consumer/selection/find validation. No capture, UI approval or full B2/A7 acceptance is inferred from contract or engine tests.
