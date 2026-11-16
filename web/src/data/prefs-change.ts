// A preference change applied to the page's copy, as the bridge applies
// it to the file (`crates/vessel-web/src/wire/prefs/change.rs`): the
// page shows a choice at once, and the bridge's answer, with its
// revision, replaces the guess (`data/prefs.ts`).

import type {
  MachineDefaults,
  PreferenceChange,
  Preferences,
  ShortcutEdit,
} from "../wire/bridge";
import type { NodeId } from "../wire/proto";

/** What every setting is before anyone chose: the bridge's defaults. */
export const DEFAULT_PREFERENCES: Preferences = {
  appearance: {
    theme: "system",
    interface_size: 14,
    code_size: 12,
    content_width: "comfortable",
    conversation: "full",
  },
  layout: { sidebar: "shown", inspector: "shown" },
  composer: { send: "enter", delivery: "queue" },
  shortcuts: {},
  notifications: { alerts: "off", title: true },
  sidebar: { group: "machine", sort: "created", ended: true, labels: [] },
  defaults: {},
};

function shortcutsAfter(
  shortcuts: Preferences["shortcuts"],
  edits: readonly ShortcutEdit[],
): Preferences["shortcuts"] {
  const next: Record<string, string[]> = { ...shortcuts };
  for (const edit of edits) {
    switch (edit.to.kind) {
      case "bindings":
        next[edit.action] = [...edit.to.keys];
        break;
      case "default":
        // A copy without the key: the action has its defaults again.
        Reflect.deleteProperty(next, edit.action);
        break;
    }
  }
  return next;
}

function withMachine(
  defaults: Preferences["defaults"],
  node: NodeId,
  change: (machine: MachineDefaults) => MachineDefaults,
): Preferences["defaults"] {
  const before = defaults[node] ?? { agent: null, agents: {} };
  return { ...defaults, [node]: change(before) };
}

/** `prefs` with `change` applied. */
export function applyChange(
  prefs: Preferences,
  change: PreferenceChange,
): Preferences {
  const appearance = (part: Partial<Preferences["appearance"]>) => ({
    ...prefs,
    appearance: { ...prefs.appearance, ...part },
  });
  switch (change.set) {
    case "theme":
      return appearance({ theme: change.value });
    case "interface_size":
      return appearance({ interface_size: change.value });
    case "code_size":
      return appearance({ code_size: change.value });
    case "content_width":
      return appearance({ content_width: change.value });
    case "conversation":
      return appearance({ conversation: change.value });
    case "sidebar_panel":
      return { ...prefs, layout: { ...prefs.layout, sidebar: change.value } };
    case "inspector_panel":
      return { ...prefs, layout: { ...prefs.layout, inspector: change.value } };
    case "send":
      return { ...prefs, composer: { ...prefs.composer, send: change.value } };
    case "delivery":
      return {
        ...prefs,
        composer: { ...prefs.composer, delivery: change.value },
      };
    case "alerts":
      return {
        ...prefs,
        notifications: { ...prefs.notifications, alerts: change.value },
      };
    case "title_count":
      return {
        ...prefs,
        notifications: { ...prefs.notifications, title: change.value },
      };
    case "sidebar_group":
      return { ...prefs, sidebar: { ...prefs.sidebar, group: change.value } };
    case "sidebar_sort":
      return { ...prefs, sidebar: { ...prefs.sidebar, sort: change.value } };
    case "sidebar_ended":
      return { ...prefs, sidebar: { ...prefs.sidebar, ended: change.value } };
    case "sidebar_labels":
      return {
        ...prefs,
        sidebar: { ...prefs.sidebar, labels: change.value.picks },
      };
    case "shortcuts":
      return {
        ...prefs,
        shortcuts: shortcutsAfter(prefs.shortcuts, change.value.edits),
      };
    case "reset_shortcuts":
      return { ...prefs, shortcuts: {} };
    case "default_agent":
      return {
        ...prefs,
        defaults: withMachine(prefs.defaults, change.value.node, (machine) => ({
          ...machine,
          agent: change.value.agent,
        })),
      };
    case "agent_defaults": {
      const { node, agent, profile, choices } = change.value;
      return {
        ...prefs,
        defaults: withMachine(prefs.defaults, node, (machine) => ({
          ...machine,
          agents: { ...machine.agents, [agent]: { profile, choices } },
        })),
      };
    }
    case "forget_defaults": {
      const next: Record<string, MachineDefaults> = { ...prefs.defaults };
      Reflect.deleteProperty(next, change.value.node);
      return { ...prefs, defaults: next };
    }
  }
}

/** `section` as it was when it says the same as `next`, else `next`. */
function same<T>(section: T, next: T): T {
  return JSON.stringify(section) === JSON.stringify(next) ? section : next;
}

/** `next` with each section of `previous` kept where it says the same:
 * every push from the bridge is a whole new document, and a screen that
 * reads one section (the sidebar's view, the shortcuts) must draw nothing
 * when another one changed (track P's rule: an unchanged state draws
 * nothing). `previous` itself when nothing changed. */
export function keepSections(
  previous: Preferences,
  next: Preferences,
): Preferences {
  const kept: Preferences = {
    appearance: same(previous.appearance, next.appearance),
    layout: same(previous.layout, next.layout),
    composer: same(previous.composer, next.composer),
    shortcuts: same(previous.shortcuts, next.shortcuts),
    notifications: same(previous.notifications, next.notifications),
    sidebar: same(previous.sidebar, next.sidebar),
    defaults: same(previous.defaults, next.defaults),
  };
  const unchanged =
    kept.appearance === previous.appearance &&
    kept.layout === previous.layout &&
    kept.composer === previous.composer &&
    kept.shortcuts === previous.shortcuts &&
    kept.notifications === previous.notifications &&
    kept.sidebar === previous.sidebar &&
    kept.defaults === previous.defaults;
  return unchanged ? previous : kept;
}
