// A project's mark where the page draws a project (the sidebar's project
// rows, Settings > Projects): its own icon (ADR 0112), else Paseo's
// lettered identity square (TRACKER 809, `data/identity.ts`). Paseo's
// `ProjectIconView` draws the image sized, never clipped.

import "./project-icon.css";

import { type ReactElement, useEffect, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { identityFill, projectInitial } from "../../data/identity";
import {
  hasProjectIcons,
  projectIcon,
} from "../../data/workspace/project-icons";
import type { NodeKey } from "../../wire/bridge";
import type { ProjectId } from "../../wire/proto";

/** `project`'s icon address on `node`, once asked; `null` until then, on
 * an older node, or when it has none. */
export function useProjectIcon(
  node: NodeKey,
  project: ProjectId,
): string | null {
  const fleet = useFleet();
  const offered = useFleetState((state) => {
    const status = state.statuses[node];
    return hasProjectIcons(
      status?.state === "connected" ? status.protocol : null,
    );
  });
  const [address, setAddress] = useState<string | null>(null);
  useEffect(() => {
    if (!offered) return;
    let current = true;
    void projectIcon(fleet, node, project).then((found) => {
      if (current) setAddress(found);
    });
    return () => {
      current = false;
    };
  }, [fleet, node, project, offered]);
  return offered ? address : null;
}

/** The project's mark (TRACKER 809, Paseo's `ProjectIconView`): its own
 * icon, else its lettered identity square; 16 square, 20 at a compact
 * width (Paseo's `densityIconProps("md")`). Not the kit's `ico` (its
 * fill and mask would show behind a transparent icon); the sidebar's swap
 * takes any mark (`sidebar-projects.css`). */
export function ProjectIcon({
  node,
  project,
  name,
}: {
  node: NodeKey;
  project: ProjectId;
  name: string;
}): ReactElement {
  const address = useProjectIcon(node, project);
  if (address === null) {
    return (
      <span
        className="project-identity"
        aria-hidden="true"
        style={{ backgroundColor: identityFill(project) }}
      >
        {projectInitial(name)}
      </span>
    );
  }
  return (
    <img
      className="project-icon"
      src={address}
      alt=""
      aria-hidden="true"
      draggable={false}
    />
  );
}
