// A menu row that opens a menu of its own, to the side (menu.tsx's
// submenu part, kept apart so each file stays within its budget).
//
// [Decision Log]
// - Reference: Paseo's menu engine (menu-surface.tsx MenuFlyout and
//   anchor.ts): the flyout opens to the right of its row, aligned with
//   the row's top, 5px over the row (SUBMENU_OVERLAP: no strip between
//   the two where the pointer belongs to neither), and is clamped into
//   the window. It never flips to the other side: a submenu that moved
//   would leave the pointer travelling toward it.
// - Before: 9px beside the row, and Base UI's default avoidance (flip to
//   the other side, else fall back below the row, over the parent).
// - Choice: the same as Paseo: inline-end, -5px, shifted along both axes
//   to stay in the window (on a phone it then covers part of the parent,
//   as Paseo's does). Base UI's hover-intent shield stays: while a
//   hover-opened submenu is open the parent's other rows take no pointer
//   until the submenu closes (the pointer leaves it, or Escape).

import "./popup.css";
import "./menu.css";

import { Menu as Base } from "@base-ui/react/menu";
import type { ReactNode } from "react";

import { Icon, type IconName } from "./icon";

/** Never flip; shift along both axes to stay in the window. */
const CLAMPED = {
  side: "shift",
  align: "shift",
  fallbackAxisSide: "none",
} as const;

/** A row that opens a menu of its own, to the side. */
export function MenuSubmenu({
  label,
  icon,
  value,
  children,
}: {
  label: string;
  icon?: IconName;
  /** The current value, muted, before the chevron (the reference's
   * "Model  Opus ›"). */
  value?: string;
  children: ReactNode;
}) {
  return (
    <Base.SubmenuRoot>
      <Base.SubmenuTrigger className="menu-item">
        {icon === undefined ? null : <Icon name={icon} size={15} />}
        <span className="label">{label}</span>
        {value === undefined ? null : (
          <span className="menu-value">{value}</span>
        )}
        <Icon name="chevron" size={14} />
      </Base.SubmenuTrigger>
      <Base.Portal>
        <Base.Positioner
          className="popup-positioner"
          sideOffset={-5}
          alignOffset={-5}
          collisionPadding={8}
          collisionAvoidance={CLAMPED}
        >
          <Base.Popup className="popup menu-popup">{children}</Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.SubmenuRoot>
  );
}
