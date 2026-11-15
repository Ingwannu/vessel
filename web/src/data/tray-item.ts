// What a composer's attached file is at each moment, and whether the
// files can go with a prompt (the tray, `tray.ts`, moves them along).

import type {
  AgentAbilities,
  AttachmentKind,
  AttachmentLimits,
  AttachmentRef,
  NodeId,
} from "../wire/proto";
import { looksLike } from "./attach";
import type { Again, Begun, UploadCalls } from "./upload";

/** Where a tray's files go, once there is a session. */
export interface TrayTarget {
  /** Authenticated recipient captured by the upload closures, when known. */
  node?: NodeId;
  calls: UploadCalls;
  abilities: AgentAbilities | null;
  limits: AttachmentLimits;
  /** The agent's name, for a refusal's words. */
  agent: string;
}

export type ItemState =
  /** Not checked yet: no session, or the agent has not said what it
   * takes. */
  | { kind: "held" }
  | { kind: "refused"; reason: string }
  /** Checked, waiting for Send. */
  | { kind: "attached" }
  | { kind: "waiting"; resume: Begun | null }
  | { kind: "uploading"; received: number }
  | { kind: "ready"; ref: AttachmentRef; node?: NodeId }
  | { kind: "failed"; reason: string; again: Again; resume: Begun | null };

export interface TrayItem {
  /** The page's own key for the chip. */
  key: string;
  name: string;
  mime: string;
  kind: AttachmentKind;
  size: number;
  file: Blob;
  /** An image's thumbnail (a `data:` address) once it is drawn. */
  preview: string | null;
  state: ItemState;
}

/** Whether a tray's files can go with a prompt now. */
export type TrayReady =
  | { kind: "ready"; refs: AttachmentRef[] }
  | { kind: "uploading" }
  | { kind: "blocked"; reason: string };

/** Whether `items` can go with a prompt now. */
export function trayReady(
  items: readonly TrayItem[],
  node?: NodeId,
): TrayReady {
  const refs: AttachmentRef[] = [];
  let uploading = false;
  for (const item of items) {
    switch (item.state.kind) {
      case "ready":
        if (node !== undefined && item.state.node !== node) {
          return {
            kind: "blocked",
            reason:
              "The selected machine changed. Attach these files again before sending.",
          };
        }
        refs.push(item.state.ref);
        break;
      case "refused":
      case "failed":
        return {
          kind: "blocked",
          reason: `${item.name}: ${item.state.reason}`,
        };
      case "held":
      case "attached":
      case "waiting":
      case "uploading":
        uploading = true;
        break;
    }
  }
  return uploading ? { kind: "uploading" } : { kind: "ready", refs };
}

/** Why an image's first bytes are not its type's, or `null` when they
 * are (the node refuses one that is not, and drops its upload). */
export async function checked(item: TrayItem): Promise<string | null> {
  if (item.kind !== "image") {
    return null;
  }
  const head = new Uint8Array(await item.file.slice(0, 16).arrayBuffer());
  const type = item.mime.replace("image/", "").toUpperCase();
  return looksLike(item.mime, head) ? null : `Not a ${type} image`;
}
