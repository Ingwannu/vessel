// Settings › Sync: what one machine shares with the others (ADR 0012):
// its instructions, skills and tool servers, the version the others
// pull, who has pulled it, and the edits they kept. The old Sync page, on
// rows and in the page's words.

import { useEffect, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { ago } from "../../data/format";
import { machineFor, machineState } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import type { Outcome } from "../../data/outcome";
import {
  loadPublished,
  memberLine,
  memberState,
  type Published,
  secretNames,
  withConflicts,
} from "../../data/sync";
import type { NodeKey } from "../../wire/bridge";
import { Badge, Banner, Row, RowList, Skeleton } from "../../ui/kit";
import { MachinePicker } from "../machines/machine-picker";
import { FactValue, SettingsPage, SettingsSection } from "./settings-page";

const LEAD =
  "What one machine shares with the others: instructions, skills and tool servers. The others pull it; an edit made on one of them is kept.";

/** Files listed before "and N more". */
const FILES_SHOWN = 12;

export function SyncSection({ machine }: { machine: NodeKey | undefined }) {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const selected = useFleetState((state) => state.selected);
  const node = machineFor(book ?? [], machine, selected);
  if (node === null) {
    return (
      <SettingsPage lead={LEAD}>
        {book === null ? (
          <Skeleton />
        ) : (
          <p className="settings-empty">No machines yet.</p>
        )}
      </SettingsPage>
    );
  }
  return (
    <SharedBy
      key={node.key}
      machine={node.key}
      label={nodeLabel(node, statuses[node.key])}
    />
  );
}

function SharedBy({ machine, label }: { machine: NodeKey; label: string }) {
  const fleet = useFleet();
  const status = useFleetState((state) => state.statuses[machine]);
  const connected = status?.state === "connected";
  const [published, setPublished] = useState<Outcome<Published> | null>(null);
  useEffect(() => {
    if (!connected) {
      return undefined;
    }
    let current = true;
    void loadPublished(fleet, machine).then((outcome) => {
      if (current) {
        setPublished(outcome);
      }
    });
    return () => {
      current = false;
    };
  }, [fleet, machine, connected]);
  const picker = <MachinePicker section="sync" machine={machine} />;
  const title = `Shared from ${label}`;
  const problem =
    published !== null && !published.ok ? published.problem : null;
  if (!connected || published?.ok !== true) {
    return (
      <SettingsPage lead={LEAD}>
        <SettingsSection title={title} action={picker}>
          {!connected ? (
            <p className="settings-empty">
              {label} is {machineState(status).word.toLowerCase()}; what it
              shares shows once it is connected.
            </p>
          ) : problem === null ? (
            <Skeleton />
          ) : (
            <p className="settings-empty">
              {label} shares nothing ({problem.message}). To make it the machine
              the others pull from, run <code>vessel fleet init</code> there; or
              pick the machine that does.
            </p>
          )}
        </SettingsSection>
      </SettingsPage>
    );
  }
  const { manifest, members, secrets } = published.value;
  const files = Object.entries(manifest.files);
  const servers = Object.entries(manifest.mcp);
  const named = secretNames(manifest);
  const kept = withConflicts(members);
  return (
    <SettingsPage lead={LEAD}>
      <SettingsSection title={title} action={picker}>
        <RowList label={title}>
          <Row
            title="Version"
            detail={{
              description: "Goes up each time what it shares changes.",
            }}
            control={<FactValue mono>{manifest.generation}</FactValue>}
          />
          <Row
            title="Secrets"
            detail={{
              description:
                named.length === 0
                  ? "The tool servers name none."
                  : `Named by the tool servers: ${named.join(", ")}. Values stay on each machine.`,
            }}
            control={
              <FactValue>
                {secrets.length} on {label}
              </FactValue>
            }
          />
        </RowList>
      </SettingsSection>
      <SettingsSection title="Files">
        {files.length === 0 ? (
          <p className="settings-empty">No files are shared.</p>
        ) : (
          <RowList label="Shared files">
            {files.slice(0, FILES_SHOWN).map(([path, file]) => (
              <Row
                key={path}
                title={<span className="mono-title">{path}</span>}
                detail={{
                  facts: [
                    `${String(file.size)} bytes`,
                    file.executable === true ? "executable" : "",
                  ],
                }}
              />
            ))}
            {files.length > FILES_SHOWN ? (
              <Row title={`and ${String(files.length - FILES_SHOWN)} more`} />
            ) : null}
          </RowList>
        )}
      </SettingsSection>
      {servers.length === 0 ? null : (
        <SettingsSection title="Tool servers">
          <RowList label="Shared tool servers">
            {servers.map(([name, transport]) => (
              <Row
                key={name}
                title={name}
                detail={{
                  facts: [
                    transport.type === "stdio"
                      ? `runs ${transport.command}`
                      : `at ${transport.url}`,
                  ],
                }}
                control={<Badge look="mono">MCP</Badge>}
              />
            ))}
          </RowList>
        </SettingsSection>
      )}
      <SettingsSection title="Machines that pull it">
        {kept.map((member) => (
          <Banner key={member.device} tone="warn">
            {member.label} kept its own edit of{" "}
            {(member.conflicts ?? []).join(", ")}. The shared version is beside
            it, named <code>.vessel-incoming</code>; Vessel never overwrites a
            file you changed.
          </Banner>
        ))}
        {members.length === 0 ? (
          <p className="settings-empty">
            None yet. Another machine joins with <code>vessel fleet join</code>{" "}
            and a link from {label}.
          </p>
        ) : (
          <RowList label="Members">
            {members.map((member) => {
              const state = memberState(member, manifest.generation);
              return (
                <Row
                  key={member.device}
                  mark={{ icon: "machine" }}
                  title={member.label}
                  detail={{ facts: memberLine(member, ago) }}
                  control={<Badge tone={state.tone}>{state.word}</Badge>}
                />
              );
            })}
          </RowList>
        )}
      </SettingsSection>
    </SettingsPage>
  );
}
