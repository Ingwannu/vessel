// Which machine a Settings section is about (Agents, Models, Devices,
// Sync): a quiet select at the right of the section's heading. Choosing
// one changes the address's `machine`, so the choice survives a reload
// and moving to another section.

import { useNavigate } from "@tanstack/react-router";

import { useFleetState } from "../../context";
import { machineChoices } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import type { SettingsSection } from "../../data/settings";
import { Select } from "../../ui/kit";

export function MachinePicker({
  section,
  machine,
}: {
  section: SettingsSection;
  machine: string;
}) {
  const navigate = useNavigate();
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const options = machineChoices(book ?? [], statuses, (node) =>
    nodeLabel(node, statuses[node.key]),
  );
  return (
    <Select
      aria-label="Machine"
      look="ghost"
      size="sm"
      width="fit"
      align="end"
      value={machine}
      options={options}
      onChange={(next) => {
        void navigate({
          to: "/settings/$section",
          params: { section },
          search: { machine: next },
        });
      }}
    />
  );
}
