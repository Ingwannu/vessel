// The Display menu's "Labels" (Paseo's Labels page of its display
// preferences): Unlabelled and every label of the connected machines, any
// number picked, the sidebar then listing only workspaces carrying one of
// them; "Clear filter" when something is picked.
//
// [Decision Log]
// - Reference: Paseo offers the Labels page with its workspace grouping,
//   Unlabelled first, then the merged labels of every host, each a toggle
//   that keeps the menu open; the pick is kept with the view.
// - Choice: the same as a submenu; the picks are kept by the bridge with
//   the rest of the view (`sidebar_labels`). It is drawn only while the
//   menu is open, so it holds the label hub only then.

import { useFleet } from "../../context";
import { prefer } from "../../data/prefs";
import { mergeCatalogs } from "../../data/workspace-labels/merge";
import { labelKey, UNLABELLED } from "../../data/workspace-labels/names";
import { entriesOf } from "../../data/records";
import {
  MenuCheckboxItem,
  MenuItem,
  MenuNote,
  MenuSeparator,
  MenuSubmenu,
} from "../../ui/kit";
import { toast } from "../../ui/toasts";
import { LabelDot } from "./chip";
import { useLabels } from "./use-labels";

export function LabelFilterMenu({ picked }: { picked: readonly string[] }) {
  const fleet = useFleet();
  const { state } = useLabels();
  const labels = mergeCatalogs(
    entriesOf(state.catalogs)
      .filter(([, catalog]) => catalog.state === "online")
      .map(([node, catalog]) => ({ node, labels: catalog.labels })),
  );
  const keys = new Set(picked.map(labelKey));
  const set = (picks: string[]) => {
    void prefer(fleet, { set: "sidebar_labels", value: { picks } }).then(
      (outcome) => {
        if (!outcome.ok) {
          toast(outcome.problem.message, "error");
        }
      },
    );
  };
  const toggle = (name: string, on: boolean) => {
    const key = labelKey(name);
    set(
      on ? [...picked, name] : picked.filter((pick) => labelKey(pick) !== key),
    );
  };
  return (
    <MenuSubmenu
      label="Labels"
      icon="tag"
      {...(picked.length === 0 ? {} : { value: String(picked.length) })}
    >
      <MenuCheckboxItem
        checked={keys.has(UNLABELLED)}
        leading={<LabelDot color={null} />}
        onChange={(on) => {
          toggle(UNLABELLED, on);
        }}
      >
        Unlabelled
      </MenuCheckboxItem>
      {labels.map((label) => (
        <MenuCheckboxItem
          key={labelKey(label.name)}
          checked={keys.has(labelKey(label.name))}
          leading={<LabelDot color={label.color} />}
          onChange={(on) => {
            toggle(label.name, on);
          }}
        >
          {label.name}
        </MenuCheckboxItem>
      ))}
      {labels.length === 0 ? <MenuNote>No labels yet.</MenuNote> : null}
      {picked.length === 0 ? null : (
        <>
          <MenuSeparator />
          <MenuItem
            icon="close"
            onSelect={() => {
              set([]);
            }}
          >
            Clear filter
          </MenuItem>
        </>
      )}
    </MenuSubmenu>
  );
}
