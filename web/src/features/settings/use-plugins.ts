// One machine's plugins as Settings › Plugins holds them (ADR 0115): the
// list, the switch, the change in flight and what the last change said,
// with Paseo's refresh on catalog changes.
//
// [Decision Log]
// - Purpose: keep the section's component to drawing; the calls and their
//   order live here, the decisions in data/plugins.ts.
// - Paseo lists again on `plugin_catalog_changed`; the page long-polls
//   `plugin.watch` while the section is open. The first poll asks with a
//   generation the node never has, so it answers its current one at once.
//   A failed poll ends the loop: the machine's status changes then, and
//   the effect runs again when it is back.

import { useCallback, useEffect, useState } from "react";

import { useFleet } from "../../context";
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
import type { NodeKey } from "../../wire/bridge";
import type { PluginListItem } from "../../wire/proto";

export interface PluginsHold {
  listed: Outcome<readonly PluginListItem[]> | null;
  /** The switch in force; `null` until read. */
  enabled: boolean | null;
  pending: PluginPending | null;
  feedback: PluginFeedback | null;
  say: (feedback: PluginFeedback | null) => void;
  load: () => void;
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

  const load = useCallback(() => {
    void listPlugins(fleet, machine).then(setListed);
    void readSwitch(fleet, machine).then((outcome) => {
      if (outcome.ok) setEnabled(outcome.value);
    });
  }, [fleet, machine]);

  useEffect(() => {
    if (supported) load();
  }, [supported, load]);

  useEffect(() => {
    if (!supported) return;
    const ended = new AbortController();
    void (async () => {
      const first = await watchPlugins(fleet, machine, Number.MAX_SAFE_INTEGER);
      if (!first.ok) return;
      let seen = first.value;
      while (!ended.signal.aborted) {
        const next = await watchPlugins(fleet, machine, seen);
        if (ended.signal.aborted || !next.ok) return;
        if (next.value !== seen) {
          seen = next.value;
          load();
        }
      }
    })();
    return () => {
      ended.abort();
    };
  }, [fleet, machine, supported, load]);

  const finish = useCallback(
    (outcome: Outcome<unknown>, success: string) => {
      setPending(null);
      setFeedback(
        outcome.ok
          ? { kind: "success", message: success }
          : { kind: "error", message: problemWords(outcome) },
      );
      load();
    },
    [load],
  );

  const act = useCallback(
    (action: PluginAction, plugin: PluginListItem) => {
      setFeedback(null);
      setPending({ action, plugin: plugin.id });
      void pluginAction(fleet, machine, action, plugin.id).then((outcome) => {
        finish(outcome, changedWords(action, plugin.id));
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
        finish(
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
        finish(
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
