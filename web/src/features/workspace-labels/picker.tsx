// A workspace row's "Labels" submenu (Paseo's row-menu picker): every
// label of the workspace's machine with its colour, the ones it carries
// checked at the right, a press toggling one while the menu stays open;
// "Create label…" under them.
//
// [Decision Log]
// - Reference: Paseo's picker page in the workspace row menu, multi-select
//   with one press in flight per label, its error under the list, a note
//   when the host is too old, and a create page with a name, a colour and
//   a Create row.
// - Difference: creating opens a small dialog rather than a page inside
//   the menu. The kit's menus have no text field (typing in a Base UI menu
//   moves the highlight), and a dialog keeps the name field an ordinary
//   field. The dialog assigns the new label to this workspace, as Paseo's
//   create page does.

import { useEffect, useMemo, useSyncExternalStore } from "react";

import { labelKey } from "../../data/workspace-labels/names";
import { PickerModel } from "../../data/workspace-labels/picker";
import type { NodeKey } from "../../wire/bridge";
import type { WorkspaceId } from "../../wire/proto";
import {
  MenuCheckboxItem,
  MenuItem,
  MenuNote,
  MenuSeparator,
  MenuSubmenu,
} from "../../ui/kit";
import { LabelDot } from "./chip";
import { useLabels } from "./use-labels";

export function LabelsSubmenu({
  node,
  workspace,
  assigned,
  onCreate,
}: {
  node: NodeKey;
  workspace: WorkspaceId;
  /** The names the workspace carries now. */
  assigned: readonly string[];
  onCreate: () => void;
}) {
  const { hub, state } = useLabels();
  const catalog = state.catalogs[node];
  const model = useMemo(
    () =>
      new PickerModel((label, on) => hub.assign(node, workspace, label, on)),
    [hub, node, workspace],
  );
  useEffect(() => {
    model.sync({
      labels: catalog?.labels ?? [],
      assigned,
      online: catalog?.state === "online",
    });
  }, [model, catalog, assigned]);
  const snapshot = useSyncExternalStore(model.subscribe, model.snapshot);
  const pending = new Set(snapshot.pending);
  const loadError =
    catalog?.error === null || catalog?.error === undefined
      ? null
      : "Unable to load labels";
  const error = snapshot.error ?? loadError;
  return (
    <MenuSubmenu label="Labels" icon="tag">
      {snapshot.rows.map((row) => (
        <MenuCheckboxItem
          key={labelKey(row.name)}
          checked={row.assigned}
          disabled={!snapshot.online || pending.has(labelKey(row.name))}
          leading={<LabelDot color={row.color} />}
          onChange={(on) => {
            void model.toggle({ name: row.name, color: row.color }, on);
          }}
        >
          {row.name}
        </MenuCheckboxItem>
      ))}
      {snapshot.rows.length > 0 ? <MenuSeparator /> : null}
      <MenuItem icon="plus" disabled={!snapshot.online} onSelect={onCreate}>
        Create label…
      </MenuItem>
      {error === null ? null : <MenuNote>{error}</MenuNote>}
      {catalog?.state === "unsupported" ? (
        <MenuNote>Update this machine to use labels.</MenuNote>
      ) : null}
    </MenuSubmenu>
  );
}
