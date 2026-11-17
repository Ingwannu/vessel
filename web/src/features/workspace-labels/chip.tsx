// A label where it is read (Paseo's chip), its colour as a dot where it
// is chosen (picker and manager rows), and the colour choice itself.
//
// [Decision Log]
// - Purpose: chips on workspace rows, dots on picker and manager rows,
//   and swatches in the create and edit views, all drawing one label the
//   same way.
// - Reference: Paseo draws the name in the label's colour on a 10% ground
//   of the same hue, exactly as tall as the line it sits on, truncating
//   rather than pushing other items off; a swatch row picks one of ten.
// - Choice: the colour is a `data-label-color` attribute read by
//   labels.css, so a chip is one element; the swatches are a radio group
//   (arrow keys move, the chosen one is checked) so the choice is
//   reachable without a pointer.

import "./labels.css";

import { useRef } from "react";

import { colorName, LABEL_COLORS } from "../../data/workspace-labels/names";
import type { Label, LabelColor } from "../../wire/proto";

export function LabelChip({ label }: { label: Label }) {
  return (
    <span className="label-chip" data-label-color={label.color}>
      {label.name}
    </span>
  );
}

/** A workspace's chips, in assignment order. */
export function LabelChips({ labels }: { labels: readonly Label[] }) {
  if (labels.length === 0) {
    return null;
  }
  return (
    <span className="label-chips">
      {labels.map((label) => (
        <LabelChip key={label.name.toLowerCase()} label={label} />
      ))}
    </span>
  );
}

/** A label's colour; `null` is the hollow dot of Unlabelled. */
export function LabelDot({ color }: { color: LabelColor | null }) {
  return color === null ? (
    <span className="label-dot none" aria-hidden="true" />
  ) : (
    <span className="label-dot" data-label-color={color} aria-hidden="true" />
  );
}

/** One of the ten colours. */
export function LabelSwatches({
  value,
  onChange,
  disabled = false,
}: {
  value: LabelColor;
  onChange: (color: LabelColor) => void;
  disabled?: boolean;
}) {
  const group = useRef<HTMLDivElement>(null);
  const move = (by: number) => {
    const at = LABEL_COLORS.indexOf(value);
    const next =
      LABEL_COLORS[(at + by + LABEL_COLORS.length) % LABEL_COLORS.length] ??
      value;
    onChange(next);
    group.current
      ?.querySelector<HTMLButtonElement>(`[data-label-color="${next}"]`)
      ?.focus();
  };
  return (
    <div
      ref={group}
      className="label-swatches"
      role="radiogroup"
      aria-label="Color"
      onKeyDown={(event) => {
        if (event.key === "ArrowRight" || event.key === "ArrowDown") {
          event.preventDefault();
          move(1);
        } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
          event.preventDefault();
          move(-1);
        }
      }}
    >
      {LABEL_COLORS.map((color) => (
        <button
          key={color}
          type="button"
          role="radio"
          className="label-swatch"
          data-label-color={color}
          aria-checked={color === value}
          aria-label={colorName(color)}
          tabIndex={color === value ? 0 : -1}
          disabled={disabled}
          onClick={() => {
            onChange(color);
          }}
        />
      ))}
    </div>
  );
}
