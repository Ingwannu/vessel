// "Projects" in the sidebar (agent-orchestrator's project tree): each
// connected machine's projects, a row each (a folder that opens to show
// the project's live sessions, its name, and on hover a "+" that starts a
// session there), the first ten, then "Show N more projects"; the
// heading's "+" leads to Settings › Projects to add one.
//
// Sessions stay under their machines too (the sidebar's own grouping);
// a project's are shown only while it is open, so the list does not
// repeat them at rest.

import "./sidebar-projects.css";

import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { isLive } from "../../data/format";
import { labelOf } from "../../data/nodes";
import { sessionMark } from "../../data/sidebar";
import { sessionName } from "../../data/titles";
import { useWhere } from "../../data/where";
import {
  projectAmong,
  projectStore,
  useProjects,
} from "../../data/workspace/project-store";
import { useStore } from "../../data/store";
import { wantedPlace } from "../../data/workspace/wanted-place";
import { Icon, IconButton, Tooltip } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { ProjectInfo, SessionInfo } from "../../wire/proto";
import { SessionMark } from "../shell/session-mark";
import { rowClass } from "../shell/sidebar";
import { useNewSession } from "../shell/use-new-session";

/** Projects listed before "Show N more", and sessions under one. */
const PROJECTS = 10;
const SESSIONS = 6;

/** Whether `session` runs in `project` (placed there, or in its folder). */
function inProject(
  session: SessionInfo,
  project: ProjectInfo,
  all: readonly ProjectInfo[],
): boolean {
  const workspace = session.workspace ?? null;
  if (workspace !== null) {
    return workspace.project === project.id;
  }
  return projectAmong(all, session.spec.cwd)?.id === project.id;
}

function MachineProjects({
  node,
  many,
  open,
  onToggle,
}: {
  node: NodeKey;
  /** Whether more than one machine is listed (rows then name theirs). */
  many: boolean;
  open: ReadonlySet<string>;
  onToggle: (key: string) => void;
}) {
  const fleet = useFleet();
  const where = useWhere();
  const projects = useProjects(fleet, node);
  const label = useFleetState((state) => labelOf(state, node));
  const sessions = useFleetState((state) => state.sessions[node]);
  const start = useNewSession();
  const [all, setAll] = useState(false);
  if (projects.kind !== "ready") {
    return null;
  }
  const list = projects.list.projects;
  const shown = all ? list : list.slice(0, PROJECTS);
  const [place, inNode, id] = where.segments;
  return (
    <>
      {shown.map((project) => {
        const key = `${node}\u0000${project.id}`;
        const isOpen = open.has(key);
        const mine = (sessions ?? []).filter(
          (session) => isLive(session) && inProject(session, project, list),
        );
        return (
          <div key={key} className="side-project">
            <button
              type="button"
              className={rowClass(false, " project")}
              aria-expanded={isOpen}
              onClick={() => {
                onToggle(key);
              }}
            >
              <span className="project-mark" aria-hidden="true">
                <Icon name={isOpen ? "folder-open" : "folder"} size={14} />
                <Icon name="chevron" size={14} />
              </span>
              <span className="label">{project.name}</span>
              {many ? <span className="project-machine">{label}</span> : null}
              {mine.length === 0 ? null : (
                <span className="count">{mine.length}</span>
              )}
            </button>
            <span className="project-actions">
              <Tooltip label="New session here">
                <IconButton
                  label={`New session in ${project.name}`}
                  icon="plus"
                  size="xs"
                  variant="ghost"
                  onClick={() => {
                    wantedPlace.set({ node, project: project.id });
                    start(node);
                  }}
                />
              </Tooltip>
            </span>
            {isOpen
              ? mine.slice(0, SESSIONS).map((session) => (
                  <Link
                    key={session.id}
                    className={rowClass(
                      place === "sessions" &&
                        inNode === node &&
                        id === session.id,
                      " sub in-project",
                    )}
                    to="/sessions/$node/$id"
                    params={{ node, id: session.id }}
                  >
                    <span className="slot">
                      <SessionMark mark={sessionMark(session)} />
                    </span>
                    <span className="label">{sessionName(session)}</span>
                  </Link>
                ))
              : null}
            {isOpen && mine.length === 0 ? (
              <p className="side-empty in-project">No live sessions</p>
            ) : null}
          </div>
        );
      })}
      {list.length > PROJECTS ? (
        <button
          type="button"
          className="nav-item more"
          onClick={() => {
            setAll(!all);
          }}
        >
          {all
            ? "Show fewer projects"
            : `Show ${String(list.length - PROJECTS)} more projects`}
        </button>
      ) : null}
    </>
  );
}

export function SidebarProjects() {
  const navigate = useNavigate();
  const book = useFleetState((state) => state.book);
  // Rows name their machine only when two or more machines have projects.
  const holding = useStore(
    projectStore,
    (all) =>
      Object.values(all).filter(
        (view) => view?.kind === "ready" && view.list.projects.length > 0,
      ).length,
  );
  const statuses = useFleetState((state) => state.statuses);
  const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set());
  const connected = (book ?? [])
    .map((node) => node.key)
    .filter((key) => statuses[key]?.state === "connected");
  if (connected.length === 0) {
    return null;
  }
  const toggle = (key: string) => {
    const next = new Set(open);
    if (!next.delete(key)) {
      next.add(key);
    }
    setOpen(next);
  };
  return (
    <section className="side-projects" aria-label="Projects">
      <div className="side-label">
        <span>Projects</span>
        <Tooltip label="Add project">
          <IconButton
            label="Add project"
            icon="plus"
            size="xs"
            onClick={() => {
              void navigate({
                to: "/settings/$section",
                params: { section: "projects" },
              });
            }}
          />
        </Tooltip>
      </div>
      <div className="side-projects-list">
        {connected.map((node) => (
          <MachineProjects
            key={node}
            node={node}
            many={holding > 1}
            open={open}
            onToggle={toggle}
          />
        ))}
      </div>
    </section>
  );
}
