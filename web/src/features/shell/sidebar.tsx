// The sidebar (DESIGN.md §2.2), agent-orchestrator's, top to bottom: the
// brand, Search, "New session", Board, then the machines with their
// sessions (or, inside Settings, its sections and a way back), and at the
// foot Settings and the theme's switch.

import "./sidebar.css";

import { Link } from "@tanstack/react-router";

import { useFleet, useFleetState } from "../../context";
import { isLive } from "../../data/format";
import { selectedNode } from "../../data/nodes";
import { prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { chooseTheme } from "../../data/theme";
import { useWhere } from "../../data/where";
import { COMPACT } from "../../ui/breakpoints";
import { BrandMark, Icon, Kbd } from "../../ui/kit";
import { SidebarSearch } from "../palette/palette";
import { SidebarProjects } from "../projects/sidebar-projects";
import { useMedia } from "./use-media";
import { ThemeMenu } from "../theme/theme";
import { SidebarMachines } from "./sidebar-machines";
import { SidebarUsage } from "../usage/sidebar-usage";
import { SidebarSettings } from "./sidebar-settings";
import { SidebarStatuses } from "./sidebar-statuses";
import { SidebarWorkspaces } from "./sidebar-workspaces";
import { useShortcutWords } from "../shortcuts/use-keys";
import { machineInView, useNewSession } from "./use-new-session";

// The owner's mark at the text's size (ui/kit/brand-mark.tsx).
const MARK = <BrandMark size={18} className="mark" />;

/** A sidebar row's class: `active` marks the current place. */
export function rowClass(active: boolean, extra = ""): string {
  return `nav-item${extra}${active ? " active" : ""}`;
}

export function Sidebar({
  onPick,
}: {
  /** A link in it was followed (the sheet closes then). */
  onPick?: () => void;
} = {}) {
  const fleet = useFleet();
  const where = useWhere();
  const [place] = where.segments;
  const selected = useFleetState((state) => selectedNode(state, undefined));
  const live = useFleetState((state) =>
    Object.values(state.sessions).reduce(
      (total, list) => total + list.filter(isLive).length,
      0,
    ),
  );
  const open = useNewSession();
  const inSettings = place === "settings";
  const grouping = useStore(prefs, (state) => state.values.sidebar.group);
  const newKeys = useShortcutWords("new-session");
  // A compact window (below 600px, as Paseo) has no sidebar beside the
  // page: it opens as a sheet (sidebar.css).
  const phone = useMedia(COMPACT);
  return (
    <nav
      className="sidebar"
      role="navigation"
      aria-label="Sidebar"
      onClick={(event) => {
        if (
          event.target instanceof Element &&
          event.target.closest("a") !== null
        ) {
          onPick?.();
        }
      }}
    >
      <div className="side-head">
        <Link className="brand" to="/sessions">
          {MARK}
          <span>Vessel</span>
        </Link>
      </div>
      <SidebarSearch />
      <div className="side-list">
        <a
          className={rowClass(place === "new")}
          href="/new"
          aria-current={place === "new" ? "page" : undefined}
          onClick={(event) => {
            if (event.metaKey || event.ctrlKey || event.shiftKey) {
              return;
            }
            event.preventDefault();
            open(machineInView(where) ?? selected);
          }}
        >
          <Icon name="compose" size={16} />
          <span className="label">New session</span>
          {newKeys === null ? null : (
            <span className="shortcut">
              <Kbd keys={[newKeys]} look="text" />
            </span>
          )}
        </a>
        <Link
          className={rowClass(
            place === "sessions" && where.segments.length === 1,
          )}
          to="/sessions"
        >
          <Icon name="board" size={16} />
          <span className="label">Board</span>
          <span className="count">{live}</span>
        </Link>
      </div>
      {/* The projects are read when their group is first drawn; on a phone
          the sidebar beside the page is hidden, so its group waits for the
          sheet (no `project.list` at a phone's start). */}
      {inSettings || (onPick === undefined && phone) ? null : (
        <SidebarProjects />
      )}
      {inSettings ? (
        <SidebarSettings current={where.segments[1] ?? null} />
      ) : grouping === "machine" ? (
        <SidebarMachines />
      ) : grouping === "workspace" ? (
        <SidebarWorkspaces />
      ) : (
        <SidebarStatuses />
      )}
      <SidebarUsage />
      <div className="side-foot">
        <Link className={rowClass(inSettings)} to="/settings">
          <Icon name="settings" size={16} />
          <span className="label">Settings</span>
        </Link>
        <ThemeMenu
          choose={(preference) => {
            void chooseTheme(fleet, preference);
          }}
        />
      </div>
    </nav>
  );
}
