// One machine's plugins as Settings › Plugins holds them (ADR 0115): the
// list, the switch, the change in flight and what the last change said,
// with Paseo's refresh on catalog changes.
//
// [Decision Log]
// - Purpose: keep the section's component to drawing; the calls and their
//   order live here, the decisions in data/plugins.ts and
//   data/plugins-follow.ts.
// - Paseo lists again on `plugin_catalog_changed` and subscribes again when
//   its host connects. The page follows `plugin.watch` while the section is
//   open and the page's link is up (`followPlugins`): a failed poll waits
//   and starts over, and a reopened link restarts it with a fresh read
//   (node review R1). The machine's status does not change on a socket
//   drop, so the link is what the effect depends on.
// - A change keeps the controls waiting until the list is read again
//   (`settleChange`, R3), and only the newest list read is kept (note 5).
// - A switch that cannot be read says why, once, under the field (note 9).

import { useCallback, useEffect, useRef, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import type { Outcome } from "../../data/outcome";
import {
  changedWords,
  installPlugin,
  installSource,
  listPlugins,
  PLUGIN_WORDS,
  type PluginAction,
  pluginAction,
  type PluginFeedback,
  type PluginPending,
  problemWords,
  readSwitch,
  setSwitch,
  watchPlugins,
} from "../../data/plugins";
import { followPlugins, settleChange } from "../../data/plugins-follow";
import type { NodeKey } from "../../wire/bridge";
import type { PluginListItem } from "../../wire/proto";

export interface PluginsHold {
  listed: Outcome<readonly PluginListItem[]> | null;
  /** The switch in force; `null` until read. */
  enabled: boolean | null;
  pending: PluginPending | null;
  feedback: PluginFeedback | null;
  say: (feedback: PluginFeedback | null) => void;
  load: () => Promise<void>;
  act: (action: PluginAction, plugin: PluginListItem) => void;
  /** Install `source`; `installed` runs once it is in (the field clears). */
  install: (source: string, installed: () => void) => void;
  toggle: (on: boolean) => void;
}

export function usePlugins(machine: NodeKey, supported: boolean): PluginsHold {
  const fleet = useFleet();
  const [listed, setListed] = useState<Outcome<
    readonly PluginListItem[]
  > | null>(null);
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [pending, setPending] = useState<PluginPending | null>(null);
  const [feedback, setFeedback] = useState<PluginFeedback | null>(null);

  const open = useFleetState((state) => state.link.kind === "open");
  // Every list read is numbered and only the newest answer is kept.
  const asked = useRef(0);

  const load = useCallback(async () => {
    asked.current += 1;
    const mine = asked.current;
    const [list, on] = await Promise.all([
      listPlugins(fleet, machine),
      readSwitch(fleet, machine),
    ]);
    if (mine !== asked.current) return;
    setListed(list);
    if (on.ok) {
      setEnabled(on.value);
    } else {
      setFeedback(
        (said) =>
          said ?? {
            kind: "error",
            message: `${PLUGIN_WORDS.switchUnread}: ${problemWords(on)}`,
          },
      );
    }
  }, [fleet, machine]);

  useEffect(() => {
    if (!supported || !open) return;
    const ended = new AbortController();
    void followPlugins(
      {
        reload: load,
        poll: (after) => watchPlugins(fleet, machine, after),
      },
      ended.signal,
    );
    return () => {
      ended.abort();
    };
  }, [fleet, machine, supported, open, load]);

  const finish = useCallback(
    (outcome: Outcome<unknown>, success: string) =>
      settleChange({
        say: () => {
          setFeedback(
            outcome.ok
              ? { kind: "success", message: success }
              : { kind: "error", message: problemWords(outcome) },
          );
        },
        reload: load,
        release: () => {
          setPending(null);
        },
      }),
    [load],
  );

  const act = useCallback(
    (action: PluginAction, plugin: PluginListItem) => {
      setFeedback(null);
      setPending({ action, plugin: plugin.id });
      void pluginAction(fleet, machine, action, plugin.id).then((outcome) => {
        void finish(outcome, changedWords(action, plugin.id));
      });
    },
    [fleet, machine, finish],
  );

  const install = useCallback(
    (text: string, installed: () => void) => {
      const source = installSource(text);
      if (source === null) return;
      setFeedback(null);
      setPending({ action: "install" });
      void installPlugin(fleet, machine, source).then((outcome) => {
        if (outcome.ok) installed();
        void finish(
          outcome,
          outcome.ok ? changedWords("install", outcome.value.id) : "",
        );
      });
    },
    [fleet, machine, finish],
  );

  const toggle = useCallback(
    (on: boolean) => {
      setFeedback(null);
      setPending({ action: "switch" });
      void setSwitch(fleet, machine, on).then((outcome) => {
        if (outcome.ok) setEnabled(outcome.value);
        void finish(
          outcome,
          on ? PLUGIN_WORDS.globalEnabled : PLUGIN_WORDS.globalDisabled,
        );
      });
    },
    [fleet, machine, finish],
  );

  return {
    listed,
    enabled,
    pending,
    feedback,
    say: setFeedback,
    load,
    act,
    install,
    toggle,
  };
}
