# One glyph declaration inventory

[Decision Log]
- Purpose: retain compiler-checked glyph identity while the asset inventory
  grows past the function-size budget.
- Existing assets and constraint: Glyph variants, ALL and Glyph::path repeated
  the same registry in three places. The F4 union's exhaustive path match had
  100 arms and 102 lines. GPUI still loads the same embedded SVG source.
- Alternatives: a lint exemption would preserve duplication; string lookup or
  a fallback would weaken compile-time coverage. No external registry or
  dependency is necessary.
- Choice: one documented assets/inventory.rs list is read by one macro_rules
  declaration in assets.rs, generating the same public enum, fixed array and
  exhaustive const path match. include! keeps their original module/API path.
  The drawing match remains separate and exhaustive, unchanged byte for byte.
- Impact: add future variants and paths to that list, and the vector to the
  existing drawing owner. Preserve order. No new glyph, vector, path, runtime
  lookup, fallback, renderer, dependency or lint exemption is introduced.

This isolated producer starts from the announced e1b73c9e app line with 95
glyphs, not F4's unpublished five additional glyphs. inventory_tests.rs freezes
all 95 pre-refactor variants, exact ALL order and paths; it is not generated
from the new inventory during a test. F4 applies the same generator and retains
its pre-change 100-item snapshot separately. That does not promote its five
glyphs into this producer.

Before-source data and source hashes are retained in
/tmp/work/m12-shell-s2/glyph-inventory-before-95; F4's separate input is
glyph-inventory-before-100. Format, size and boundary checks are performed
locally without compilation; actual Kit strict Clippy and tests run only on
the build VM. Until those logs exist, this remains source-only.
