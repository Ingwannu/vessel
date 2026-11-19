// Settings › Plugins (ADR 0115): a machine's plugins, as Paseo's host
// Plugins page (`packages/app/src/screens/settings/plugins-page.tsx`):
// the trust notice, the machine's plugin switch, the install field and
// its feedback, the installations (plugin-rows.tsx), the logs and remove
// dialogs (plugin-dialogs.tsx), and the offline and too-old states.
//
// [Decision Log]
// - Purpose: make Paseo's plugin management reachable on the page; the
//   node's `plugin.*` calls do the work (data/plugins.ts, use-plugins.ts).
// - Reuse: Settings › Devices' shape (a machine picker in the heading, a
//   component keyed by the machine so nothing of one machine shows under
//   another's name) and the kit's rows, switch, menu, dialog and banner.
// - Paseo's feedback is an inline alert under the install field: the page
//   draws it there too, a note on success and an error banner on failure,
//   not a toast.
// - Stated differences: no docs link (Vessel has no published plugin
//   reference yet); the field asks for a directory, the one source the
//   node installs (ADR 0115); the trust notice does not say plugins run in
//   the app (ADR 0079: the app draws their data with its own controls).

import "./plugins-section.css";

import { useState } from "react";

import { useFleetState } from "../../context";
import { machineFor } from "../../data/machines";
import {
  installSource,
  PLUGIN_WORDS,
  pluginPageState,
  speaksPlugins,
} from "../../data/plugins";
import type { NodeKey } from "../../wire/bridge";
import type { PluginListItem } from "../../wire/proto";
import {
  Banner,
  Button,
  Field,
  Row,
  RowList,
  Skeleton,
  Switch,
  TextField,
} from "../../ui/kit";
import { MachinePicker } from "../machines/machine-picker";
import { PluginLogsDialog, PluginRemoveDialog } from "./plugin-dialogs";
import { PluginList } from "./plugin-rows";
import { SettingsPage, SettingsSection } from "./settings-page";
import { usePlugins } from "./use-plugins";

const LEAD =
  "Code that extends a machine: what is installed, whether it runs, and its logs.";

export function PluginsSection({ machine }: { machine: NodeKey | undefined }) {
  const book = useFleetState((state) => state.book);
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
  return <PluginsOf key={node.key} machine={node.key} />;
}

/** One machine's plugins; keyed by the machine. */
function PluginsOf({ machine }: { machine: NodeKey }) {
  const status = useFleetState((state) => state.statuses[machine]);
  const connected = status?.state === "connected";
  const supported =
    status?.state === "connected" && speaksPlugins(status.protocol);
  const plugins = usePlugins(machine, supported);
  const [source, setSource] = useState("");
  const [logsOf, setLogsOf] = useState<PluginListItem | null>(null);
  const [removing, setRemoving] = useState<PluginListItem | null>(null);
  const picker = <MachinePicker section="plugins" machine={machine} />;
  const state = pluginPageState({
    connected,
    supported,
    loading: plugins.listed === null,
    error: plugins.listed?.ok === false,
    pluginCount: plugins.listed?.ok === true ? plugins.listed.value.length : 0,
  });
  if (state === "offline" || state === "unsupported") {
    return (
      <SettingsPage lead={LEAD}>
        <SettingsSection title={PLUGIN_WORDS.title} action={picker}>
          <Banner>
            {state === "offline" ? (
              <>
                <strong>{PLUGIN_WORDS.offlineTitle}</strong>{" "}
                {PLUGIN_WORDS.offlineDescription}
              </>
            ) : (
              <strong>{PLUGIN_WORDS.updateTitle}</strong>
            )}
          </Banner>
        </SettingsSection>
      </SettingsPage>
    );
  }
  const busy = plugins.pending !== null;
  const ready = installSource(source) !== null;
  const install = () => {
    plugins.install(source, () => {
      setSource("");
    });
  };
  const keep = (plugin: PluginListItem) => {
    plugins.say({ kind: "success", message: `Kept ${plugin.id}` });
    setRemoving(null);
  };
  return (
    <SettingsPage lead={LEAD}>
      <SettingsSection title={PLUGIN_WORDS.title} action={picker}>
        <Banner>
          <strong>{PLUGIN_WORDS.trustedTitle}</strong>{" "}
          {PLUGIN_WORDS.trustedDescription}
        </Banner>
        <RowList label={PLUGIN_WORDS.globalTitle}>
          <Row
            title={PLUGIN_WORDS.globalTitle}
            detail={{ description: PLUGIN_WORDS.globalHint }}
            control={
              <Switch
                checked={plugins.enabled === true}
                onChange={plugins.toggle}
                disabled={busy || plugins.enabled === null}
                label={PLUGIN_WORDS.globalTitle}
              />
            }
          />
        </RowList>
        <div className="plugin-install">
          <Field label={PLUGIN_WORDS.sourceLabel}>
            <TextField
              value={source}
              onChange={setSource}
              placeholder={PLUGIN_WORDS.sourcePlaceholder}
              mono
              spellCheck={false}
              disabled={busy}
              onKeyDown={(event) => {
                if (event.key === "Enter" && ready && !busy) install();
              }}
            />
          </Field>
          <Button onClick={install} disabled={!ready || busy}>
            {plugins.pending?.action === "install"
              ? PLUGIN_WORDS.installing
              : PLUGIN_WORDS.install}
          </Button>
        </div>
        {plugins.feedback === null ? null : plugins.feedback.kind ===
          "error" ? (
          <Banner tone="error">{plugins.feedback.message}</Banner>
        ) : (
          <p className="section-note" role="status">
            {plugins.feedback.message}
          </p>
        )}
        <PluginList
          listed={plugins.listed}
          pending={plugins.pending}
          onRetry={plugins.load}
          handlers={{
            onAction: plugins.act,
            onLogs: setLogsOf,
            onRemove: setRemoving,
          }}
        />
      </SettingsSection>
      {logsOf === null ? null : (
        <PluginLogsDialog
          machine={machine}
          plugin={logsOf}
          onClose={() => {
            setLogsOf(null);
          }}
        />
      )}
      {removing === null ? null : (
        <PluginRemoveDialog
          plugin={removing}
          onKeep={() => {
            keep(removing);
          }}
          onRemove={() => {
            const plugin = removing;
            setRemoving(null);
            plugins.act("remove", plugin);
          }}
        />
      )}
    </SettingsPage>
  );
}
