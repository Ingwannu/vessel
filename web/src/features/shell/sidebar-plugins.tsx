// The sidebar's plugin rows (ADR 0115, "Surfaces"): Paseo's
// `addSidebarHeaderItem` / `addSidebarFooterItem` rows
// (public-docs/plugins/reference.md:718-846), each opening one of its
// plugin's screens. Paseo gives plugin rows one shared icon
// (`Blocks`, packages/app/src/screens/settings/sidebar/
// sidebar-nav-section.tsx:50-64); the title is the contribution's.
//
// [Decision Log]
// - Purpose: a plugin's screen needs a way in besides the palette.
// - Choice: header rows after the page's own rows at the top of the list,
//   footer rows above Settings, every connected machine's in machine and
//   declaration order; a row is active while its screen is open. A device
//   without admin sees them too: a screen is drawn read-only there.

import "./sidebar-plugins.css";

import { Link } from "@tanstack/react-router";

import { useFleetState } from "../../context";
import { pluginCatalogs } from "../../data/plugin-catalog";
import {
  rememberedScreens,
  type SidebarItem,
  sidebarItems,
} from "../../data/plugin-surfaces";
import { useStore } from "../../data/store";
import { shownPluginScreen, useWhere } from "../../data/where";
import type { SidebarPlacement } from "../../wire/proto";
import { Icon } from "../../ui/kit";
import { rowClass } from "./sidebar";

/** The rows of `placement`, merged across the connected machines. */
function usePluginRows(placement: SidebarPlacement): SidebarItem[] {
  const catalogs = useStore(pluginCatalogs, (all) => all);
  const remembered = useStore(rememberedScreens, (all) => all);
  const statuses = useFleetState((state) => state.statuses);
  const current = shownPluginScreen(useWhere());
  return sidebarItems(catalogs, placement, {
    online: (key) => statuses[key]?.state === "connected",
    current,
    remembered,
  });
}

function Rows({ items }: { items: readonly SidebarItem[] }) {
  const [first, , plugin, screen] = useWhere().segments;
  return items.map((item) => {
    const target = item.target;
    // A merged row is active on its screen from any machine.
    const active =
      first === "plugin" &&
      plugin === target.plugin &&
      screen === target.screen;
    return (
      <Link
        key={item.key}
        className={rowClass(active)}
        to="/plugin/$node/$plugin/$screen"
        params={{
          node: target.node,
          plugin: target.plugin,
          screen: target.screen,
        }}
      >
        <Icon name="blocks" size={16} />
        <span className="label">{item.title}</span>
      </Link>
    );
  });
}

/** The header rows, inside the list at the top of the sidebar. */
export function SidebarPluginItems({
  placement,
}: {
  placement: SidebarPlacement;
}) {
  return <Rows items={usePluginRows(placement)} />;
}

/** The footer rows, in their own block above Settings; nothing when there
 * are none, so the foot keeps its place at the bottom. */
export function SidebarPluginFoot() {
  const items = usePluginRows("footer");
  if (items.length === 0) return null;
  return (
    <div className="side-plugin-foot">
      <Rows items={items} />
    </div>
  );
}
