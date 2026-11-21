// What every control of one drawn plugin surface shares (ADR 0115,
// "Surfaces"): the machine and run it came from, whether this device may
// call its methods, and the context its calls carry.
//
// [Decision Log]
// - Purpose: a surface is a tree of declared controls; each one that acts
//   needs the same captured run (ADR 0079: an action carries its
//   generation) and the same answer to "may this device invoke".
// - Choice: one plain value passed down the tree; the pure walks over a
//   surface (which documents it binds, whether anything invokes) live here
//   and are tested on their own.

import type { NodeKey } from "../../../wire/bridge";
import type {
  ClientInstallation,
  InvokeContext,
  MenuEntry,
  PluginControl,
  StatusTone,
  Surface,
} from "../../../wire/proto";
import type { Tone } from "../../../ui/kit";
import type { SurfaceTarget } from "../../../data/plugin-surfaces";

/** One drawn surface's shared facts. */
export interface SurfaceScope {
  node: NodeKey;
  installation: ClientInstallation;
  mayInvoke: boolean;
  context: InvokeContext;
  /** Open another surface of the same plugin. */
  open: (target: SurfaceTarget) => void;
  /** The contribution's title: the name of its cards' lists for a screen
   * reader (node review note 9). */
  title: string;
}

/** The kit tone of a declared status tone. */
export function toneOf(tone: StatusTone): Tone {
  switch (tone) {
    case "neutral":
      return "neutral";
    case "success":
      return "ok";
    case "warning":
      return "warn";
    case "danger":
      return "danger";
  }
}

/** Every control of `surface`, nested ones included (containers, popovers
 * and menus), in order. */
export function everyControl(surface: Surface): PluginControl[] {
  const found: PluginControl[] = [];
  const visit = (controls: readonly PluginControl[]) => {
    for (const control of controls) {
      found.push(control);
      const content = control.content;
      switch (content.kind) {
        case "stack":
        case "row":
        case "section":
        case "card":
          visit(content.surface.controls);
          break;
        case "button":
          if (content.button.behavior.kind === "popover")
            visit(content.button.behavior.surface.controls);
          break;
        case "text":
        case "markdown":
        case "text_field":
        case "choice":
        case "toggle":
        case "status":
        case "settings_switch":
        case "settings_select":
        case "settings_input":
          break;
      }
    }
  };
  visit(surface.controls);
  return found;
}

/** The settings documents `surface` binds, each once, in order. */
export function boundDocuments(surface: Surface): string[] {
  const documents = new Set<string>();
  for (const { content } of everyControl(surface)) {
    if (
      content.kind === "settings_switch" ||
      content.kind === "settings_select" ||
      content.kind === "settings_input"
    )
      documents.add(content.settings);
  }
  return [...documents];
}

/** Whether any entry of a menu, nested menus included, needs admin:
 * opening a screen does not (node review note 7). */
function menuNeedsAdmin(items: readonly MenuEntry[]): boolean {
  return items.some((entry) => {
    if (entry.kind === "separator") return false;
    const behavior = entry.behavior;
    if (behavior.kind === "menu") return menuNeedsAdmin(behavior.items);
    if (behavior.kind === "popover") return false;
    return behavior.action.kind !== "open_screen";
  });
}

/** Whether a device without admin would find something it cannot use:
 * a control that calls a method or a bound settings control. */
export function needsAdmin(surface: Surface): boolean {
  return everyControl(surface).some(({ content }) => {
    switch (content.kind) {
      case "text_field":
      case "choice":
      case "toggle":
      case "settings_switch":
      case "settings_select":
      case "settings_input":
        return true;
      case "button": {
        const behavior = content.button.behavior;
        // A popover's own controls are walked; opening a screen needs none.
        if (behavior.kind === "popover") return false;
        return behavior.kind === "menu"
          ? menuNeedsAdmin(behavior.items)
          : behavior.action.kind !== "open_screen";
      }
      case "text":
      case "markdown":
      case "stack":
      case "row":
      case "section":
      case "card":
      case "status":
        return false;
    }
  });
}

/** The secret inputs of `surface`, each as `<settings>\u0000<key>`. */
export function secretInputs(surface: Surface): string[] {
  return everyControl(surface).flatMap(({ content }) =>
    content.kind === "settings_input" && content.mode === "secret"
      ? [`${content.settings}\u0000${content.key}`]
      : [],
  );
}
