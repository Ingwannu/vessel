// Switch (DESIGN.md §6.13): on or off, taking effect at once. Not for a
// choice a form submits later (a Checkbox is).
//
// agent-orchestrator's: a 32 by 16 track with a 14px thumb that travels
// 16px; off is the ink's thumb on a 4% track, on is the primary button's
// fill with a thumb in its text colour; inset shading only; the hit area
// reaches 12px past the sides and 8px above and below.

import "./check-label.css";
import "./switch.css";

import { Switch as Base } from "@base-ui/react/switch";

export function Switch({
  checked,
  onChange,
  label,
  showLabel = false,
  disabled,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Its name. Visible beside it with `showLabel`; otherwise the row it
   * sits in shows it and this names the control for a screen reader. */
  label: string;
  showLabel?: boolean;
  disabled?: boolean;
}) {
  const control = (
    <Base.Root
      className="switch"
      checked={checked}
      onCheckedChange={(next) => {
        onChange(next);
      }}
      disabled={disabled}
      aria-label={showLabel ? undefined : label}
    >
      <Base.Thumb className="switch-thumb" />
    </Base.Root>
  );
  return showLabel ? (
    <label
      className="check-label"
      data-disabled={disabled === true ? "" : undefined}
    >
      {control}
      <span>{label}</span>
    </label>
  ) : (
    control
  );
}
