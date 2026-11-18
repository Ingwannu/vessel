// A project's own icon where the page draws a project (the sidebar's
// project rows, Settings > Projects), else the folder mark it had (ADR
// 0112; Paseo's `ProjectIconView` draws the image sized, never clipped).

import "./project-icon.css";

import { type ReactElement, useEffect, useState } from "react";

import { useFleet, useFleetState } from "../../context";
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

/** The icon at `size`, or `fallback` (the folder mark). The image takes
 * the `ico` class so a row's hover swap treats it as the mark. */
export function ProjectIcon({
  node,
  project,
  size,
  fallback,
}: {
  node: NodeKey;
  project: ProjectId;
  size: 14 | 16;
  fallback: ReactElement;
}): ReactElement {
  const address = useProjectIcon(node, project);
  if (address === null) return fallback;
  return (
    <img
      className="ico project-icon"
      src={address}
      alt=""
      aria-hidden="true"
      width={size}
      height={size}
      draggable={false}
    />
  );
}
