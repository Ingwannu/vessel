// Segmented (DESIGN.md §6.13): one of two to four choices, all in view
// (the theme: system, light, dark). A radio group, so arrow keys move the
// choice and Tab leaves it.
//
// agent-orchestrator's settings segment (a 2px-inset track on the field
// fill, with its edge; 13px items, 32px tall), and the chosen segment as
// GOAT draws it: a thumb lifted by a hairline, since the reference's own
// (its menu's selected fill on the field fill) cannot be told apart on a
// white card. `sm` is GOAT's 24px size, for toolbars.

import "./segmented.css";

import { Radio } from "@base-ui/react/radio";
import { RadioGroup } from "@base-ui/react/radio-group";

import { type Choice, choiceOf } from "./choice";
import { Icon } from "./icon";

export function Segmented<V extends string>({
  label,
  value,
  onChange,
  options,
  size = "md",
  iconsOnly = false,
}: {
  /** The group's name for a screen reader ("Theme"). */
  label: string;
  value: V;
  onChange: (value: V) => void;
  options: readonly Choice<V>[];
  size?: "md" | "sm";
  /** Icons alone, each named by its label (a toolbar's). */
  iconsOnly?: boolean;
}) {
  return (
    <RadioGroup
      className={size === "sm" ? "segmented sm" : "segmented"}
      aria-label={label}
      value={value}
      onValueChange={(next: unknown) => {
        const option = choiceOf(options, next);
        if (option !== undefined) {
          onChange(option.value);
        }
      }}
    >
      {options.map((option) => (
        <Radio.Root
          key={option.value}
          className="segment"
          value={option.value}
          disabled={option.disabled}
          aria-label={iconsOnly ? option.label : undefined}
        >
          {option.icon === undefined ? null : (
            <Icon name={option.icon} size={size === "sm" ? 14 : 15} />
          )}
          {iconsOnly ? null : <span>{option.label}</span>}
        </Radio.Root>
      ))}
    </RadioGroup>
  );
}
