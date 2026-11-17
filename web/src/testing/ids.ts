// Tests' handles: a fixture names a session "s1" and a machine "laptop",
// shorter than a node's UUID, so it cannot go through the page's guards
// (`bridge/ids.ts`). This is the one function that brands without
// checking a handle's shape; only tests and their fixtures import it
// (eslint keeps it out of everything else). It sits outside the layers so
// the bridge's tests and data's alike can use it.

import type { LaunchId, NodeKey, Nonce } from "../wire/bridge";
import type {
  AttachmentId,
  BlobHash,
  CommandsDigest,
  ContentHash,
  DeviceId,
  FieldName,
  HarnessSessionId,
  NodeId,
  NodePublicKey,
  OptionId,
  OptionValueId,
  PermissionId,
  PermissionOptionId,
  ProjectId,
  QuestionId,
  QueueEntryId,
  SessionId,
  ToolCallId,
  TransferId,
  UploadId,
  UsageAccountKey,
  UsageReportId,
  UsageSourceId,
  WorktreeId,
} from "../wire/proto";

/** The text handles a fixture may name, by their wire names. */
interface Handles {
  AttachmentId: AttachmentId;
  BlobHash: BlobHash;
  CommandsDigest: CommandsDigest;
  ContentHash: ContentHash;
  DeviceId: DeviceId;
  FieldName: FieldName;
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
  ProjectId: ProjectId;
  QuestionId: QuestionId;
  QueueEntryId: QueueEntryId;
  SessionId: SessionId;
  ToolCallId: ToolCallId;
  TransferId: TransferId;
  UploadId: UploadId;
  UsageAccountKey: UsageAccountKey;
  UsageReportId: UsageReportId;
  UsageSourceId: UsageSourceId;
  WorktreeId: WorktreeId;
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
