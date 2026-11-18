// A menu opened on the thing it acts on (Paseo's ContextMenu: a right
// click, or a touch held 500 ms), with the kit's menu rows inside
// (`menu.tsx`: MenuItem, MenuSeparator) and the menu's popup recipe.
//
// Why Base UI's ContextMenu and not the kit Menu on a hidden trigger: its
// parts are the menu's own (an item is a MenuItem), it opens at the
// pointer and handles the long press, so the rows, keys and focus are
// those of every other kit menu.

import "./popup.css";
import "./menu.css";

import { ContextMenu as Base } from "@base-ui/react/context-menu";
import type { ReactElement, ReactNode } from "react";

export function ContextMenu({
  trigger,
  children,
}: {
  /** The element it acts on; Base UI adds its handlers to it. */
  trigger: ReactElement;
  children: ReactNode;
}) {
  return (
    <Base.Root>
      <Base.Trigger render={trigger} />
      <Base.Portal>
        <Base.Positioner className="popup-positioner" collisionPadding={8}>
          {/* Paseo's file actions menu is 220 wide (file-actions-menu.tsx). */}
          <Base.Popup className="popup menu-popup actions">
            {children}
          </Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  );
}
