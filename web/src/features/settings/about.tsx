// Settings › About: which Vessel this page is, which each machine runs
// and the protocol it speaks with this page, and the software the page is
// built from with each licence (Paseo's About lists the connected hosts;
// the licences are what the bundle carries, `data/about.ts`).

import { useFleetState } from "../../context";
import { BUNDLED, VESSEL_LICENCE } from "../../data/about";
import { machineState } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import { Badge, Row, RowList } from "../../ui/kit";
import { FactValue, SettingsPage, SettingsSection } from "./settings-page";

export function AboutSection() {
  const link = useFleetState((state) => state.link);
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  return (
    <SettingsPage lead="The versions in use, and the software this page is built from.">
      <SettingsSection title="Vessel">
        <RowList label="Vessel">
          <Row
            title="This page"
            detail={{ description: "Served by vessel ui on this machine." }}
            control={
              <FactValue mono>
                {link.kind === "open" ? link.version : "not connected"}
              </FactValue>
            }
          />
          <Row
            title="Licence"
            detail={{ description: "Source-available; see LICENSE.md." }}
            control={<FactValue>{VESSEL_LICENCE}</FactValue>}
          />
        </RowList>
      </SettingsSection>
      <SettingsSection title="Machines">
        <RowList label="Machine versions">
          {(book ?? []).map((node) => {
            const status = statuses[node.key];
            return (
              <Row
                key={node.key}
                mark={{ icon: "machine" }}
                title={nodeLabel(node, status)}
                detail={{
                  facts:
                    status?.state === "connected"
                      ? [
                          `Speaks protocol ${String(status.protocol)} with this page`,
                          `supports ${String(status.node.protocol.min)} to ${String(status.node.protocol.max)}`,
                        ]
                      : [machineState(status).word],
                }}
                control={
                  status?.state === "connected" ? (
                    <FactValue mono>{status.node.version}</FactValue>
                  ) : undefined
                }
              />
            );
          })}
        </RowList>
      </SettingsSection>
      <SettingsSection title="Software in this page">
        <RowList label="Licences">
          {BUNDLED.map((entry) => (
            <Row
              key={entry.name}
              title={entry.name}
              detail={{ description: entry.role }}
              control={
                <>
                  {entry.text === undefined ? null : (
                    <a
                      className="fact-link"
                      href={entry.text}
                      target="_blank"
                      rel="noreferrer"
                    >
                      Licence text
                    </a>
                  )}
                  <Badge look="mono">{entry.licence}</Badge>
                </>
              }
            />
          ))}
        </RowList>
      </SettingsSection>
    </SettingsPage>
  );
}
