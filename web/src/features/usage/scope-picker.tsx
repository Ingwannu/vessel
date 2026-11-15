// Existing Settings/Select rows: no account selector or vendor lookup.
// The node's historical names are distinct from currently configured logins.
import { useFleetState } from "../../context";
import {
  usageChoices,
  usageScopeKey,
  type RollingUsageScope,
  type UsageNames,
} from "../../data/usage/scope";
import { Row, RowList, Select } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import { FactValue } from "../settings/settings-page";

export function UsageScopePicker({
  node,
  scope,
  names,
  onChange,
}: {
  node: NodeKey;
  scope: RollingUsageScope;
  names: UsageNames | null;
  onChange: (scope: RollingUsageScope) => void;
}) {
  const agents = useFleetState((state) => state.agents[node]);
  const choices = usageChoices(names?.groups ?? [], scope, agents);
  return (
    <RowList label="Usage scope">
      <Row
        title="Agent and profile"
        detail={{
          description: (
            <>
              Names come from the last All agents report on this machine. All
              profiles includes reports without a recorded profile name.
              {names?.incomplete === true
                ? " Some names may be missing from this partial report."
                : null}
            </>
          ),
        }}
        control={
          choices.length === 1 ? (
            <FactValue>All agents</FactValue>
          ) : (
            <div className="usage-scope">
              <Select
                aria-label="Usage agent or profile"
                size="sm"
                width="fit"
                value={usageScopeKey(scope)}
                options={choices}
                onChange={(value) => {
                  const chosen = choices.find(
                    (choice) => choice.value === value,
                  );
                  if (chosen !== undefined) onChange(chosen.scope);
                }}
              />
            </div>
          )
        }
      />
    </RowList>
  );
}
