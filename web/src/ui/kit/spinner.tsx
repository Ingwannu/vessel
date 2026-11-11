// Spinner (DESIGN.md §6.13): a ring with a gap that turns, in the current
// text colour. Stopped under reduced motion (styles/motion.css), where it
// still shows as a ring with a gap.

/** 12px in rows and buttons, 14px beside a 14px label, 16px alone. */
import "./spinner.css";

export type SpinnerSize = 12 | 14 | 16;

export function Spinner({
  size = 12,
  label,
}: {
  size?: SpinnerSize;
  /** What is in progress, for a screen reader; without it the spinner is
   * decoration beside words that already say so. */
  label?: string;
}) {
  return label === undefined ? (
    <span className={`spinner s${size}`} aria-hidden="true" />
  ) : (
    <span className={`spinner s${size}`} role="status" aria-label={label} />
  );
}
