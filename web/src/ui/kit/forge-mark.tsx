// A forge's mark beside "Open on <forge>" (TRACKER 467 step B): one
// colour, the surrounding text's, as Paseo draws a forge in its open list
// (`open-in-editor/button.tsx:56`, the muted foreground). A vector, so it
// stays sharp at any zoom.

import { FORGE_MARK_PATHS, type ForgeMarkName } from "./forge-marks";

/** Decorative: the control's name carries the forge for a screen reader. */
export function ForgeMark({
  forge,
  size,
}: {
  forge: ForgeMarkName;
  size: 14 | 16;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      aria-hidden="true"
      focusable="false"
    >
      <path fill="currentColor" d={FORGE_MARK_PATHS[forge]} />
    </svg>
  );
}
