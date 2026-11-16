# Dialog header search row (Kit seam)

Kit leaf for the label manager's search (app review RETURN of row 316: the
consumer drew the standalone boxed `SearchField` in the body). Additive on
package base 14.6; dialogs without a search render exactly as before.

[Decision Log]
- Purpose: Paseo's label manager passes its query input as the sheet header's
  `search` (workspace-labels/manager-modal.tsx 120-138); `AdaptiveModalSheet`
  draws it under the title inside the header container
  (adaptive-modal-sheet.tsx 107-110 rule, 149-155 `searchRow`, 186-191
  `searchInput`, 443-461 render): a 16-point muted Search mark, gap 8, the
  sheet's 24 inset and 12 below, the input flex 1 with 8 above and below at
  the base size, placeholder muted; no box, fill, clear control or width cap;
  the same header on compact (614) and desktop (781).
- Existing assets: the Kit dialog header (title, caller actions, close, rule)
  and `SearchField`, which wraps the identified one-line `TextField` engine
  (`bare` renderer, focus, IME, change path, `with_source` FieldId).
- Alternatives: the boxed `SearchField::element` in the body (the returned
  look: it is Paseo's screen filter rail); a header slot taking any element (a
  second input or an area style could arrive through it); a new input engine.
- Choice: `SearchField::header_search` returns a typed `HeaderSearch` holding
  the same engine's unboxed input; `DialogContent::with_header_search` and
  `DialogInput::with_header_search` pass it to the one dialog renderer, which
  draws Paseo's row under the title and moves the rule below the row. Only the
  Kit can make a `HeaderSearch`.
- Why: one engine and one dialog header; the owner keeps the query, its
  per-opening reset and its admission.
- Impact: no change for dialogs without a search (same title row and rule).
  The owner resets the field per opening (Paseo's `resetKey`) and decides when
  the search shows (Paseo shows it while listing, not while editing).

Tests (`dialog::header_search_tests`): the row sits directly under the title
across its width, the input starts 48 in (inset, mark, gap) and ends 24 from the
edge, the input is 36 tall and the row 48, on desktop and the compact sheet;
typing reaches the owner's change callback through the engine.
