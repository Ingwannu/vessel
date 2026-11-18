// "Manage labels" (Paseo's manager modal), from the palette and the view
// menu: one machine's labels, searched; a label opened to rename,
// recolour or delete it.
//
// [Decision Log]
// - Reference: Paseo's manager-modal.tsx. The listing: the search in the
//   sheet's header (unboxed, under the title, not focused on open), the
//   HostFilter pill when there is more than one host (no "all"), then
//   one row per label on a single left rail (the 10px dot, the name, a
//   pencil that fades in on hover and is always there on touch and
//   phones), one empty line for no labels and no matches alike, and the
//   offline / update-host / error lines. Editing: the header title "Edit
//   label" with a back arrow, the Name field and the ten swatches, and a
//   footer with Delete (outline: red belongs to the confirmation) at the
//   left and Save (only for a real change, loading while it runs) at the
//   right. Delete first says how many workspaces carry the label.
// - Words: Paseo's, so "host" here where the rest of the page says
//   "machine".
// - Choice: the rules live in the model (data/workspace-labels/
//   manager.ts); this draws it. The kit Dialog's header back and header
//   search are Paseo's sheet header (kit/dialog.tsx).
// - Kept from before, a deliberate difference: back from editing keeps
//   the search, and the box shows it, so the box and the list agree.

import { useEffect, useMemo, useState, useSyncExternalStore } from "react";

import { useFleetState } from "../../context";
import { useStore } from "../../data/store";
import { labelManager } from "../../data/workspace-labels/hub";
import {
  type ManagedMachine,
  ManagerModel,
  type ManagerSnapshot,
} from "../../data/workspace-labels/manager";
import { labelKey } from "../../data/workspace-labels/names";
import { nodeLabel } from "../../data/nodes";
import {
  Button,
  Dialog,
  HostFilter,
  type HostOption,
  IconButton,
} from "../../ui/kit";
import { LabelDot } from "./chip";
import { DeleteLabelDialog } from "./delete-dialog";
import { LabelEditView } from "./edit-view";
import { useLabels } from "./use-labels";

type Label = ManagedMachine["labels"][number];

/** The word for a machine the manager cannot edit, in Paseo's terms. */
function stateWord(state: ManagedMachine["state"]): string {
  switch (state) {
    case "online":
      return "Online";
    case "offline":
      return "Offline";
    case "unsupported":
      return "Needs update";
  }
}

/** The machines for the kit's host filter: the dot green when online, and
 * why one cannot be edited after its name. */
function hostOptions(
  machines: readonly ManagedMachine[],
): HostOption<ManagedMachine["node"]>[] {
  return machines.map((machine) => ({
    value: machine.node,
    label: machine.name,
    tone: machine.state === "online" ? "ok" : "neutral",
    status: stateWord(machine.state),
    ...(machine.state === "online" ? {} : { detail: stateWord(machine.state) }),
  }));
}

/** The manager, drawn by the app's root while it is open. */
export function LabelManagerHost() {
  const open = useStore(labelManager, (value) => value);
  return open ? (
    <LabelManager
      onClose={() => {
        labelManager.set(false);
      }}
    />
  ) : null;
}

