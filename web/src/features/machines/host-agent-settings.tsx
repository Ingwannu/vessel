import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import {
  readHostAgentSettings,
  setVesselTools,
} from "../../data/host-agent-settings";
import { Alert, Row, RowList, Switch } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { NodeSettings } from "../../wire/proto";
import { SettingsSection } from "../settings/settings-page";
import { AppendSystemPromptRow } from "./append-system-prompt";

/** The selected machine owns the setting. The control reflects its receipt,
 * so a refused write leaves the current position visible. */
export function HostAgentSettings({ machine }: { machine: NodeKey }) {
  const fleet = useFleet();
  const [settings, setSettings] = useState<NodeSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void readHostAgentSettings(fleet, machine).then((result) => {
      if (current && result.ok) {
        setSettings(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [fleet, machine]);

  const save = (next: boolean) => {
    setSaving(true);
    setProblem(null);
    void setVesselTools(fleet, machine, next).then((result) => {
      setSaving(false);
      if (result.ok) {
        setSettings((current) =>
          current === null ? null : { ...current, vessel_tools: result.value },
        );
      } else {
        setProblem(result.problem.message);
      }
    });
  };

  const enabled = settings?.vessel_tools ?? null;
  const hasPrompt =
    settings?.append_system_prompt_transports?.includes("codex_app_server") ===
    true;
  if (enabled === null && !hasPrompt) {
    return null;
  }

  return (
    <SettingsSection title="Agents">
      <RowList label="Agent settings">
        {enabled === null ? null : (
          <Row
            title="Enable Vessel tools"
            detail={{
              description:
                "Agents will be able to manage worktrees, agents and schedules",
            }}
            control={
              <Switch
                label="Inject Vessel tools"
                checked={enabled}
                disabled={saving}
                onChange={save}
              />
            }
          />
        )}
        {hasPrompt ? <AppendSystemPromptRow machine={machine} /> : null}
      </RowList>
      {problem === null ? null : (
        <Alert
          variant="error"
          size="sm"
          title="Unable to update Vessel tools"
          description={problem}
        />
      )}
    </SettingsSection>
  );
}
