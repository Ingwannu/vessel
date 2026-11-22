import type { NodeKey } from "../wire/bridge";
import type { RestartRecovery, SetRestartRecovery } from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";

export const DEFAULT_RESUME_PROMPT = "Resume working.";

const DEFAULT_POLICY: RestartRecovery = {
  enabled: false,
  prompt: DEFAULT_RESUME_PROMPT,
};

/** Only a successful explicit read distinguishes default Off from an older peer. */
export function readRestartRecovery(
  fleet: Fleet,
  node: NodeKey,
): Promise<Outcome<RestartRecovery>> {
  return attempt(async () => {
    const settings = await fleet.bridge.call(node, "node.settings", {
      include_restart_recovery: true,
    });
    return settings.restart_recovery ?? DEFAULT_POLICY;
  });
}

export function saveRestartRecovery(
  fleet: Fleet,
  node: NodeKey,
  change: SetRestartRecovery,
): Promise<Outcome<RestartRecovery>> {
  return attempt(async () => {
    const settings = await fleet.bridge.call(node, "node.set_settings", {
      restart_recovery: change,
    });
    return settings.restart_recovery ?? DEFAULT_POLICY;
  });
}
