# ADR 0100: HTML rendered ranges and optional highlight foreground in the shared text engine

Status: V1 contract direction clear; separate implementation candidate uncompiled.
Owner: B2, the existing text-engine seam owner. A7 owns only its prompt caller.
Failing-first test-only baseline: package base10, 3e7820e3.
Implementation candidate: announced package base13.1, ad14f5f6.

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

## V1 direction incorporated before source

Direction on exact66e6e2ad clears this same-engine contract only. HTML key identity is a separate domain scoped to the exact parsed document/committed revision; the same key/traversal must reach index, ordinary paint, reveal and selection. It cannot survive a reparse as authority even when text length and leaf ordinal repeat. Preserve the existing asynchronous commit fence, including A→B→A; invalidate at accepted replacement and commit.

Color-only updates invalidate the same retained StyledText/TextRun cache entry without changing metrics, fonts, underline/link/code styling, fade semantics, background order or selection overlay. Last overlapping Some foreground wins; None never erases an earlier supplied foreground.

Applied kit marks must match the actual retained owner and committed rendered revision as well as request key, relevant palette and current-match ordinal. Equal external keys after a reparse cannot suppress reapplication. Only an accepted complete set is cached; invalid UTF-8/reversed/OOB sets preserve prior paint, empty accepted sets clear, and reveal acceptance remains separate from real scrolling.

The approved failing-first strategy uses existing APIs: HTML assertions fail on Unsupported and Markdown actual-drawn TextRun assertions fail on missing ink. Later fixture helpers add only explicit opt-in/foreground calls; assertions remain. Final qualification also includes both themes, overlap/update/clear, actual partial/multiblock Plain selection and list reveal, reparse/A→B→A stale-work rejection, default HTML Unsupported, unchanged Markdown/source behavior and rejected-mark retry. This is not source/runtime/UI or caller adoption.

Baseline fixtures are now authored before feature code. The literal test first proves UTF-8/entities/whitespace and Plain copy, then requires actual HTML range acceptance and atomic invalid UTF-8 refusal/clear; the default HTML fixture preserves Unsupported. The Markdown fixture draws the same TextView, reads its actual retained TextRun color/font, requires current ink, then verifies clearing restores the original runs. Only cfg(test) module registration and tests accompany this stage; fixture helpers intentionally use existing constructors/background-only ranges so the missing capabilities fail at real runtime assertions. No nonexistent opt-in/foreground symbol is referenced.

The test-only baseline additionally requires reveal_range acceptance for the same valid literal UTF-8 range. It keeps the original highlight/copy/ink assertions and references no new API. This prevents later highlight support alone from hiding an unchanged Unsupported reveal. Original baseline ad889 remains available; neither baseline has been executed while the approved engine-unit recipe is pending.


## Source candidate and execution boundary

The original test-only baseline remains immutable, with no capability setup:
`eff369b040c1c3a2480f0213b777a3321cc35d36`. The separate candidate adds only
the opt-in/foreground calls in its original fixture helpers; its existing
result, copy, actual-run ink, reveal and clear assertions remain intact.
Neither source has been run through the approved engine-unit scratch recipe.
The scratch recipe is tool-owner work: no product manifest, dependency or lock
is changed to simulate an engine test. Native dependency compilation cannot be
reported as executing the vendored engine's cfg(test) tests.

The same IndexBuilder now assigns HTML keys into the retained InlineState
while building the committed index on the UI thread. Keys have a separate
Rendered domain, view owner, committed revision, top-level block and global
leaf ordinal. Paint obtains that same key from the same state used by
selection; reveal uses its top-level block. Markdown Source keys keep their
original encoding/order and source remap. HTML indexes are built eagerly at
opt-in/commit so a retained snapshot has fixed keys before an append can share
old leaf states. This trades one rendered-index traversal per committed
opted-in HTML parse for immutable snapshot identity. No background parser
mutates displayed identity. Named element IDs retain all identity components;
the fixed opaque ID is too small to carry them without truncation.

Foreground ranges are resolved independently of backgrounds, in original
input order, and refine only color after authored styles and before existing
stream fade. They reach the original retained TextRun cache; changing ink
changes its existing run comparison. Background-only input adds no foreground.
One opaque acceptance receipt includes the actual owner/committed revision and
retains a per-set Arc token; invalid input preserves the previous receipt,
accepted empty sets clear paint with a fresh receipt, and clear/reparse revoke
it. The single kit mark cache also checks palette and current ordinal.

Additional candidate fixtures cover atomic multi-range refusal, accepted-empty
clear, A-B-A, rejected stale async commit, actual enclosing-list reveal, native
partial/multiblock literal selection, and actual retained run overlap/update/
clear under both appearance settings. They are authored assertions, not passed
execution evidence. Product A7 remains unchanged and its caller adoption held.


The component compatibility facade selectively re-exports existing text types.
The kit imports the additive stamp/support types directly from its already
owned gpui-base dependency, while its existing TextView/State facade remains
unchanged. No new dependency or facade patch is needed. This import boundary was
corrected during source preflight before any engine/candidate job.
