// What Settings › About lists: the software this page is built from and
// the licence each is under. A list kept by hand, checked against
// `package.json` by its test, so a dependency added without a line here
// fails `npm test` rather than going unlisted.

/** One package the page bundles. */
export interface Bundled {
  name: string;
  /** Its SPDX licence id. */
  licence: string;
  /** What it does here. */
  role: string;
  /** The licence's text as the page serves it, for the fonts (the
   * libraries' are in their packages). */
  text?: string;
}

/** Every runtime dependency in `package.json`, in its order. */
export const BUNDLED: readonly Bundled[] = [
  { name: "@base-ui/react", licence: "MIT", role: "Controls' behaviour" },
  {
    name: "@fontsource-variable/geist",
    licence: "OFL-1.1",
    role: "Geist, the page's face",
    text: "/licenses/Geist-OFL.txt",
  },
  {
    name: "@fontsource-variable/geist-mono",
    licence: "OFL-1.1",
    role: "Geist Mono, for code and ids",
    text: "/licenses/GeistMono-OFL.txt",
  },
  { name: "@tanstack/react-router", licence: "MIT", role: "Addresses" },
  { name: "@xterm/addon-fit", licence: "MIT", role: "Terminal sizing" },
  { name: "@xterm/xterm", licence: "MIT", role: "The terminal" },
  {
    name: "pretendard",
    licence: "OFL-1.1",
    role: "Pretendard, for Hangul",
    text: "/licenses/Pretendard-OFL.txt",
  },
  { name: "react", licence: "MIT", role: "The page" },
  { name: "react-dom", licence: "MIT", role: "The page" },
  { name: "react-markdown", licence: "MIT", role: "Agents' replies" },
  { name: "yaml", licence: "ISC", role: "Markdown files' front matter" },
];

/** Vessel's own licence (`LICENSE.md`, ADR 0014). */
export const VESSEL_LICENCE = "PolyForm Strict 1.0.0";
