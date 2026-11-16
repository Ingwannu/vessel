// A menu row that opens a menu of its own, to the side (menu.tsx's
// submenu part, kept apart so each file stays within its budget).

import "./popup.css";
import "./menu.css";

import { Menu as Base } from "@base-ui/react/menu";
import type { ReactNode } from "react";

import { Icon, type IconName } from "./icon";

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
          sideOffset={9}
          alignOffset={-5}
          collisionPadding={8}
        >
          <Base.Popup className="popup menu-popup">{children}</Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.SubmenuRoot>
  );
}
