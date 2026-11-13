// A button with its menu beside it (DESIGN.md §6.13): GOAT's git
// control, the action that makes sense now on the left, the others behind
// a chevron on the right, joined into one shape. Each half is a kit
// Button; a tooltip can say why the action is off.

import "./split-button.css";

import type { ReactNode } from "react";

import {
  Button,
  type ButtonSize,
  type ButtonVariant,
  IconButton,
} from "./button";
import type { IconName } from "./icon";
import { Menu } from "./menu";
import { Tooltip } from "./tooltip";

export function SplitButton({
  label,
  icon,
  onClick,
  disabled = false,
  busy = false,
  reason,
  menuLabel,
  menu,
  variant = "secondary",
  size = "sm",
}: {
  label: string;
  icon?: IconName;
  onClick: () => void;
  disabled?: boolean;
  busy?: boolean;
  /** Why the action is off: shown on hover. */
  reason?: string;
  /** The chevron's name ("Git action options"). */
  menuLabel: string;
  /** The menu's items. */
  menu: ReactNode;
  variant?: Extract<ButtonVariant, "secondary" | "primary">;
  size?: Exclude<ButtonSize, "md">;
}) {
  const main = (
    <Button
      variant={variant}
      size={size}
      onClick={onClick}
      disabled={disabled}
      busy={busy}
      {...(icon === undefined ? {} : { icon })}
    >
      {label}
    </Button>
  );
  return (
    <span className="split-button">
      {reason === undefined ? (
        main
      ) : (
        <Tooltip label={reason}>
          {/* A disabled button gets no hover; its wrapper does. */}
          <span className="split-button-reason">{main}</span>
        </Tooltip>
      )}
      <Menu
        align="end"
        width="wide"
        trigger={
          <IconButton
            label={menuLabel}
            icon="chevron-down"
            variant={variant}
            size={size}
          />
        }
      >
        {menu}
      </Menu>
    </span>
  );
}
