// Where a new session runs, in the tray hung under the box
// (agent-orchestrator's project picker there): the machine, then the
// folder, a field that suggests the folders sessions already ran in on
// that machine and takes anything typed (data/folders.ts).

import type { FolderSuggestion } from "../../data/folders";
import type { Choice } from "../../ui/kit";
import { Combobox, Select } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";

export function WhereTray({
  machines,
  machine,
  onMachine,
  folders,
  folder,
  onFolder,
}: {
  machines: readonly Choice<NodeKey>[];
  machine: NodeKey | null;
  onMachine: (machine: NodeKey) => void;
  folders: readonly FolderSuggestion[];
  folder: string;
  onFolder: (folder: string) => void;
}) {
  return (
    <>
      <span className="where-machine">
        <Select
          aria-label="Machine"
          look="ghost"
          size="sm"
          width="fit"
          value={machine}
          options={machines.map((option) => ({ ...option, icon: "machine" }))}
          placeholder="No machine connected"
          onChange={onMachine}
          side="bottom"
        />
      </span>
      <span className="where-folder">
        <Combobox
          look="ghost"
          size="sm"
          mono
          icon="folder"
          aria-label="Folder"
          value={folder}
          onChange={onFolder}
          suggestions={folders}
          placeholder="Home folder"
        />
      </span>
    </>
  );
}
