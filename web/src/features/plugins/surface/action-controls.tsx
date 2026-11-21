// The declared controls that act (ADR 0079 contract one, ADR 0115
// "Surfaces"): buttons with an action, a menu or a popover, and the fields
// whose change calls a plugin method.
//
// [Decision Log]
// - Purpose: a plugin's screen offers actions as data; the page draws them
//   with its kit and runs them against the run the catalog named.
// - What a control sends (ADR 0115, "What a control sends"): a text field
//   `{value: <text>}` on Enter, a choice `{value: <option id>}`, a toggle
//   `{value: <bool>}`, a button its declared input, each with the
//   surface's context. A failure is a toast, as the palette's plugin items
//   report one; success is the plugin's to show (it can declare again).
// - A device without admin sees these disabled (coordinator ruling on P3b,
//   Q1); opening another screen needs no admin and stays enabled.

import { useState } from "react";

import { useFleet } from "../../../context";
import { invokePlugin } from "../../../data/plugin-catalog";
import { problemWords } from "../../../data/plugins";
import type { Fleet } from "../../../data/fleet";
import type {
  Action,
  Button as ButtonData,
  MenuEntry,
  PluginChoiceOption,
  RawJson,
  ToggleState,
} from "../../../wire/proto";
import {
  Button,
  Field,
  Menu,
  MenuItem,
  MenuSeparator,
  MenuSubmenu,
  Popover,
  Select,
  Switch,
  TextField,
} from "../../../ui/kit";
import { toast } from "../../../ui/toasts";
import type { SurfaceScope } from "./scope";
import { Controls } from "./plugin-surface";

/** Whether this device may take `action`. */
function allowed(scope: SurfaceScope, action: Action): boolean {
  return action.kind === "open_screen" || scope.mayInvoke;
}

/** Call `method` with `input`; a failure becomes a toast. Resolves when
 * the call has an outcome, either way. */
function call(
  fleet: Fleet,
  scope: SurfaceScope,
  method: string,
  input: RawJson,
): Promise<void> {
  return invokePlugin(fleet, {
    node: scope.node,
    installation: scope.installation,
    method,
    input,
    context: scope.context,
  }).then((outcome) => {
    if (!outcome.ok) toast(problemWords(outcome), "error");
  });
}

/** Take `action`. */
function take(fleet: Fleet, scope: SurfaceScope, action: Action) {
  const plugin = scope.installation.address.plugin;
  switch (action.kind) {
    case "invoke":
      void call(fleet, scope, action.method, action.input);
      return;
    case "open_screen":
      scope.open({
        kind: "screen",
        node: scope.node,
        plugin,
        screen: action.screen,
      });
      return;
    case "open_settings":
      scope.open({
        kind: "settings",
        node: scope.node,
        plugin,
        screen: action.screen,
      });
      return;
  }
}

function Entries({
  scope,
  items,
}: {
  scope: SurfaceScope;
  items: readonly MenuEntry[];
}) {
  const fleet = useFleet();
  return items.map((entry) => {
    if (entry.kind === "separator") return <MenuSeparator key={entry.id} />;
    if (entry.availability === "hidden") return null;
    const behavior = entry.behavior;
    if (behavior.kind === "menu")
      return (
        <MenuSubmenu key={entry.id} label={entry.title}>
          <Entries scope={scope} items={behavior.items} />
        </MenuSubmenu>
      );
    // A popover inside a menu has no anchor once the menu closes; it is
    // offered as its button outside menus only.
    if (behavior.kind === "popover") return null;
    const action = behavior.action;
    return (
      <MenuItem
        key={entry.id}
        disabled={entry.availability === "disabled" || !allowed(scope, action)}
        onSelect={() => {
          take(fleet, scope, action);
        }}
      >
        {entry.title}
      </MenuItem>
    );
  });
}

/** A declared button. */
export function PluginButton({
  scope,
  button,
}: {
  scope: SurfaceScope;
  button: ButtonData;
}) {
  const fleet = useFleet();
  if (button.availability === "hidden") return null;
  const off = button.availability === "disabled";
  const behavior = button.behavior;
  switch (behavior.kind) {
    case "action":
      return (
        <Button
          size="sm"
          disabled={off || !allowed(scope, behavior.action)}
          onClick={() => {
            take(fleet, scope, behavior.action);
          }}
        >
          {button.title}
        </Button>
      );
    case "menu":
      return (
        <Menu
          align="start"
          trigger={
            <Button size="sm" icon="chevron-down" disabled={off}>
              {button.title}
            </Button>
          }
        >
          <Entries scope={scope} items={behavior.items} />
        </Menu>
      );
    case "popover":
      return (
        <Popover
          title={button.title}
          trigger={
            <Button size="sm" disabled={off}>
              {button.title}
            </Button>
          }
        >
          <Controls scope={scope} controls={behavior.surface.controls} />
        </Popover>
      );
  }
}

/** A text field; Enter sends its text. */
export function PluginTextField({
  scope,
  label,
  submit,
}: {
  scope: SurfaceScope;
  label: string;
  submit: string;
}) {
  const fleet = useFleet();
  const [text, setText] = useState("");
  return (
    <Field label={label} disabled={!scope.mayInvoke}>
      <TextField
        value={text}
        onChange={setText}
        disabled={!scope.mayInvoke}
        onKeyDown={(event) => {
          if (event.key !== "Enter") return;
          event.preventDefault();
          void call(fleet, scope, submit, { value: text });
        }}
      />
    </Field>
  );
}

/** A finite choice; a pick sends its option id. */
export function PluginChoice({
  scope,
  label,
  options,
  selected,
  submit,
}: {
  scope: SurfaceScope;
  label: string;
  options: readonly PluginChoiceOption[];
  selected: string | null;
  submit: string;
}) {
  const fleet = useFleet();
  // The declared value is drawn; a pick shows until its call has an
  // outcome, then the declaration (which the helper may send again) wins
  // (node review R3).
  const [pending, setPending] = useState<string | null>(null);
  const value = pending ?? selected;
  return (
    <Field label={label} disabled={!scope.mayInvoke}>
      <Select
        aria-label={label}
        value={value}
        disabled={!scope.mayInvoke}
        options={options.map((option) => ({
          value: option.id,
          label: option.title,
        }))}
        onChange={(next: string) => {
          setPending(next);
          void call(fleet, scope, submit, { value: next }).then(() => {
            setPending(null);
          });
        }}
      />
    </Field>
  );
}

/** A switch; a change sends its new state. */
export function PluginToggle({
  scope,
  label,
  state,
  submit,
}: {
  scope: SurfaceScope;
  label: string;
  state: ToggleState;
  submit: string;
}) {
  const fleet = useFleet();
  // As the choice: the declared state, with a change shown only until its
  // call has an outcome (node review R3).
  const [pending, setPending] = useState<boolean | null>(null);
  const on = pending ?? state === "on";
  return (
    <Switch
      label={label}
      showLabel
      checked={on}
      disabled={!scope.mayInvoke}
      onChange={(next) => {
        setPending(next);
        void call(fleet, scope, submit, { value: next }).then(() => {
          setPending(null);
        });
      }}
    />
  );
}
