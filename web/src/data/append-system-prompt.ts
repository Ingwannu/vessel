import type { NodeKey } from "../wire/bridge";
import type { AppendSystemPrompt } from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";

/** Read the private value only after a person opens its editor. */
export function readAppendSystemPrompt(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<AppendSystemPrompt>> {
  return attempt(() =>
    fleet.bridge.call(node, "node.append_system_prompt", {}),
  );
}

export function saveAppendSystemPrompt(
  fleet: Fleet,
  node: NodeKey,
  prompt: AppendSystemPrompt,
): Promise<Outcome<void>> {
  return attempt(async () => {
    await fleet.bridge.call(node, "node.set_settings", {
      append_system_prompt: prompt,
    });
  });
}
