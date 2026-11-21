// A titled group of menu rows (the kit menu's, `menu.tsx`), with an
// optional accessory at the heading's end. Its own file to keep the menu
// within the page's file budget.

import { Menu as Base } from "@base-ui/react/menu";
import type { ReactNode } from "react";

/** A heading over the rows that follow it, in a group so a screen reader
 * hears it with them; `accessory` sits at the heading's end (Paseo's
 * group accessory, "Edit profiles" beside "Terminal profiles"). */
export function MenuGroup({
  label,
  accessory,
  children,
}: {
  label: string;
  accessory?: ReactNode;
  children: ReactNode;
}) {
  const heading = (
    <Base.GroupLabel className="menu-label">{label}</Base.GroupLabel>
  );
  return (
    <Base.Group>
      {accessory === undefined ? (
        heading
      ) : (
        <div className="menu-label-row">
          {heading}
          {accessory}
        </div>
      )}
      {children}
    </Base.Group>
  );
}
