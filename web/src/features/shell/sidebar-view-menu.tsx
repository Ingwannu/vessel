// The sidebar's "Display" menu (Paseo's: a sliders button by the list's
// heading): group by machine, by status or by workspace, sort by creation
// or by last activity, and whether ended sessions are listed; grouped by
// workspace, the Labels filter and "Manage labels…" too (Paseo offers
// both with its workspace grouping). Kept by the bridge, so every tab and
// every launch shows the same list.

import { useFleet } from "../../context";
import { prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { labelManager } from "../../data/workspace-labels/hub";
import {
  IconButton,
  Menu,
  MenuGroup,
  MenuItem,
  MenuRadioGroup,
  MenuSeparator,
  MenuSwitchItem,
  Tooltip,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { PreferenceChange } from "../../wire/bridge";
import { LabelFilterMenu } from "../workspace-labels/filter-menu";

export function SidebarViewMenu() {
  const fleet = useFleet();
  const view = useStore(prefs, (state) => state.values.sidebar);
  const change = (next: PreferenceChange) => {
    void prefer(fleet, next).then((outcome) => {
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
  return (
    <Menu
      align="end"
      trigger={
        <Tooltip label="Display">
          <IconButton label="Display" icon="sliders" size="xs" />
        </Tooltip>
      }
    >
      <MenuGroup label="Grouping">
        <MenuRadioGroup
          value={view.group}
          onChange={(group) => {
            change({ set: "sidebar_group", value: group });
          }}
          options={[
            { value: "machine", label: "Machine" },
            { value: "status", label: "Status" },
            { value: "workspace", label: "Workspace" },
          ]}
        />
      </MenuGroup>
      {view.group === "workspace" ? (
        <>
          <MenuSeparator />
          <LabelFilterMenu picked={view.labels} />
          <MenuItem
            icon="tag"
            onSelect={() => {
              labelManager.set(true);
            }}
          >
            Manage labels…
          </MenuItem>
        </>
      ) : null}
      <MenuSeparator />
      <MenuGroup label="Sort">
        <MenuRadioGroup
          value={view.sort}
          onChange={(sort) => {
            change({ set: "sidebar_sort", value: sort });
          }}
          options={[
            { value: "created", label: "Created" },
            { value: "activity", label: "Last activity" },
          ]}
        />
      </MenuGroup>
      <MenuSeparator />
      <MenuSwitchItem
        checked={view.ended}
        onChange={(on) => {
          change({ set: "sidebar_ended", value: on });
        }}
      >
        Show ended sessions
      </MenuSwitchItem>
    </Menu>
  );
}
