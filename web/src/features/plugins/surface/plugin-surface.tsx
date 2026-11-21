// A declared plugin surface drawn with the page's kit (ADR 0079 contract
// one, ADR 0115 "Surfaces"): what Paseo's screen and settings bodies are,
// as data. Paseo's host owns the frame (header, back, error boundary) and
// the plugin the body (public-docs/plugins/reference.md:700-716); here the
// body is the declared control tree.
//
// [Decision Log]
// - Purpose: one renderer for screens and settings screens (and, in P3b-2,
//   panels), so a control looks and behaves the same wherever it is.
// - Choice: a switch over every declared kind; TypeScript's exhaustive
//   switch makes a kind added to the wire fail the build here. Layout
//   kinds map to plain containers with the design's spacing; the kit draws
//   every control.
// - A device without admin (coordinator ruling on P3b, Q1) sees the whole
//   surface with one muted line at its top when anything on it needs admin;
//   the controls that would need it are disabled where they are drawn.
// - The documents a surface binds are read when it opens and again each
//   time the machine's `plugin.watch` generation moves.

import "./surface.css";

import { useEffect } from "react";

import { useFleet, useFleetState } from "../../../context";
import { Markdown } from "../../../data/markdown";
import { pluginCatalogs } from "../../../data/plugin-catalog";
import { dropDrafts } from "../../../data/plugin-settings";
import { clearRefusal, loadDoc } from "../../../data/plugin-settings-docs";
import { readOnlyLine } from "../../../data/plugin-surfaces";
import { useStore } from "../../../data/store";
import type { PluginControl, Surface } from "../../../wire/proto";
import { RowList, StatusDot } from "../../../ui/kit";
import { SettingsSection } from "../../settings/settings-page";
import {
  PluginButton,
  PluginChoice,
  PluginTextField,
  PluginToggle,
} from "./action-controls";
import {
  boundDocuments,
  needsAdmin,
  secretInputs,
  type SurfaceScope,
  toneOf,
} from "./scope";
import { DocStatus } from "./doc-status";
import {
  SettingsInputRow,
  SettingsSelectRow,
  SettingsSwitchRow,
} from "./settings-rows";

function ControlView({
  scope,
  control,
}: {
  scope: SurfaceScope;
  control: PluginControl;
}) {
  const content = control.content;
  switch (content.kind) {
    case "text":
      return <p className="plugin-text">{content.text}</p>;
    case "markdown":
      return (
        <div className="plugin-markdown">
          <Markdown httpsOnly>{content.text}</Markdown>
        </div>
      );
    case "stack":
      return (
        <div className="plugin-stack">
          <Controls scope={scope} controls={content.surface.controls} />
        </div>
      );
    case "row":
      return (
        <div className="plugin-row">
          <Controls scope={scope} controls={content.surface.controls} />
        </div>
      );
    case "section":
      return (
        <SettingsSection title={content.title}>
          <div className="plugin-stack">
            <Controls scope={scope} controls={content.surface.controls} />
          </div>
        </SettingsSection>
      );
    case "card":
      // Paseo's SettingsCard: the page's row list, a divider between rows.
      return (
        <RowList label={scope.title}>
          {content.surface.controls.map((child) => (
            <CardItem key={child.id} scope={scope} control={child} />
          ))}
        </RowList>
      );
    case "status":
      return (
        <span className="plugin-status-line">
          <StatusDot tone={toneOf(content.tone)} label={content.label} />
          <span>{content.label}</span>
        </span>
      );
    case "button":
      return <PluginButton scope={scope} button={content.button} />;
    case "text_field":
      return (
        <PluginTextField
          scope={scope}
          label={content.label}
          submit={content.submit}
        />
      );
    case "choice":
      return (
        <PluginChoice
          scope={scope}
          label={content.label}
          options={content.options}
          selected={content.selected}
          submit={content.submit}
        />
      );
    case "toggle":
      return (
        <PluginToggle
          scope={scope}
          label={content.label}
          state={content.state}
          submit={content.submit}
        />
      );
    case "settings_switch":
      return (
        <RowList label={content.label}>
          <SettingsSwitchRow scope={scope} control={content} />
        </RowList>
      );
    case "settings_select":
      return (
        <RowList label={content.label}>
          <SettingsSelectRow scope={scope} control={content} />
        </RowList>
      );
    case "settings_input":
      return (
        <RowList label={content.label}>
          <SettingsInputRow scope={scope} control={content} />
        </RowList>
      );
  }
}

