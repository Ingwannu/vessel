// The page's one compact breakpoint, as Paseo's (constants/layout.ts
// `isCompactBreakpoint`: its `xs` and `sm`, below `md` = 600 in
// styles/unistyles.ts): a window narrower than 600px is compact. Below it
// the sidebar leaves the page for a sheet, forms stack, hit targets grow.
//
// [Decision Log]
// - Purpose: TRACKER 805. The page folded at 768px (its own number, in a
//   TS query and a dozen stylesheets) where Paseo, and since 798 the
//   native app (kit PHONE = COMPACT_LAYOUT_WIDTH), fold at 600.
// - Constraint: CSS cannot import a TS constant, and a CSS custom
//   property cannot be read inside `@media`.
// - Alternatives: `@custom-media` (a draft; Lightning CSS resolves it only
//   inside the file that defines it, and the build compiles each
//   stylesheet alone); a lint that compares numbers in two places (still
//   two copies).
// - Choice: the number lives here; stylesheets write `@media (--compact)`
//   and the build puts this query in its place (vite.config.ts
//   `compactMedia`), the same query the TS side matches (use-media.ts).
// - Cost: a stylesheet is only right through the build (vite dev and
//   build both run the plugin); the plugin refuses a width query that
//   names the fold's number itself (599/600, or the old 767/768).

/** The narrowest window that is not compact (Paseo's `md`). */
export const COMPACT_WIDTH = 600;

/** The media query of a compact window. */
export const COMPACT = `(width < ${String(COMPACT_WIDTH)}px)`;

/** The name stylesheets write for {@link COMPACT}: `@media (--compact)`. */
export const COMPACT_MEDIA_NAME = "(--compact)";
