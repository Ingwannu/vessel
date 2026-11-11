// Select (DESIGN.md §6.2): one value from a closed list, drawn by the
// page. The look is agent-orchestrator's settings option menu, the select
// it uses everywhere: a filled trigger (the secondary button's surface)
// with the value first and a chevron that turns when open; a popup of
// 13px rows where the chosen one keeps a 7% wash (no check mark) and the
// highlighted one a 4% wash.
//
// Behaviour is Base UI's Select: typeahead, arrow keys, Home and End, the
// list scrolled to the chosen row, placement that flips at the window's
// edge, Escape and an outside click to close, focus back on the trigger.
// The popup opens below the trigger (not over it, Base UI's default) as
// the reference's does.

import "./popup.css";
import "./select.css";

import { Select as Base } from "@base-ui/react/select";

import { type Choice, choiceOf, runs } from "./choice";
import { Icon } from "./icon";

export interface SelectProps<V extends string> {
  /** `null` while nothing is chosen: the placeholder shows. */
  value: V | null;
  onChange: (value: V) => void;
  options: readonly Choice<V>[];
  /** The control's name when it is not inside a Field. */
  "aria-label"?: string;
  /** `md` (32px) in a form; `sm` (28px) in a row or a toolbar. */
  size?: "md" | "sm";
  /** `filled` is the settings select; `ghost` is a composer's quiet pill,
   * filled only on hover and while open. */
  look?: "filled" | "ghost";
  /** `fill` takes its container's width (a form); `fit` its value's. */
  width?: "fill" | "fit";
  /** Shown while `value` matches no option. */
  placeholder?: string;
  disabled?: boolean;
  /** Which edge the popup lines up with. */
  align?: "start" | "end";
  /** Where the popup opens; it flips when there is no room. */
  side?: "bottom" | "top";
}

export function Select<V extends string>({
  value,
  onChange,
  options,
  size = "md",
  look = "filled",
  width = "fill",
  placeholder = "Choose…",
  disabled,
  align = "start",
  side = "bottom",
  "aria-label": ariaLabel,
}: SelectProps<V>) {
  const chosen = choiceOf(options, value);
  const classes = [
    "select-trigger",
    size === "sm" ? "sm" : "",
    look === "ghost" ? "ghost" : "",
    width === "fill" ? "fill" : "",
  ]
    .filter((part) => part !== "")
    .join(" ");
  return (
    <Base.Root
      value={value}
      onValueChange={(next) => {
        const option = choiceOf(options, next);
        if (option !== undefined) {
          onChange(option.value);
        }
      }}
      items={options.map((option) => ({
        value: option.value,
        label: option.label,
      }))}
      disabled={disabled}
    >
      <Base.Trigger className={classes} aria-label={ariaLabel}>
        <span className="select-value">
          {chosen?.icon === undefined ? null : (
            <Icon name={chosen.icon} size={14} />
          )}
          <span className="label">{chosen?.label ?? placeholder}</span>
          {chosen?.detail === undefined ? null : (
            <span className="detail">{chosen.detail}</span>
          )}
        </span>
        <span className="select-chevron">
          <Icon name="chevron-down" size={13} />
        </span>
      </Base.Trigger>
      <Base.Portal>
        <Base.Positioner
          className="popup-positioner"
          alignItemWithTrigger={false}
          side={side}
          align={align}
          sideOffset={6}
          collisionPadding={8}
        >
          <Base.Popup
            className={
              width === "fill"
                ? "popup select-popup fill"
                : "popup select-popup"
            }
          >
            <Base.List className="popup-scroll">
              {runs(options).map((run, index) =>
                run.group === undefined ? (
                  run.options.map((option) => (
                    <Item key={option.value} option={option} />
                  ))
                ) : (
                  <Base.Group key={`${run.group}-${String(index)}`}>
                    <Base.GroupLabel className="menu-label">
                      {run.group}
                    </Base.GroupLabel>
                    {run.options.map((option) => (
                      <Item key={option.value} option={option} />
                    ))}
                  </Base.Group>
                ),
              )}
            </Base.List>
          </Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  );
}

function Item<V extends string>({ option }: { option: Choice<V> }) {
  return (
    <Base.Item
      className="menu-item"
      value={option.value}
      label={option.label}
      disabled={option.disabled}
    >
      {option.icon === undefined ? null : <Icon name={option.icon} size={15} />}
      <Base.ItemText className="label">{option.label}</Base.ItemText>
      {option.detail === undefined ? null : (
        <span className="detail">{option.detail}</span>
      )}
    </Base.Item>
  );
}
