// Menu (DESIGN.md §6.13): actions and settings behind a trigger, drawn by
// the page. agent-orchestrator's action menu: a popup of 13px rows (about
// 32px tall), 15px icons, the highlighted row on a 4% wash, labels at
// 10px, shortcuts at the right, a hairline between groups; checkbox rows
// with a 16px box, radio rows with a check at the right, submenus with a
// chevron.
//
// Behaviour is Base UI's Menu: roving focus, typeahead, submenus by arrow
// keys and hover, Escape and an outside click to close, focus back on the
// trigger, placement that flips at the window's edge.
//
// Composed, not configured: a menu is <Menu trigger={…}> with the parts
// below as children, so a screen can lay a menu out as it reads. The radio
// group takes its options as data so its values stay the caller's union.

import "./popup.css";
import "./menu.css";

import { Menu as Base } from "@base-ui/react/menu";
import type { ReactElement, ReactNode } from "react";

import { type Choice, choiceOf, runs } from "./choice";
import { Icon, type IconName } from "./icon";

export interface MenuProps {
  /** The element that opens it: a kit Button or IconButton. Base UI adds
   * its handlers and ARIA to it. */
  trigger: ReactElement;
  children: ReactNode;
  side?: "bottom" | "top" | "right" | "left";
  align?: "start" | "center" | "end";
  /** `narrow` is a composer's agent menu (256px, agent-orchestrator's
   * `w-64`); `wide` holds a description under each row (320px). */
  width?: "auto" | "narrow" | "wide";
}

function widthClass(width: "auto" | "narrow" | "wide"): string {
  switch (width) {
    case "auto":
      return "popup menu-popup";
    case "narrow":
      return "popup menu-popup narrow";
    case "wide":
      return "popup menu-popup wide";
  }
}

export function Menu({
  trigger,
  children,
  side = "bottom",
  align = "start",
  width = "auto",
}: MenuProps) {
  return (
    <Base.Root>
      <Base.Trigger render={trigger} />
      <Base.Portal>
        <Base.Positioner
          className="popup-positioner"
          side={side}
          align={align}
          sideOffset={6}
          collisionPadding={8}
        >
          <Base.Popup className={widthClass(width)}>{children}</Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  );
}

/** The parts every row has: an icon, the label, a description, a
 * shortcut. */
function RowContent({
  icon,
  children,
  description,
  shortcut,
}: {
  icon: IconName | undefined;
  children: ReactNode;
  description: string | undefined;
  shortcut: string | undefined;
}) {
  return (
    <>
      {icon === undefined ? null : <Icon name={icon} size={15} />}
      {description === undefined ? (
        <span className="label">{children}</span>
      ) : (
        <span className="label stacked">
          <span>{children}</span>
          <span className="description">{description}</span>
        </span>
      )}
      {shortcut === undefined ? null : (
        <span className="menu-shortcut">{shortcut}</span>
      )}
    </>
  );
}

export function MenuItem({
  icon,
  onSelect,
  shortcut,
  description,
  disabled,
  tone = "default",
  children,
}: {
  icon?: IconName;
  onSelect: () => void;
  shortcut?: string;
  description?: string;
  disabled?: boolean;
  /** `danger` for the one destructive row, kept apart by a separator. */
  tone?: "default" | "danger";
  children: ReactNode;
}) {
  return (
    <Base.Item
      className={tone === "danger" ? "menu-item danger" : "menu-item"}
      disabled={disabled}
      onClick={() => {
        onSelect();
      }}
    >
      <RowContent icon={icon} description={description} shortcut={shortcut}>
        {children}
      </RowContent>
    </Base.Item>
  );
}

export function MenuCheckboxItem({
  checked,
  onChange,
  shortcut,
  disabled,
  leading,
  children,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  shortcut?: string;
  disabled?: boolean;
  /** Drawn in the box's place (a label's colour dot, Paseo's picker),
   * with the check then at the right as a radio row draws it. */
  leading?: ReactNode;
  children: ReactNode;
}) {
  return (
    <Base.CheckboxItem
      className="menu-item"
      checked={checked}
      onCheckedChange={(next) => {
        onChange(next);
      }}
      disabled={disabled}
      closeOnClick={false}
    >
      {leading ?? (
        <span className="menu-box" aria-hidden="true">
          <Base.CheckboxItemIndicator className="menu-box-mark">
            <Icon name="check" size={12} />
          </Base.CheckboxItemIndicator>
        </span>
      )}
      <RowContent icon={undefined} description={undefined} shortcut={shortcut}>
        {children}
      </RowContent>
      {leading === undefined ? null : (
        <span className="menu-check" aria-hidden="true">
          <Base.CheckboxItemIndicator className="menu-check-mark">
            <Icon name="check" size={14} />
          </Base.CheckboxItemIndicator>
        </span>
      )}
    </Base.CheckboxItem>
  );
}

/** One choice among `options`, the chosen one checked at the right;
 * options that share a `group` sit under its heading. */
export function MenuRadioGroup<V extends string>({
  value,
  onChange,
  options,
}: {
  value: V;
  onChange: (value: V) => void;
  options: readonly Choice<V>[];
}) {
  return (
    <Base.RadioGroup
      value={value}
      onValueChange={(next: unknown) => {
        const option = choiceOf(options, next);
        if (option !== undefined) {
          onChange(option.value);
        }
      }}
    >
      {runs(options).map((run, index) =>
        run.group === undefined ? (
          run.options.map((option) => (
            <RadioRow key={option.value} option={option} />
          ))
        ) : (
          <Base.Group key={`${run.group}-${String(index)}`}>
            <Base.GroupLabel className="menu-label">
              {run.group}
            </Base.GroupLabel>
            {run.options.map((option) => (
              <RadioRow key={option.value} option={option} />
            ))}
          </Base.Group>
        ),
      )}
    </Base.RadioGroup>
  );
}

function RadioRow<V extends string>({ option }: { option: Choice<V> }) {
  return (
    <Base.RadioItem
      className="menu-item"
      value={option.value}
      disabled={option.disabled}
      closeOnClick={true}
    >
      <RowContent
        icon={option.icon}
        description={option.detail}
        shortcut={undefined}
      >
        {option.label}
      </RowContent>
      <span className="menu-check" aria-hidden="true">
        <Base.RadioItemIndicator className="menu-check-mark">
          <Icon name="check" size={14} />
        </Base.RadioItemIndicator>
      </span>
    </Base.RadioItem>
  );
}

export function MenuSeparator() {
  return <Base.Separator className="menu-separator" />;
}

/** A heading over the rows that follow it, in a group so a screen reader
 * hears it with them. */
export function MenuGroup({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <Base.Group>
      <Base.GroupLabel className="menu-label">{label}</Base.GroupLabel>
      {children}
    </Base.Group>
  );
}
