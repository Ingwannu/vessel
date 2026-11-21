// A pull request's checks as one glyph: Paseo's `CheckIndicator`
// (`components/sidebar/workspace-meta-row/check-indicator.tsx`, pinned
// 0.15.4), TRACKER 426 on the page.
//
// [Decision Log]
// - Purpose: one shape for every outcome, so a row's checks always take
//   the same room and only the inside changes: a check when every one
//   passed, a cross when one failed, a pie filling clockwise from twelve
//   by quarters while they run.
// - Constraint: the kit's icons are CSS masks of fixed drawings
//   (`icon.tsx`); a pie that fills is five drawings, the ring two more.
// - Choice: one inline SVG on Paseo's own geometry (a 24 box, the ring
//   at radius 11 with a stroke of 2 so half the stroke stays inside, the
//   pie at radius 7 so it reads as a separate shape, a full pie drawn as
//   a disc because an arc whose ends meet paints nothing), in the current
//   text colour so the item's tone colours it. Decorative: the item it
//   sits in carries the name.

const BOX = 24;
const CENTER = 12;
const OUTLINE = 11;
const PIE = 7;
const STROKE = 2;
const CHECK = "M8 12.5 L11 15.5 L16.5 9";
const CROSS = "M8.5 8.5 L15.5 15.5 M15.5 8.5 L8.5 15.5";
/** The wedge for one, two and three quarters. */
const WEDGES = [
  "M12 12V5A7 7 0 0 1 19 12Z",
  "M12 12V5A7 7 0 0 1 12 19Z",
  "M12 12V5A7 7 0 1 1 5 12Z",
] as const;

export function CheckIndicator({
  outcome,
  quarters,
  size = 12,
}: {
  outcome: "passed" | "failed" | "running";
  /** Filled quarters of the pie while running (0 to 4). */
  quarters: number;
  size?: number;
}) {
  return (
    <svg
      className="check-indicator"
      width={size}
      height={size}
      viewBox={`0 0 ${String(BOX)} ${String(BOX)}`}
      aria-hidden="true"
      focusable="false"
    >
      <circle
        cx={CENTER}
        cy={CENTER}
        r={OUTLINE}
        fill="none"
        stroke="currentColor"
        strokeWidth={STROKE}
      />
      {outcome === "running" ? (
        <Pie quarters={quarters} />
      ) : (
        <path
          d={outcome === "passed" ? CHECK : CROSS}
          fill="none"
          stroke="currentColor"
          strokeWidth={STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
    </svg>
  );
}

function Pie({ quarters }: { quarters: number }) {
  if (quarters <= 0) {
    return null;
  }
  if (quarters >= 4) {
    return <circle cx={CENTER} cy={CENTER} r={PIE} fill="currentColor" />;
  }
  const wedge = WEDGES[quarters - 1];
  return wedge === undefined ? null : <path d={wedge} fill="currentColor" />;
}
