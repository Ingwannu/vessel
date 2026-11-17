// The sidebar grouped by workspace (Paseo's sidebar shape inside Vessel's
// sidebar, coordinator ruling 2026-11-16): each machine's registry
// workspaces, their label chips, and the sessions that belong to them.
//
// [Decision Log]
// - Purpose: labels live on workspaces (ADR 0086), so the sidebar needs
//   workspace rows to show and assign them. Paseo's sidebar lists
//   workspaces, each with its agents under it and its labels on the row.
// - Membership: a session belongs to the workspace the registry snapshot
//   names for it. A session newer than the snapshot is placed by the same
//   rule the node derives (ADR 0106): a session at a project's root
//   belongs to that project's oldest Active, foreground `Local`
//   workspace. Derived members are not revisioned, so the page applies
//   the rule itself instead of waiting for the registry's long poll to
//   run out.
// - Listing: Active, foreground workspaces only (Paseo leaves archived and
//   background ones out), empty ones too, as Paseo lists a workspace with
//   no agent. Pinned first, then by their newest listed session in the
//   view's order, then by creation. Sessions in no workspace (a plain
//   folder) are kept in one trailing group per machine, shown only while
//   no label is picked: a label filter is about workspaces.
// - Titles: the workspace's own title, else its folder's name.

import type { NodeKey, SidebarView } from "../../wire/bridge";
import type {
  Label,
  RegistrySnapshot,
  SessionInfo,
  WorkspaceDescriptor,
  WorkspaceId,
} from "../../wire/proto";
import type { FleetState } from "../fleet";
import { folderName } from "../format";
import { nodeLabel } from "../nodes";
import { activeAt, listed, sessionRow, type SidebarSession } from "../sidebar";
import { passesLabels } from "./filter";
import type { LabelsState } from "./hub";
import { chipsOf } from "./merge";

/** How many sessions a workspace lists before "more". */
export const SIDEBAR_WORKSPACE_SESSIONS = 5;

/** One workspace's row and its sessions. */
export interface SidebarWorkspace {
  node: NodeKey;
  /** `null`: the machine's sessions in no workspace. */
  id: WorkspaceId | null;
  title: string;
  /** Its machine's name, for a fleet of several. */
  machine: string;
  pinned: boolean;
  /** The names it carries, in assignment order. */
  labels: readonly string[];
  /** Those names drawn from its machine's catalog. */
  chips: readonly Label[];
  sessions: readonly SidebarSession[];
  more: number;
}

function live(workspace: WorkspaceDescriptor): boolean {
  return (
    workspace.lifecycle.kind === "active" &&
    workspace.visibility === "foreground"
  );
}

/** Each project's root workspace: its oldest Active, foreground `Local`
 * one, as the node derives membership (ADR 0106). */
function rootOwners(snapshot: RegistrySnapshot): Map<string, WorkspaceId> {
  const owners = new Map<string, WorkspaceDescriptor>();
  for (const workspace of snapshot.workspaces) {
    if (!live(workspace) || workspace.backing.kind !== "local") {
      continue;
    }
    const held = owners.get(workspace.project);
    const older =
      held === undefined ||
      workspace.created_at < held.created_at ||
      (workspace.created_at === held.created_at && workspace.id < held.id);
    if (older) {
      owners.set(workspace.project, workspace);
    }
  }
  return new Map([...owners].map(([project, owner]) => [project, owner.id]));
}

/** The workspace `session` belongs to in `snapshot`, if any. */
export function workspaceOf(
  snapshot: RegistrySnapshot,
  session: SessionInfo,
  owners: Map<string, WorkspaceId> = rootOwners(snapshot),
): WorkspaceId | null {
  const member = snapshot.memberships.find(
    (membership) =>
      membership.resource.kind === "session" &&
      membership.resource.session === session.id,
  );
  if (member !== undefined) {
    return member.workspace;
  }
  const place = session.workspace;
  if (
    place === null ||
    place === undefined ||
    (place.worktree !== null && place.worktree !== undefined)
  ) {
    return null;
  }
  return owners.get(place.project) ?? null;
}

function assignedNames(
  snapshot: RegistrySnapshot,
  workspace: WorkspaceId,
): readonly string[] {
  return (
    snapshot.labels?.find((entry) => entry.workspace === workspace)?.labels ??
    []
  );
}

/** Every machine's listed workspaces, filtered by `picked` labels. */
export function sidebarWorkspaces(
  fleet: FleetState,
  labels: LabelsState,
  view: SidebarView,
  picked: readonly string[],
  limit: number = SIDEBAR_WORKSPACE_SESSIONS,
): SidebarWorkspace[] {
  return (fleet.book ?? []).flatMap((node) => {
    const snapshot = labels.registries[node.key];
    const machine = nodeLabel(node, fleet.statuses[node.key]);
    const shown = listed(fleet.sessions[node.key] ?? [], view);
    const catalog = labels.catalogs[node.key]?.labels ?? [];
    const groups = new Map<WorkspaceId | null, SessionInfo[]>();
    const owners = snapshot === undefined ? null : rootOwners(snapshot);
    for (const session of shown) {
      const id =
        snapshot === undefined || owners === null
          ? null
          : workspaceOf(snapshot, session, owners);
      groups.set(id, [...(groups.get(id) ?? []), session]);
    }
    const rows =
      snapshot === undefined
        ? []
        : snapshot.workspaces
            .filter(live)
            .map((workspace) =>
              entry(
                node.key,
                machine,
                workspace,
                assignedNames(snapshot, workspace.id),
                catalog,
                groups.get(workspace.id) ?? [],
                limit,
              ),
            );
    const kept = rows
      .filter((row) => passesLabels(row.labels, picked))
      .sort((left, right) => order(left, right, groups, view));
    const loose = groups.get(null) ?? [];
    if (picked.length > 0 || loose.length === 0) {
      return kept;
    }
    return [
      ...kept,
      {
        node: node.key,
        id: null,
        title: "Other sessions",
        machine,
        pinned: false,
        labels: [],
        chips: [],
        sessions: loose
          .slice(0, limit)
          .map((session) => sessionRow(node.key, session)),
        more: Math.max(0, loose.length - limit),
      },
    ];
  });
}

interface Ordered extends SidebarWorkspace {
  created: string;
}

function entry(
  node: NodeKey,
  machine: string,
  workspace: WorkspaceDescriptor,
  names: readonly string[],
  catalog: readonly Label[],
  sessions: readonly SessionInfo[],
  limit: number,
): Ordered {
  return {
    node,
    id: workspace.id,
    title: workspace.title ?? folderName(workspace.path) ?? workspace.path,
    machine,
    pinned: workspace.pin === "pinned",
    labels: names,
    chips: chipsOf(catalog, names),
    sessions: sessions
      .slice(0, limit)
      .map((session) => sessionRow(node, session)),
    more: Math.max(0, sessions.length - limit),
    created: workspace.created_at,
  };
}

function order(
  left: Ordered,
  right: Ordered,
  groups: Map<WorkspaceId | null, SessionInfo[]>,
  view: SidebarView,
): number {
  if (left.pinned !== right.pinned) {
    return left.pinned ? -1 : 1;
  }
  const newest = (row: Ordered): string => {
    const first = groups.get(row.id)?.[0];
    if (first === undefined) {
      return row.created;
    }
    return view.sort === "activity" ? activeAt(first) : first.created_at;
  };
  return newest(right).localeCompare(newest(left));
}
