// A project's own icon (ADR 0112, Paseo's `projects/icon-cache.ts` over
// `project_icon_request`): asked once per project and page load
// (`project.icon`), drawn as an image from a `data:` address carrying the
// type the node decided from the bytes.
//
// [Decision Log]
// - Purpose: projects in the sidebar and in Settings > Projects show their
//   own mark, as Paseo's do.
// - Constraint: the page keeps nothing in browser storage (ADR 0107: a new
//   origin each launch) and its policy allows images from itself and
//   `data:` only; an SVG must never be put into the page as markup.
// - Choice: a promise per node and project in memory, so every row of
//   one project shares one call (a failed call is dropped and asked again
//   by the next row drawn); the answer becomes a `data:` address
//   with the node's content type, drawn by an `img`, which runs no script
//   and reaches nothing. A failed call or no icon is `null`: the folder
//   mark stays, as on a node older than the protocol.
// - Cost: an icon changed on disk shows on the next page load (Paseo
//   refreshes its cache when the project's files change).

import type { NodeKey } from "../../wire/bridge";
import type { IconImage, IconType, ProjectId } from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt } from "../outcome";

/** `project.icon`'s protocol: a copy of vessel-proto's
 * `HOST_FOLDERS_PROTOCOL` (assigned 32 at its node-line merge);
 * the two must stay equal. */
export const HOST_FOLDERS_PROTOCOL = 32;

/** Whether a node speaking `protocol` answers `project.icon`. */
export function hasProjectIcons(protocol: number | null): boolean {
  return protocol !== null && protocol >= HOST_FOLDERS_PROTOCOL;
}

const CONTENT_TYPES: Readonly<Record<IconType, string>> = {
  png: "image/png",
  ico: "image/x-icon",
  svg: "image/svg+xml",
};

/** The address an `img` draws `icon` from: the node's type, its bytes. */
export function iconAddress(icon: IconImage): string {
  return `data:${CONTENT_TYPES[icon.type]};base64,${icon.bytes}`;
}

const asked = new Map<string, Promise<string | null>>();

/** `project`'s icon on `node` as an address, or `null` (none, or the call
 * failed); one call per project and page load. */
export function projectIcon(
  fleet: Fleet,
  node: NodeKey,
  project: ProjectId,
): Promise<string | null> {
  const key = `${node}\u0000${project}`;
  const known = asked.get(key);
  if (known !== undefined) return known;
  const answer = attempt(() =>
    fleet.bridge.call(node, "project.icon", { project }),
  ).then((outcome) => {
    // A failed call is asked again by the next row drawn; only an answer
    // (an icon or none) is kept for the page load.
    if (!outcome.ok) {
      asked.delete(key);
      return null;
    }
    const icon = outcome.value.icon;
    return icon === undefined || icon === null ? null : iconAddress(icon);
  });
  asked.set(key, answer);
  return answer;
}
