// The menu a tree row opens on a right click or a held touch (Paseo's
// file-explorer-pane.tsx 344-396 around file-actions-menu.tsx): the
// row's actions in Paseo's order and groups (`file-actions.ts`), a
// hairline between groups, Delete red.

import { Fragment, type ReactElement } from "react";

import { ContextMenu, MenuItem, MenuSeparator } from "../../ui/kit";
import { type FileActionEntry, fileActions } from "./file-actions";
import type { ExplorerActions, ExplorerTarget } from "./use-explorer-actions";

export function EntryMenu({
  trigger,
  entry,
  target,
  actions,
}: {
  trigger: ReactElement;
  entry: FileActionEntry;
  target: ExplorerTarget;
  actions: ExplorerActions;
}) {
  const rows = fileActions(entry, actions.can);
  // No row to offer: the browser's own menu stays, as on any page.
  if (rows.length === 0) return trigger;
  return (
    <ContextMenu trigger={trigger}>
      {rows.map((row) => (
        <Fragment key={row.key}>
          {row.separatorBefore ? <MenuSeparator /> : null}
          <MenuItem
            icon={row.icon}
            tone={row.destructive ? "danger" : "default"}
            onSelect={() => {
              actions.act(row.key, target);
            }}
          >
            {row.label}
          </MenuItem>
        </Fragment>
      ))}
    </ContextMenu>
  );
}
