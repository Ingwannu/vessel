// The sidebar's "Display" menu (Paseo's: a sliders button by the list's
// heading): group by machine or by status, sort by creation or by last
// activity, and whether ended sessions are listed. Kept by the bridge, so
// every tab and every launch shows the same list.

import { useFleet } from "../../context";
import { prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import {
  IconButton,
  Menu,
  MenuGroup,
  MenuRadioGroup,
  MenuSeparator,
  MenuSwitchItem,
  Tooltip,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { PreferenceChange } from "../../wire/bridge";

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
          ]}
        />
      </MenuGroup>
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
