# Shared inline external link for A9

Coordinator c1893bfe delegates this control-only seam to G3. V1 confirmed
that lead9e18cf0d and the ledger have no equivalent. G2 delegates exactly
ArrowUpRight's inventory/vector/test seam (bc263667), retains PackagePlus,
Gauge and ProviderRow. No URL opener or area drawing changes are in this slice.

Reference: Paseo0.15.4 `components/ui/external-link.tsx:35–121`, full source
read, and `components/provider-catalog-list.tsx:81–88` first consumer. Label12,
arrow12, gap4, hitSlop8 in both input layouts, transparent background and
muted→foreground for both label and actual SVG mask. No pressed wash, disabled
state or responsive32/44 pill is invented. The arrow is an original24-grid
stroke1.5 line drawing (the app-wide LucideProvider value); every old glyph/vector remains
unchanged and the canonical inventory grows92→93. The caller retains its typed
URL and opener; kit imports no proto/View/browser/OS dependency. Existing
Tooltips supplies the optional hint at the consumer.

[Decision Log]
- Purpose: one faithful reusable inline link for installation instructions.
- Existing constraint: Ghost buttons have different size/fill and area URL
  actions are not shared controls; SVG hover ink must be set on the mask itself.
- Alternatives: area pressable, Ghost substitute, or OS opener in kit.
- Choice: public pure presentation with optional label and caller click handler.
- Why: link appearance is shared while opening/navigation belongs to the app.
- Impact: one module/export, one glyph with literal both-theme geometry and
  real GPUI expanded-hit/row-pitch/callback-isolation tests. Raster bounds test
  checks the actual12px asset. There is no new dependency.

Necessary native difference: Paseo specifies font12 without a line-height;
RNWeb uses platform-normal text metrics. Existing native Type::Meta supplies
12/16 regular and embedded Geist/Pretendard fallbacks. No false claim of a
measured browser-normal line-height equivalence is made. GPUI accessibility
and keyboard link semantics are the existing kit-wide gap. Optional arbitrary
React children have no consumer here; this API accepts the first consumer's
plain localized label or no label. Tooltip and actual URL opening are not
part of this control and are not proved by its tests.

This commit is source only until exact remote Kit narrow checks complete.
The previous Agents narrow runs were signalled143 before compilation; those
are not a control pass. No whole A9 gate or first-use capture is claimed.


First exact Kit Clippy on e2afa5df returned101 (remote step result1): this
base has no icon::XS token, and children-bounds observation belongs to Div
before `.id()` creates Stateful<Div>. Corrected to one named source12
constant and the same observer before stateful wrapping; no test assertion,
geometry, glyph, callback or reference value changed. The red log is retained
at the track's external-link-20261115T104331Z run. Corrected narrow is pending.


The corrected53c20227 Kit Clippy compiled the actual Kit and returned101 on
four documentation identifiers needing code marks and three measured-test
coordinate operators forbidden by the workspace's arithmetic lint. The follow-up
keeps every measured assertion and click coordinate, expressing their same
values through the existing checked-coordinate/mul_add convention. No allow,
expect, weakened assertion, geometry or callback change was introduced. The
separate red run remains external-link-fix-20261115T105358Z; tests did not run.


V1's complete039c source return found the sole P2: the component does not
specify stroke width, but `_layout.tsx:926` wraps AppShell in LucideProvider
with `styles/theme.ts:602` ICON_STROKE_WIDTH1.5. The earlier package-default2
claim was wrong. Reuse the unchanged canonical1.5 line-glyph renderer for this
new arrow, and pin its entire loaded SVG string, including literal1.5 stroke,
in the raster test. Every other vector and the95-entry consumer inventory stay
unchanged; no per-area adjustment. Exact corrected narrow is pending. The039c
queued narrow is old-source evidence only and cannot close this source return.
