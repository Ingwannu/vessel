// Two more rows a menu can hold (DESIGN.md §6.13), beside menu.tsx's: a
// setting that is on or off, shown as a switch at the row's right, and a
// line of text that is not a choice.
//
// agent-orchestrator's model menu ends with its agent's on/off settings
// ("Fast mode", "Thinking") as rows with a switch (12px text, 8px by 12px)
// that keep the menu open, so several can be flipped; and says in the
// menu itself why a list is empty. The switch is drawn, not a control of
// its own: the row is the one control (Base UI's checkbox item, so a
// screen reader hears a checked menu item), and a click or Enter on it
// fires once.

import "./popup.css";
import "./menu.css";
import "./switch.css";

import { Menu as Base } from "@base-ui/react/menu";
import type { ReactNode } from "react";

export function MenuSwitchItem({
  checked,
  onChange,
  disabled,
  children,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  children: ReactNode;
}) {
  const on = checked ? "" : undefined;
  return (
    <Base.CheckboxItem
      className="menu-item menu-switch-row"
      checked={checked}
      onCheckedChange={(next) => {
        onChange(next);
      }}
      disabled={disabled}
      closeOnClick={false}
    >
      <span className="label">{children}</span>
      <span className="switch" data-checked={on} aria-hidden="true">
        <span className="switch-thumb" data-checked={on} />
      </span>
    </Base.CheckboxItem>
  );
}

/** Text in a menu that is not a row to choose: why its choices are
 * missing, or what they depend on. */
export function MenuNote({ children }: { children: ReactNode }) {
  return <div className="menu-note">{children}</div>;
}