function LabelManager({ onClose }: { onClose: () => void }) {
  const { hub, state } = useLabels();
  const book = useFleetState((value) => value.book);
  const statuses = useFleetState((value) => value.statuses);
  const selected = useFleetState((value) => value.selected);
  const model = useMemo(
    () =>
      new ManagerModel({
        update: (node, name, change) => hub.update(node, name, change),
        inspect: (node, name) => hub.inspect(node, name),
        remove: (node, name) => hub.remove(node, name),
      }),
    [hub],
  );
  useEffect(() => {
    const machines: ManagedMachine[] = (book ?? []).map((node) => {
      const catalog = state.catalogs[node.key];
      return {
        node: node.key,
        name: nodeLabel(node, statuses[node.key]),
        state:
          catalog?.state === "online" || catalog?.state === "unsupported"
            ? catalog.state
            : "offline",
        labels: catalog?.labels ?? [],
        error: catalog?.error ?? null,
      };
    });
    model.syncMachines(machines, selected);
  }, [model, book, statuses, state.catalogs, selected]);
  const view = useSyncExternalStore(model.subscribe, model.snapshot);
  const [asking, setAsking] = useState<{
    name: string;
    count: number;
    answer: (yes: boolean) => void;
  } | null>(null);
  // Which button started the operation under way: only it draws busy.
  const [pressed, setPressed] = useState<"save" | "delete" | null>(null);
  const editable = view.machine?.state === "online";
  const save = () => {
    setPressed("save");
    void model.save();
  };
  const sheet =
    view.draft !== null && view.editing !== null
      ? {
          title: "Edit label",
          back: {
            label: "Back",
            onBack: () => {
              model.cancelEdit();
            },
          },
          footer: (
            <div className="label-manager-foot">
              <Button
                variant="outline"
                disabled={!editable || (view.pending && pressed !== "delete")}
                busy={view.pending && pressed === "delete" && asking === null}
                onClick={() => {
                  setPressed("delete");
                  void model.delete(
                    (count) =>
                      new Promise<boolean>((answer) => {
                        setAsking({
                          name: view.editing?.name ?? "",
                          count,
                          answer,
                        });
                      }),
                  );
                }}
              >
                Delete
              </Button>
              <Button
                variant="primary"
                disabled={
                  !view.dirty ||
                  !editable ||
                  (view.pending && pressed !== "save")
                }
                busy={view.pending && pressed === "save"}
                onClick={save}
              >
                Save
              </Button>
            </div>
          ),
        }
      : {
          title: "Manage labels",
          search: {
            value: view.query,
            onChange: (query: string) => {
              model.setQuery(query);
            },
            placeholder: "Search labels",
          },
        };
  return (
    <>
      <Dialog onClose={onClose} {...sheet}>
        {view.draft === null || view.editing === null ? (
          <LabelListing
            view={view}
            disabled={!editable || view.pending}
            onMachine={(node) => {
              model.selectMachine(node);
            }}
            onEdit={(label) => {
              model.edit(label);
            }}
          />
        ) : (
          <LabelEditView
            draft={view.draft}
            disabled={!editable || view.pending}
            error={view.error}
            onName={(typed) => {
              model.setName(typed);
            }}
            onColor={(color) => {
              model.setColor(color);
            }}
            onSubmit={save}
          />
        )}
      </Dialog>
      {asking === null ? null : (
        <DeleteLabelDialog
          name={asking.name}
          count={asking.count}
          onAnswer={(yes) => {
            asking.answer(yes);
            setAsking(null);
          }}
        />
      )}
    </>
  );
}

function LabelListing({
  view,
  disabled,
  onMachine,
  onEdit,
}: {
  view: ManagerSnapshot;
  disabled: boolean;
  onMachine: (node: ManagedMachine["node"]) => void;
  onEdit: (label: Label) => void;
}) {
  const machine = view.machine;
  const error = view.error ?? machine?.error ?? null;
  return (
    <>
      {view.machines.length > 1 && view.node !== null ? (
        <HostFilter
          hosts={hostOptions(view.machines)}
          value={view.node}
          onChange={onMachine}
          disabled={view.pending}
        />
      ) : null}
      <div className="label-list">
        {view.labels.map((label) => (
          <div key={labelKey(label.name)} className="label-row">
            <LabelDot color={label.color} />
            <span className="label-name">{label.name}</span>
            <IconButton
              label={`Edit ${label.name}`}
              icon="edit"
              size="xs"
              disabled={disabled}
              onClick={() => {
                onEdit(label);
              }}
            />
          </div>
        ))}
        {view.labels.length === 0 ? (
          <p className="label-note">No labels on this host.</p>
        ) : null}
      </div>
      {machine?.state === "offline" ? (
        <p className="label-note">This host is offline.</p>
      ) : null}
      {machine?.state === "unsupported" ? (
        <p className="label-note">Update this host to manage labels.</p>
      ) : null}
      {error === null ? null : <p className="label-note error">{error}</p>}
    </>
  );
}
