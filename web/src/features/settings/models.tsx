// Settings › Models: a machine's model routes and profiles, read-only, in
// rows (`model.list`, `profile.list`). They are written in that machine's
// own files; the page says which.

import { useFleetState } from "../../context";
import { machineFor, machineState } from "../../data/machines";
import { profileFacts, routeFacts } from "../../data/model-rows";
import { nodeLabel } from "../../data/nodes";
import { Row, RowList, Skeleton } from "../../ui/kit";
import { MachinePicker } from "../machines/machine-picker";
import { SettingsPage, SettingsSection } from "./settings-page";

export function ModelsSection({ machine }: { machine: string | undefined }) {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const allRoutes = useFleetState((state) => state.models);
  const allProfiles = useFleetState((state) => state.profiles);
  const node = machineFor(book ?? [], statuses, machine);
  const lead =
    "Where each machine's agents get their models. Written in that machine's models.toml and profiles.toml.";
  if (node === null) {
    return (
      <SettingsPage lead={lead}>
        {book === null ? (
          <Skeleton />
        ) : (
          <p className="settings-empty">No machines yet.</p>
        )}
      </SettingsPage>
    );
  }
  const label = nodeLabel(node, statuses[node.key]);
  const connected = statuses[node.key]?.state === "connected";
  const routes = allRoutes[node.key];
  const profiles = allProfiles[node.key];
  const picker = <MachinePicker section="models" machine={node.key} />;
  if (!connected && routes === undefined) {
    return (
      <SettingsPage lead={lead}>
        <SettingsSection title={`Model routes on ${label}`} action={picker}>
          <p className="settings-empty">
            {label} is {machineState(statuses[node.key]).word.toLowerCase()};
            its models show once it is connected.
          </p>
        </SettingsSection>
      </SettingsPage>
    );
  }
  return (
    <SettingsPage lead={lead}>
      <SettingsSection title={`Model routes on ${label}`} action={picker}>
        {routes === undefined ? (
          <Skeleton />
        ) : routes === null ? (
          <p className="settings-empty">
            {label} runs a Vessel older than model routes.
          </p>
        ) : routes.length === 0 ? (
          <p className="settings-empty">
            No routes: its agents use their own providers.
          </p>
        ) : (
          <RowList label={`Model routes on ${label}`}>
            {routes.map((route) => (
              <Row
                key={route.name}
                title={route.name}
                detail={{ facts: routeFacts(route) }}
              />
            ))}
          </RowList>
        )}
      </SettingsSection>
      <SettingsSection title="Profiles">
        {profiles === undefined ? (
          <Skeleton />
        ) : profiles.length === 0 ? (
          <p className="settings-empty">
            No profiles: each agent runs with its own account.
          </p>
        ) : (
          <RowList label={`Profiles on ${label}`}>
            {profiles.map((profile) => (
              <Row
                key={profile.name}
                title={profile.label ?? profile.name}
                detail={{ facts: profileFacts(profile) }}
                control={
                  profile.label === undefined ||
                  profile.label === null ? undefined : (
                    <span className="fact-value mono">{profile.name}</span>
                  )
                }
              />
            ))}
          </RowList>
        )}
      </SettingsSection>
    </SettingsPage>
  );
}
