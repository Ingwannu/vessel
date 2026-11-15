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
  existing drawing owner. Append only: never insert within or reorder the
  frozen prefix. Its snapshot remains immutable when new glyphs are added. No new glyph, vector, path, runtime
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

Lead append-contract correction: the frozen95 snapshot checks a complete
unchanged prefix and ALL length at least95; later appended glyphs are covered
by the existing uniqueness/path-existence/raster tests. The snapshot is never
regenerated just to admit additions. No production code changed.

## Exact final package verification

Final code/test commit bb33cee31b037f487ff6840b9db0b86a960e835b completed
A13 shell2-20261115T151942Z-bb33cee3 with all six native steps exit0:
fonts, format, quality, strict all-target Clippy, app build and tests.
Raw totals independently summed:1433 passed/0 failed/2 existing ignored;
Kit181 passed/0 failed/0 ignored, including the frozen95-prefix/path test.
Changed Kit source was actually checked and compiled; App was checked/built.
Logs: /tmp/work/m12-shell-s2/glyph-inventory-package-native.
The prior e7 Kit narrow result remains prior-source evidence only.

This result covers the isolated95 registry and unchanged baseline native API.
It does not approve F4's five additional glyphs, consumer ink/geometry, native
OS input, the sidebar package or measured popup placement. No new capture
matrix was taken: no vector or renderer changed in this mechanical producer.
V1's bounded source/package disposition and lead integration remain separate.

V1 final disposition9373dccd: bounded mechanical SOURCE/native PASS for exact
e1b73c9e8..bb33cee31 +docs-only db04de651. Independent original enum/ALL/path and
variant documentation order match; old loaders/tests, three drawing sources and
external mark bytes match. Exact six-step A13 and Kit181/0/0 verified. Audit:
/tmp/work/m12-review/s2-glyph-inventory-bb33/audit.json. Lead may adopt the isolated
producer preserving newer append-only entries. No F4 five/artwork/consumer
UI/OS/F5 approval and no repeated successful gate/capture matrix requested.
