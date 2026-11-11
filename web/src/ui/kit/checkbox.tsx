// Checkbox (DESIGN.md §6.13): one of several independent choices a form
// submits. agent-orchestrator's: 16px, radius 6, an edge at the strong
// hairline on the 4% field fill; checked, the primary's fill with a 12px
// check in its text colour.

import "./check-label.css";
import "./checkbox.css";

import { Checkbox as Base } from "@base-ui/react/checkbox";

import { Icon } from "./icon";

export function Checkbox({
  checked,
  onChange,
  label,
  showLabel = true,
  disabled,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  /** Without it, `label` only names the control for a screen reader. */
  showLabel?: boolean;
  disabled?: boolean;
}) {
  const control = (
    <Base.Root
      className="checkbox"
      checked={checked}
      onCheckedChange={(next) => {
        onChange(next);
      }}
      disabled={disabled}
      aria-label={showLabel ? undefined : label}
    >
      <Base.Indicator className="checkbox-mark">
        <Icon name="check" size={12} />
      </Base.Indicator>
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
