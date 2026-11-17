// A provider's approved mark beside its usage report (Paseo usage/card.tsx
// `useProviderIcon`; the native cards' agent marks).
//
// [Decision Log] (2026-11-16, impl U, TRACKER 771, coordinator return)
// - Purpose: a usage card names its provider by the vendor's own mark, as
//   Paseo and the native app do, not by a generic glyph.
// - Constraint: only approved brand forms; the page's content security
//   policy allows images from itself and data URIs only.
// - Choice: the native kit's artwork (spikes/native/kit/src/agent_mark/
//   paths.rs), copied byte for byte into provider-marks/, bundled by Vite
//   as images. Marks with light and dark forms switch with the page's
//   scheme in CSS (the system's, or the pinned `data-theme`).
// - Cost: two copies of the artwork; a change to a native mark is copied
//   here too.
import "./provider-mark.css";

import claude from "./provider-marks/claude.svg";
import codexDark from "./provider-marks/codex-dark.svg";
import codexLight from "./provider-marks/codex-light.svg";
import githubDark from "./provider-marks/github-dark.svg";
import githubLight from "./provider-marks/github-light.svg";
import opencodeDark from "./provider-marks/opencode-dark.svg";
import opencodeLight from "./provider-marks/opencode-light.svg";

export type ProviderMarkName = "claude" | "codex" | "opencode" | "github";

const ART: Record<ProviderMarkName, { light: string; dark: string }> = {
  claude: { light: claude, dark: claude },
  codex: { light: codexLight, dark: codexDark },
  opencode: { light: opencodeLight, dark: opencodeDark },
  github: { light: githubLight, dark: githubDark },
};

/** Decorative: the card's name carries the provider for a screen reader. */
export function ProviderMark({
  name,
  size,
}: {
  name: ProviderMarkName;
  size: 14 | 16;
}) {
  const art = ART[name];
  if (art.light === art.dark) {
    return (
      <img
        className="provider-mark"
        src={art.light}
        width={size}
        height={size}
        alt=""
      />
    );
  }
  return (
    <span className="provider-mark-pair">
      <img
        className="provider-mark light"
        src={art.light}
        width={size}
        height={size}
        alt=""
      />
      <img
        className="provider-mark dark"
        src={art.dark}
        width={size}
        height={size}
        alt=""
      />
    </span>
  );
}
