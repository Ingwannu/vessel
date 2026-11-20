// Settings › Plugins' list (ADR 0115): Paseo's catalog card and its
// `PluginRow` (plugins-page.tsx): the id with its status badge (Paseo's
// StatusBadge, drawn here: a tinted pill without a dot, its own rule in
// plugins-section.css; the kit's Badge keeps its look elsewhere), the
// description, source and error under it, the enable switch, and a menu
// of Logs, Reload (only while enabled) and Remove; or the list's loading
// (an sm Alert), error (an sm error Alert with an outline Retry) and empty
// state (the card with its words centred), as Paseo's `catalogContent`.

import type { Outcome } from "../../data/outcome";
import {
  PLUGIN_WORDS,
  type PluginAction,
  type PluginPending,
  pluginSourceLine,
  problemWords,
  statusLook,
} from "../../data/plugins";
import type { PluginListItem } from "../../wire/proto";
import {
  Alert,
  Button,
  IconButton,
  Menu,
  MenuItem,
  Row,
  RowList,
  Switch,
} from "../../ui/kit";

export interface PluginRowHandlers {
  onAction: (action: PluginAction, plugin: PluginListItem) => void;
  onLogs: (plugin: PluginListItem) => void;
  onRemove: (plugin: PluginListItem) => void;
}

/** The installations, or why there are none to show. */
export function PluginList({
  listed,
  pending,
  onRetry,
  handlers,
}: {
  listed: Outcome<readonly PluginListItem[]> | null;
  pending: PluginPending | null;
  onRetry: () => void;
  handlers: PluginRowHandlers;
}) {
  if (listed === null) {
    return <Alert size="sm" title={PLUGIN_WORDS.loading} />;
  }
  if (!listed.ok) {
    return (
      <Alert
        size="sm"
        variant="error"
        title={PLUGIN_WORDS.errorTitle}
        description={problemWords(listed)}
      >
        <Button size="sm" variant="outline" onClick={onRetry}>
          {PLUGIN_WORDS.retry}
        </Button>
      </Alert>
    );
  }
  if (listed.value.length === 0) {
    return (
      <RowList label={PLUGIN_WORDS.title}>
        <li className="plugin-empty">{PLUGIN_WORDS.empty}</li>
      </RowList>
    );
  }
  return (
    <RowList label={PLUGIN_WORDS.title}>
      {listed.value.map((plugin) => (
        <PluginRow
          key={plugin.id}
          plugin={plugin}
          pending={pending}
          handlers={handlers}
        />
      ))}
    </RowList>
  );
}

/** What Paseo shows under a row's id. */
function PluginHint({ plugin }: { plugin: PluginListItem }) {
  const sourceLine = pluginSourceLine(plugin);
  return (
    <>
      {plugin.description === undefined ? null : (
        <div>{plugin.description}</div>
      )}
      {sourceLine === null ? null : (
        <div className="plugin-source">{sourceLine}</div>
      )}
      {plugin.error === undefined ? null : (
        <div className="plugin-error">{plugin.error}</div>
      )}
    </>
  );
}

/** One installation (Paseo's PluginRow). */
function PluginRow({
  plugin,
  pending,
  handlers,
}: {
  plugin: PluginListItem;
  pending: PluginPending | null;
  handlers: PluginRowHandlers;
}) {
  const look = statusLook(plugin.status);
  const busy = pending !== null;
  const mine =
    pending !== null && "plugin" in pending && pending.plugin === plugin.id
      ? pending.action
      : null;
  const hinted =
    plugin.description !== undefined ||
    plugin.path !== undefined ||
    plugin.error !== undefined;
  return (
    <Row
      title={
        <span className="plugin-title">
          {plugin.id}
          <span className={`plugin-status ${look.tone}`}>{look.word}</span>
        </span>
      }
      {...(hinted
        ? { detail: { description: <PluginHint plugin={plugin} /> } }
        : {})}
      control={
        <div
          className="plugin-controls"
          aria-label={`${plugin.id} ${look.word}`}
        >
          <Switch
            checked={plugin.enabled}
            onChange={(on) => {
              handlers.onAction(on ? "enable" : "disable", plugin);
            }}
            disabled={busy}
            label={`${plugin.id}: ${
              plugin.enabled ? PLUGIN_WORDS.disable : PLUGIN_WORDS.enable
            }`}
          />
          <Menu
            align="end"
            trigger={
              <IconButton
                label={`Actions for ${plugin.id}`}
                icon="more"
                variant="ghost"
                size="sm"
                disabled={busy}
              />
            }
          >
            <MenuItem
              onSelect={() => {
                handlers.onLogs(plugin);
              }}
            >
              {PLUGIN_WORDS.logsAction}
            </MenuItem>
            <MenuItem
              onSelect={() => {
                handlers.onAction("reload", plugin);
              }}
              disabled={busy || !plugin.enabled}
            >
              {mine === "reload" ? PLUGIN_WORDS.reloading : PLUGIN_WORDS.reload}
            </MenuItem>
            <MenuItem
              icon="trash"
              tone="danger"
              onSelect={() => {
                handlers.onRemove(plugin);
              }}
              disabled={busy}
            >
              {mine === "remove" ? PLUGIN_WORDS.removing : PLUGIN_WORDS.remove}
            </MenuItem>
          </Menu>
        </div>
      }
    />
  );
}
