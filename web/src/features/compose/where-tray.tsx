// Where a new session runs, in the tray hung under the box
// (agent-orchestrator's project pill there, GOAT's workspace selector):
// the machine; the project and how the session uses it (where-place.tsx);
// or, with no project, the folder, a field that suggests the folders
// sessions already ran in on that machine and takes anything typed
// (data/folders.ts).

import "./where-tray.css";

import { useMemo } from "react";

import { useFleetState } from "../../context";
import { folderSuggestions } from "../../data/folders";
import type { Choice } from "../../ui/kit";
import { Combobox, Select } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { Starter } from "./use-starter";
import { WherePlace } from "./where-place";

export function WhereTray({
  machines,
  machine,
  onMachine,
  folder,
  onFolder,
  starter,
}: {
  machines: readonly Choice<NodeKey>[];
  machine: NodeKey | null;
  onMachine: (machine: NodeKey) => void;
  folder: string;
  onFolder: (folder: string) => void;
  starter: Starter;
}) {
  const sessions = useFleetState((state) =>
    machine === null ? undefined : state.sessions[machine],
  );
  const folders = useMemo(() => folderSuggestions(sessions ?? []), [sessions]);
  return (
    <span className="where">
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
      <WherePlace machine={machine} starter={starter} />
      {starter.place.kind === "path" ? (
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
      ) : null}
    </span>
  );
}
