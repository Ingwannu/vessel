// "Machines" in the sidebar: each machine with its state and live count,
// and its sessions under it by title, as the Display menu says
// (data/sidebar.ts; grouped by status, sidebar-statuses.tsx). A machine's
// row opens it in Settings' Machines (its detail, whose "Open the board"
// scopes the board to it); a session's row opens the session; the
// heading's "+" adds a machine (the dialog the welcome, Settings and the
// palette open too).

import { Link } from "@tanstack/react-router";
import { Fragment, useMemo } from "react";

import { useFleetState } from "../../context";
import { openAddMachine } from "../../data/add-machine";
import { prefs } from "../../data/prefs";
import { sidebarMachines } from "../../data/sidebar";
import { useStore } from "../../data/store";
import { useWhere } from "../../data/where";
import { IconButton, StatusDot, Tooltip } from "../../ui/kit";
import { SessionMark } from "./session-mark";
import { SidebarViewMenu } from "./sidebar-view-menu";
import { rowClass } from "./sidebar";

export function SidebarMachines() {
  const where = useWhere();
  const state = useFleetState((value) => value);
  const view = useStore(prefs, (value) => value.values.sidebar);
  const machines = useMemo(() => sidebarMachines(state, view), [state, view]);
  const [place, node, id] = where.segments;
  const machineOpen = place === "settings" && node === "machines";
  return (
    <>
      <div className="side-label">
        <span>Machines</span>
        <SidebarViewMenu />
        <Tooltip label="Add machine">
          <IconButton
            label="Add machine"
            icon="plus"
            size="xs"
            onClick={openAddMachine}
          />
        </Tooltip>
      </div>
      <div className="side-scroll">
        {machines.map((machine) => (
          <Fragment key={machine.key}>
            <Link
              className={rowClass(
                machineOpen && where.machine === machine.key,
                machine.connected ? " node" : " node offline",
              )}
              to="/settings/$section"
              params={{ section: "machines" }}
              search={{ machine: machine.key }}
            >
              <StatusDot tone={machine.tone} label={machine.word} />
              <span className="label">{machine.label}</span>
              {machine.connected ? (
                <span className="count">{machine.live}</span>
              ) : (
                <span className="dash">—</span>
              )}
            </Link>
            {machine.sessions.map((session) => (
              <Link
                key={session.id}
                className={rowClass(
                  place === "sessions" &&
                    node === machine.key &&
                    id === session.id,
                  session.unread ? " sub unread" : " sub",
                )}
                to="/sessions/$node/$id"
                params={{ node: machine.key, id: session.id }}
              >
                <span className="slot">
                  <SessionMark mark={session.mark} unread={session.unread} />
                </span>
                <span className="label">{session.title}</span>
              </Link>
            ))}
            {machine.more > 0 ? (
              <Link
                className="nav-item more"
                to="/sessions"
                search={{ node: machine.key }}
              >
                {machine.more} more
              </Link>
            ) : null}
          </Fragment>
        ))}
      </div>
    </>
  );
}