/** One child of a card: a setting is a row of the list; anything else is
 * a row holding the control. */
function CardItem({
  scope,
  control,
}: {
  scope: SurfaceScope;
  control: PluginControl;
}) {
  const content = control.content;
  if (content.kind === "settings_switch")
    return <SettingsSwitchRow scope={scope} control={content} />;
  if (content.kind === "settings_select")
    return <SettingsSelectRow scope={scope} control={content} />;
  if (content.kind === "settings_input")
    return <SettingsInputRow scope={scope} control={content} />;
  return (
    <li className="row plugin-card-item">
      <ControlView scope={scope} control={control} />
    </li>
  );
}

/** Declared controls, in order. */
export function Controls({
  scope,
  controls,
}: {
  scope: SurfaceScope;
  controls: readonly PluginControl[];
}) {
  return controls.map((control) => (
    <ControlView key={control.id} scope={scope} control={control} />
  ));
}

/** Read the documents `surface` binds now and whenever the machine's
 * catalog generation moves (a save, reset or migration moves it). */
function useBoundDocuments(scope: SurfaceScope, surface: Surface) {
  const fleet = useFleet();
  const generation = useStore(
    pluginCatalogs,
    (all) => all.get(scope.node)?.catalog.generation ?? null,
  );
  // A node restart starts its watch count again, so a reconnect is a
  // reason to read again on its own (node review note 5).
  const connected = useFleetState(
    (state) => state.statuses[scope.node]?.state === "connected",
  );
  const plugin = scope.installation.address.plugin;
  const documents = boundDocuments(surface).join("\n");
  // Strings, so a catalog re-read (a new surface object) does not count
  // as leaving the surface.
  const secrets = secretInputs(surface).join("\n");
  useEffect(() => {
    if (!scope.mayInvoke || documents === "" || !connected) return;
    for (const settings of documents.split("\n"))
      void loadDoc(fleet, { node: scope.node, plugin, settings });
  }, [
    fleet,
    scope.node,
    scope.mayInvoke,
    plugin,
    documents,
    generation,
    connected,
  ]);
  // Leaving the surface clears its refusals and drops a secret input's
  // text (node review notes 1 and 8); other drafts stay (ADR 0115).
  useEffect(
    () => () => {
      for (const settings of documents.split("\n")) {
        if (settings === "") continue;
        const at = { node: scope.node, plugin, settings };
        clearRefusal(at, null);
        const keys = secrets
          .split("\n")
          .filter((pair) => pair.startsWith(`${settings}\u0000`))
          .map((pair) => pair.slice(settings.length + 1));
        dropDrafts(at, keys);
      }
    },
    [scope.node, plugin, documents, secrets],
  );
}

/** A whole declared surface. `machine` names the machine in the muted line
 * a device without admin sees. */
export function PluginSurface({
  scope,
  surface,
  machine,
}: {
  scope: SurfaceScope;
  surface: Surface;
  machine: string;
}) {
  useBoundDocuments(scope, surface);
  return (
    <div className="plugin-surface">
      {!scope.mayInvoke && needsAdmin(surface) ? (
        <p className="plugin-read-only">{readOnlyLine(machine)}</p>
      ) : null}
      {scope.mayInvoke
        ? boundDocuments(surface).map((settings) => (
            <DocStatus key={settings} scope={scope} settings={settings} />
          ))
        : null}
      <Controls scope={scope} controls={surface.controls} />
    </div>
  );
}
