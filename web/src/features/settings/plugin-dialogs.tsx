// Settings › Plugins' dialogs (ADR 0115): Paseo's PluginLogsSheet (the
// retained tail with Refresh, its loading, error and empty states, a
// stderr head in the danger colour) and its remove confirmation
// ("Remove {id}?", the source directory is not deleted).

import {
  type ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";

import { useFleet } from "../../context";
import type { Outcome } from "../../data/outcome";
import {
  logHead,
  PLUGIN_WORDS,
  pluginLogs,
  problemWords,
} from "../../data/plugins";
import type { NodeKey } from "../../wire/bridge";
import type { PluginListItem, PluginLogs } from "../../wire/proto";
import { Alert, Button, Dialog } from "../../ui/kit";

/** Paseo's PluginLogsSheet, as the kit's dialog (a sheet when compact). */
export function PluginLogsDialog({
  machine,
  plugin,
  onClose,
}: {
  machine: NodeKey;
  plugin: PluginListItem;
  onClose: () => void;
}) {
  const fleet = useFleet();
  const [logs, setLogs] = useState<Outcome<PluginLogs> | null>(null);
  const [fetching, setFetching] = useState(false);
  // Every read is numbered and only the newest answer is kept, so a slow
  // first read cannot replace a Refresh's newer tail (node review note 8).
  const asked = useRef(0);
  const read = useCallback(() => {
    asked.current += 1;
    const mine = asked.current;
    return pluginLogs(fleet, machine, plugin.id).then((outcome) => {
      if (mine === asked.current) setLogs(outcome);
    });
  }, [fleet, machine, plugin.id]);
  const refresh = useCallback(() => {
    setFetching(true);
    void read().then(() => {
      setFetching(false);
    });
  }, [read]);
  // The first read when the dialog opens; Refresh reads again. The state is
  // set only in the answer's callback, never during the effect itself; a
  // closed dialog's answer is dropped.
  useEffect(() => {
    void read();
    return () => {
      asked.current += 1;
    };
  }, [read]);
  let content: ReactNode;
  if (logs === null) {
    content = <p className="plugin-logs-state">{PLUGIN_WORDS.logsLoading}</p>;
  } else if (!logs.ok) {
    content = (
      <Alert
        size="sm"
        variant="error"
        title={PLUGIN_WORDS.logsErrorTitle}
        description={problemWords(logs)}
      >
        <Button size="sm" variant="outline" onClick={refresh}>
          {PLUGIN_WORDS.logsRefresh}
        </Button>
      </Alert>
    );
  } else if (logs.value.entries.length === 0) {
    content = <p className="plugin-logs-state">{PLUGIN_WORDS.logsEmpty}</p>;
  } else {
    content = (
      <div className="plugin-logs">
        {logs.value.entries.map((entry) => (
          <div key={entry.sequence} className="plugin-log">
            <span
              className={
                entry.stream === "stderr"
                  ? "plugin-log-head stderr"
                  : "plugin-log-head"
              }
            >
              {logHead(entry.at, entry.stream)}
            </span>
            <p className="plugin-log-message">{entry.message}</p>
          </div>
        ))}
      </div>
    );
  }
  return (
    <Dialog
      title={`Logs: ${plugin.id}`}
      onClose={onClose}
      size="lg"
      actions={
        <Button
          size="sm"
          variant="outline"
          onClick={refresh}
          disabled={fetching || logs === null}
        >
          {fetching ? PLUGIN_WORDS.logsRefreshing : PLUGIN_WORDS.logsRefresh}
        </Button>
      }
    >
      {content}
    </Dialog>
  );
}

/** Paseo's remove confirmation; a dismissal keeps the plugin. */
export function PluginRemoveDialog({
  plugin,
  onKeep,
  onRemove,
}: {
  plugin: PluginListItem;
  onKeep: () => void;
  onRemove: () => void;
}) {
  return (
    <Dialog
      title={`Remove ${plugin.id}?`}
      onClose={onKeep}
      footer={
        <>
          <Button variant="ghost" onClick={onKeep}>
            Cancel
          </Button>
          <Button variant="danger-solid" onClick={onRemove}>
            {PLUGIN_WORDS.remove}
          </Button>
        </>
      }
    >
      <p className="dialog-copy">{PLUGIN_WORDS.removeConfirmMessage}</p>
    </Dialog>
  );
}
