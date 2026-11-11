// A machine's "…" menu (GOAT's environment row's): Rename, Make default,
// then Forget… apart under a separator, in the danger colour, behind a
// confirming dialog. What a machine offers is `machineActions`' decision:
// this machine has no Forget and no Rename (it is not a book entry).

import { machineActions } from "../../data/machines";
import type { BookNode } from "../../wire/bridge";
import { IconButton, Menu, MenuItem, MenuSeparator } from "../../ui/kit";

/** What the menu asks its row's page to do. */
export type MachineAction = "rename" | "default" | "forget";

export function MachineMenu({
  node,
  label,
  onAction,
}: {
  node: BookNode;
  label: string;
  onAction: (action: MachineAction) => void;
}) {
  const offers = machineActions(node);
  if (!offers.rename && !offers.makeDefault && !offers.forget) {
    return null;
  }
  return (
    <Menu
      align="end"
      trigger={
        <IconButton
          label={`Actions for ${label}`}
          icon="more"
          variant="ghost"
          size="sm"
        />
      }
    >
      {offers.rename ? (
        <MenuItem
          icon="edit"
          onSelect={() => {
            onAction("rename");
          }}
        >
          Rename…
        </MenuItem>
      ) : null}
      {offers.makeDefault ? (
        <MenuItem
          icon="check"
          onSelect={() => {
            onAction("default");
          }}
        >
          Make default
        </MenuItem>
      ) : null}
      {offers.forget ? (
        <>
          <MenuSeparator />
          <MenuItem
            icon="close"
            tone="danger"
            onSelect={() => {
              onAction("forget");
            }}
          >
            Forget…
          </MenuItem>
        </>
      ) : null}
    </Menu>
  );
}
