// The sidebar (DESIGN.md §2.2), agent-orchestrator's, top to bottom: the
// brand, Search, "New session", Board, then the machines with their
// sessions (or, inside Settings, its sections and a way back), and at the
// foot Settings and the theme's switch.

import "./sidebar.css";

import { Link } from "@tanstack/react-router";

import { useFleetState } from "../../context";
import { isLive } from "../../data/format";
import { selectedNode } from "../../data/nodes";
import { useWhere } from "../../data/where";
import { Icon, Kbd } from "../../ui/kit";
import { SidebarSearch } from "../palette/palette";
import { ThemeMenu } from "../theme/theme";
import { SidebarMachines } from "./sidebar-machines";
import { SidebarSettings } from "./sidebar-settings";
import { machineInView, useNewSession } from "./use-new-session";

const MARK = (
  <span className="mark" aria-hidden="true">
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M2 8.5 6 2.5 10 8.5" />
      <path d="M4 8.5h4" />
    </svg>
  </span>
);

/** A sidebar row's class: `active` marks the current place. */
export function rowClass(active: boolean, extra = ""): string {
  return `nav-item${extra}${active ? " active" : ""}`;
}

export function Sidebar() {
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
  return (
    <nav className="sidebar" role="navigation" aria-label="Sidebar">
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
          <span className="shortcut">
            <Kbd keys={["N"]} look="text" />
          </span>
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
      {inSettings ? (
        <SidebarSettings current={where.segments[1] ?? null} />
      ) : (
        <SidebarMachines />
      )}
      <div className="side-foot">
        <Link className={rowClass(inSettings)} to="/settings">
          <Icon name="settings" size={16} />
          <span className="label">Settings</span>
        </Link>
        <ThemeMenu />
      </div>
    </nav>
  );
}
