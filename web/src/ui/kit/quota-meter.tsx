// A read-only share, following Paseo usage/meter.tsx. The caller provides
// the validated view value: this control never counts, clamps, polls, or
// adds the context ring's minimum arc. Zero must paint zero width.
import "./quota-meter.css";

import type { QuotaPercent } from "../../quota-percent";

export type QuotaMeterTone = "default" | "ok" | "warning" | "danger";
export type QuotaMeterEmphasis = "normal" | "quiet";

function toneClass(tone: QuotaMeterTone): string {
  switch (tone) {
    case "default":
      return "quota-meter-muted";
    case "ok":
      return "quota-meter-ok";
    case "warning":
      return "quota-meter-warning";
    case "danger":
      return "quota-meter-danger";
  }
}

function fillClass(tone: QuotaMeterTone, emphasis: QuotaMeterEmphasis): string {
  switch (emphasis) {
    case "normal":
      return toneClass(tone);
    case "quiet":
      return "quota-meter-quiet";
  }
}

/** A non-interactive, named quota bar; its parent supplies used/remaining words. */
export function QuotaMeter({
  value,
  label,
  valueText,
  tone = "default",
  emphasis = "normal",
  className,
}: {
  value: QuotaPercent;
  label: string;
  valueText?: string;
  tone?: QuotaMeterTone;
  emphasis?: QuotaMeterEmphasis;
  className?: string;
}) {
  return (
    <div
      className={`quota-meter${className === undefined ? "" : ` ${className}`}`}
      role="meter"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={value.percent}
      aria-valuetext={valueText}
    >
      <span
        className={`quota-meter-fill ${fillClass(tone, emphasis)}`}
        style={{ width: `${String(value.percent)}%` }}
        aria-hidden="true"
      />
    </div>
  );
}
