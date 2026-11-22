import type { Fleet } from "./fleet";
import type { NodeKey } from "../wire/bridge";
import type { AppendSystemPrompt, NodeSettings } from "../wire/proto";
import { attempt, type Outcome } from "./outcome";

/** Optional fields tell the card which controls the owning node serves. */
export function readHostAgentSettings(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<NodeSettings>> {
  return attempt(() => fleet.bridge.call(node, "node.settings", {}));
}

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

export function setVesselTools(
  fleet: Fleet,
  node: NodeKey,
  enabled: boolean,
): Promise<Outcome<boolean>> {
  return attempt(async () => {
    const settings = await fleet.bridge.call(node, "node.set_settings", {
      vessel_tools: enabled,
    });
    if (settings.vessel_tools == null) {
      throw new Error("This machine did not confirm the tool setting.");
    }
    return settings.vessel_tools;
  });
}
