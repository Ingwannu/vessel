// "Machines" in the sidebar: each machine with its state and live count,
// and its live sessions under it by title (data/sidebar.ts). A machine's
// row opens the board scoped to it; a session's row opens the session.

import { Link } from "@tanstack/react-router";
import { Fragment, useMemo } from "react";

import { useFleetState } from "../../context";
import { sidebarMachines } from "../../data/sidebar";
import { useWhere } from "../../data/where";
import { StatusDot } from "../../ui/kit";
import { SessionMark } from "./session-mark";
import { rowClass } from "./sidebar";

export function SidebarMachines() {
  const where = useWhere();
  const state = useFleetState((value) => value);
  const machines = useMemo(() => sidebarMachines(state), [state]);
  const [place, node, id] = where.segments;
  return (
    <>
      <div className="side-label">
        <span>Machines</span>
      </div>
      <div className="side-scroll">
        {machines.map((machine) => (
          <Fragment key={machine.key}>
            <Link
              className={rowClass(
                place === "sessions" &&
                  where.segments.length === 1 &&
                  where.node === machine.key,
                machine.connected ? " node" : " node offline",
              )}
              to="/sessions"
              search={{ node: machine.key }}
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
                  " sub",
                )}
                to="/sessions/$node/$id"
                params={{ node: machine.key, id: session.id }}
              >
                <span className="slot">
                  <SessionMark mark={session.mark} />
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
