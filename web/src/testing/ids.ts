// Tests' handles: a fixture names a session "s1" and a machine "laptop",
// shorter than a node's UUID, so it cannot go through the page's guards
// (`bridge/ids.ts`). This is the one function that brands without
// checking a handle's shape; only tests and their fixtures import it
// (eslint keeps it out of everything else). It sits outside the layers so
// the bridge's tests and data's alike can use it.

import type { LaunchId, NodeKey, Nonce } from "../wire/bridge";
import type {
  BlobHash,
  DeviceId,
  HarnessSessionId,
  NodeId,
  NodePublicKey,
  OptionId,
  OptionValueId,
  PermissionId,
  PermissionOptionId,
  SessionId,
  ToolCallId,
  TransferId,
} from "../wire/proto";

/** The text handles a fixture may name, by their wire names. */
interface Handles {
  BlobHash: BlobHash;
  DeviceId: DeviceId;
  HarnessSessionId: HarnessSessionId;
  LaunchId: LaunchId;
  NodeId: NodeId;
  NodeKey: NodeKey;
  NodePublicKey: NodePublicKey;
  Nonce: Nonce;
  OptionId: OptionId;
  OptionValueId: OptionValueId;
  PermissionId: PermissionId;
  PermissionOptionId: PermissionOptionId;
  SessionId: SessionId;
  ToolCallId: ToolCallId;
  TransferId: TransferId;
}

/** True for any text but the empty one, which no handle of `kind` is. */
function named<K extends keyof Handles>(
  kind: K,
  text: string,
): text is Handles[K] {
  return kind.length > 0 && text.length > 0;
}

/** A fixture's `text` as a handle of `kind`
 * (`fixtureId("SessionId", "s1")`). */
export function fixtureId<K extends keyof Handles>(
  kind: K,
  text: string,
): Handles[K] {
  if (named(kind, text)) {
    return text;
  }
  throw new Error(`A fixture's ${kind} is never empty.`);
}
