// The sidebar grouped by status (the Display menu's "Status"; Paseo's
// status buckets in the board's words): Needs you, Working, Idle, and
// Ended when ended sessions are shown, each session with its machine.

import { Link } from "@tanstack/react-router";
import { Fragment, useMemo } from "react";

import { useFleetState } from "../../context";
import { prefs } from "../../data/prefs";
import { sidebarStatuses } from "../../data/sidebar";
import { useStore } from "../../data/store";
import { useWhere } from "../../data/where";
import { BranchChip } from "../workspace/branch-chip";
import { SessionMark } from "./session-mark";
import { rowClass } from "./sidebar";
import { SidebarViewMenu } from "./sidebar-view-menu";

export function SidebarStatuses() {
  const where = useWhere();
  const state = useFleetState((value) => value);
  const view = useStore(prefs, (value) => value.values.sidebar);
  const groups = useMemo(() => sidebarStatuses(state, view), [state, view]);
  const [place, node, id] = where.segments;
  return (
    <>
      <div className="side-label">
        <span>Sessions</span>
        <SidebarViewMenu />
      </div>
      <div className="side-scroll">
        {groups.length === 0 ? <p className="side-empty">No sessions</p> : null}
        {groups.map((group) => (
          <Fragment key={group.mark}>
            <div className="side-status">{group.label}</div>
            {group.sessions.map((session) => (
              <Link
                key={`${session.node}/${session.id}`}
                className={rowClass(
                  place === "sessions" &&
                    node === session.node &&
                    id === session.id,
                  " sub",
                )}
                to="/sessions/$node/$id"
                params={{ node: session.node, id: session.id }}
              >
                <span className="slot">
                  <SessionMark mark={session.mark} />
                </span>
                <span className="label">{session.title}</span>
                <BranchChip branch={session.branch} />
                <span className="side-machine">{session.machine}</span>
              </Link>
            ))}
            {group.more > 0 ? (
              <Link className="nav-item more" to="/sessions">
                {group.more} more
              </Link>
            ) : null}
          </Fragment>
        ))}
      </div>
    </>
  );
}
