import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { readVesselTools, setVesselTools } from "../../data/vessel-tools";
import { Alert, Row, RowList, Switch } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import { SettingsSection } from "../settings/settings-page";

/** The selected machine owns the setting. The control reflects its receipt,
 * so a refused write leaves the current position visible. */
export function VesselToolsCard({ machine }: { machine: NodeKey }) {
  const fleet = useFleet();
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void readVesselTools(fleet, machine).then((result) => {
      if (current && result.ok) {
        setEnabled(result.value);
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
        setEnabled(result.value);
      } else {
        setProblem(result.problem.message);
      }
    });
  };

  if (enabled === null) {
    return null;
  }

  return (
    <SettingsSection title="Orchestration">
      <RowList label="Orchestration">
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
