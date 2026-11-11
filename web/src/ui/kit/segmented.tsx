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
import { Icon, type IconName } from "./icon";

/** The choices: labelled, or (`iconsOnly`, a toolbar's) icons alone, each
 * named by its label, in which case every option must have its icon. */
export type SegmentedOptions<V extends string> =
  | { iconsOnly?: false; options: readonly Choice<V>[] }
  | { iconsOnly: true; options: readonly (Choice<V> & { icon: IconName })[] };

export function Segmented<V extends string>(
  props: {
    /** The group's name for a screen reader ("Theme"). */
    label: string;
    value: V;
    onChange: (value: V) => void;
    size?: "md" | "sm";
  } & SegmentedOptions<V>,
) {
  const { label, value, onChange, options, size = "md" } = props;
  const iconsOnly = props.iconsOnly === true;
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
