// Identity-bound transport for prepared submissions. A check sends no node RPC;
// it is not an admission ticket. Each later call checks the recipient again.
import type { NodeKey, SubmitCheck } from "../wire/bridge";
import type {
  NodeId,
  SessionId,
  SubmitPrompt,
  SubmitPromptReply,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";
import type { UploadCalls } from "./upload";

/** Check the SDK's actual negotiated support before preparing attachments. */
export function checkSubmit(
  fleet: Fleet,
  node: NodeKey,
  expectedNode: NodeId,
  request: SubmitPrompt,
): Promise<Outcome<SubmitCheck>> {
  return attempt(async () => {
    const reply = await fleet.bridge.op({
      op: "check_submit",
      node,
      expected_node: expectedNode,
      request,
    });
    if (reply.op !== "check_submit")
      throw new Error("Unexpected submit check reply");
    return reply.check;
  });
}

/** Send the retained identity/payload once. This never retries or falls back. */
export function submitPrepared(
  fleet: Fleet,
  node: NodeKey,
  expectedNode: NodeId,
  request: SubmitPrompt,
): Promise<Outcome<SubmitPromptReply>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.submit_prompt", request, expectedNode),
  );
}

/** Reuse the chunk uploader with a mandatory recipient constraint on ALL calls.
 * The prepared owner retains its blob, upload state and final references. */
export function preparedUploads(
  fleet: Fleet,
  node: NodeKey,
  expectedNode: NodeId,
  session: SessionId,
): UploadCalls {
  return {
    begin: (name, mime, size) =>
      attempt(() =>
        fleet.bridge.call(
          node,
          "attachment.begin",
          { session, name, mime, size },
          expectedNode,
        ),
      ),
    write: (upload, offset, data) =>
      attempt(() =>
        fleet.bridge.call(
          node,
          "attachment.write",
          { upload, offset, data },
          expectedNode,
        ),
      ),
    finish: (upload) =>
      attempt(() =>
        fleet.bridge.call(node, "attachment.finish", { upload }, expectedNode),
      ),
  };
}
