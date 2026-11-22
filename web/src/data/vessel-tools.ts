import type { Fleet } from "./fleet";
import type { NodeKey } from "../wire/bridge";
import { attempt, type Outcome } from "./outcome";

/** A reply without the field is an older or unassigned host. */
export function readVesselTools(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<boolean | null>> {
  return attempt(async () => {
    const settings = await fleet.bridge.call(node, "node.settings", {});
    return settings.vessel_tools ?? null;
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
