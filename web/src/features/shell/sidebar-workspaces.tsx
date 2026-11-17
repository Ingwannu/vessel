// The sidebar grouped by workspace (the Display menu's "Workspace";
// Paseo's sidebar shape inside Vessel's): each machine's registry
// workspaces, their label chips under the name, a row menu with the
// Labels picker, and their sessions under them (data/workspace-labels/
// groups.ts decides who belongs where and what the filter keeps).

import { Link } from "@tanstack/react-router";
import { Fragment, useMemo, useState } from "react";

import { useFleetState } from "../../context";
import { prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { useWhere } from "../../data/where";
import {
  type SidebarWorkspace,
  sidebarWorkspaces,
} from "../../data/workspace-labels/groups";
import type { NodeKey } from "../../wire/bridge";
import type { WorkspaceId } from "../../wire/proto";
import { Icon, IconButton, Menu } from "../../ui/kit";
import { BranchChip } from "../workspace/branch-chip";
import { LabelChips } from "../workspace-labels/chip";
import { CreateLabelDialog } from "../workspace-labels/create-dialog";
import { LabelsSubmenu } from "../workspace-labels/picker";
import { useLabels } from "../workspace-labels/use-labels";
import { usePruneFilter } from "../workspace-labels/use-prune-filter";
import { SessionMark } from "./session-mark";
import { rowClass } from "./sidebar";
import { SidebarViewMenu } from "./sidebar-view-menu";

export function SidebarWorkspaces() {
  const where = useWhere();
  const fleet = useFleetState((value) => value);
  const view = useStore(prefs, (value) => value.values.sidebar);
  const { hub, state } = useLabels();
  usePruneFilter(state, view.labels);
  const rows = useMemo(
    () => sidebarWorkspaces(fleet, state, view, view.labels),
    [fleet, state, view],
  );
  const [creating, setCreating] = useState<{
    node: NodeKey;
    workspace: WorkspaceId;
  } | null>(null);
  const machines = (fleet.book ?? []).length;
  const [place, node, id] = where.segments;
  return (
    <>
      <div className="side-label">
        <span>Workspaces</span>
        <SidebarViewMenu />
      </div>
      <div className="side-scroll">
        {rows.length === 0 ? (
          <p className="side-empty">
            {view.labels.length > 0
              ? "No workspace has these labels"
              : "No workspaces"}
          </p>
        ) : null}
        {rows.map((row) => (
          <Fragment key={`${row.node}/${row.id ?? ""}`}>
            <WorkspaceHead
              row={row}
              several={machines > 1}
              onCreate={(workspace) => {
                setCreating({ node: row.node, workspace });
              }}
            />
            {row.sessions.map((session) => (
              <Link
                key={session.id}
                className={rowClass(
                  place === "sessions" &&
                    node === row.node &&
                    id === session.id,
                  session.unread ? " sub unread" : " sub",
                )}
                to="/sessions/$node/$id"
                params={{ node: row.node, id: session.id }}
              >
                <span className="slot">
                  <SessionMark mark={session.mark} unread={session.unread} />
                </span>
                <span className="label">{session.title}</span>
                <BranchChip branch={session.branch} />
              </Link>
            ))}
            {row.more > 0 ? (
              <Link
                className="nav-item more"
                to="/sessions"
                search={{ node: row.node }}
              >
                {row.more} more
              </Link>
            ) : null}
          </Fragment>
        ))}
      </div>
      {creating === null ? null : (
        <CreateLabelDialog
          hub={hub}
          node={creating.node}
          workspace={creating.workspace}
          onClose={() => {
            setCreating(null);
          }}
        />
      )}
    </>
  );
}

/** A workspace's row: its folder, its name, its machine in a fleet of
 * several, its row menu, and its chips under the name. */
function WorkspaceHead({
  row,
  several,
  onCreate,
}: {
  row: SidebarWorkspace;
  several: boolean;
  onCreate: (workspace: WorkspaceId) => void;
}) {
  const workspace = row.id;
  return (
    <div className="side-workspace">
      <div className="nav-item workspace">
        <Icon name={workspace === null ? "list" : "folder"} size={16} />
        <span className="label">{row.title}</span>
        {several ? <span className="side-machine">{row.machine}</span> : null}
        {workspace === null ? null : (
          <span className="side-workspace-menu">
            <Menu
              align="end"
              trigger={
                <IconButton label="Workspace actions" icon="more" size="xs" />
              }
            >
              <LabelsSubmenu
                node={row.node}
                workspace={workspace}
                assigned={row.labels}
                onCreate={() => {
                  onCreate(workspace);
                }}
              />
            </Menu>
          </span>
        )}
      </div>
      {row.chips.length === 0 ? null : (
        <div className="side-workspace-meta">
          <LabelChips labels={row.chips} />
        </div>
      )}
    </div>
  );
}
