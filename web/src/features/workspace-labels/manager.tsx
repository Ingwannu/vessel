// "Manage labels" (Paseo's manager modal), from the palette and the view
// menu: one machine's labels, searched; a label opened to rename,
// recolour or delete it.
//
// [Decision Log]
// - Reference: Paseo's manager lists a host's labels with their colours,
//   a search over them, and an empty state; opening one shows its name
//   and colour with Save (only when something changed) and Delete, which
//   first says how many workspaces carry the label and asks; a host that
//   is offline or too old says so instead of offering edits.
// - Choice: the same, with the machine picked from a select when there
//   are several (Paseo's host filter). The rules live in the model
//   (data/workspace-labels/manager.ts); this draws it.

import { useEffect, useMemo, useState, useSyncExternalStore } from "react";

import { useFleetState } from "../../context";
import { useStore } from "../../data/store";
import { labelManager } from "../../data/workspace-labels/hub";
import {
  type ManagedMachine,
  ManagerModel,
} from "../../data/workspace-labels/manager";
import { labelKey } from "../../data/workspace-labels/names";
import { nodeLabel } from "../../data/nodes";
import {
  Button,
  Dialog,
  EmptyState,
  Field,
  FieldGroup,
  Select,
  TextField,
} from "../../ui/kit";
import { LabelDot, LabelSwatches } from "./chip";
import { DeleteLabelDialog } from "./delete-dialog";
import { useLabels } from "./use-labels";

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
      };
    });
    model.syncMachines(machines);
  }, [model, book, statuses, state.catalogs]);
  const view = useSyncExternalStore(model.subscribe, model.snapshot);
  const [asking, setAsking] = useState<{
    name: string;
    count: number;
    answer: (yes: boolean) => void;
  } | null>(null);
  // Which button started the operation under way: only it draws busy.
  const [pressed, setPressed] = useState<"save" | "delete" | null>(null);
  const editable = view.machine?.state === "online";
  const editing = view.draft !== null && view.editing !== null;
  const footer = editing ? (
    <>
      <Button
        variant="danger"
        disabled={(view.pending && pressed !== "delete") || !editable}
        busy={view.pending && pressed === "delete" && asking === null}
        onClick={() => {
          setPressed("delete");
          void model.delete(
            (count) =>
              new Promise<boolean>((answer) => {
                setAsking({ name: view.editing?.name ?? "", count, answer });
              }),
          );
        }}
      >
        Delete
      </Button>
      <Button
        variant="ghost"
        disabled={view.pending}
        onClick={() => {
          model.cancelEdit();
        }}
      >
        Back
      </Button>
      <Button
        variant="primary"
        disabled={
          !view.dirty || !editable || (view.pending && pressed !== "save")
        }
        busy={view.pending && pressed === "save"}
        onClick={() => {
          setPressed("save");
          void model.save();
        }}
      >
        Save
      </Button>
    </>
  ) : undefined;
  return (
    <>
      <Dialog
        title={editing ? "Edit label" : "Manage labels"}
        onClose={onClose}
        {...(footer === undefined ? {} : { footer })}
      >
        {view.machines.length > 1 ? (
          <Field label="Machine">
            <Select
              value={view.node ?? ""}
              onChange={(node) => {
                const chosen = view.machines.find((each) => each.node === node);
                if (chosen !== undefined) {
                  model.selectMachine(chosen.node);
                }
              }}
              options={view.machines.map((machine) => ({
                value: machine.node,
                label: machine.name,
              }))}
              disabled={view.pending || editing}
            />
          </Field>
        ) : null}
        {editing && view.draft !== null ? (
          <>
            <Field label="Name" error={view.error}>
              <TextField
                value={view.draft.typed}
                onChange={(typed) => {
                  model.setName(typed);
                }}
                disabled={view.pending || !editable}
                autoFocus
              />
            </Field>
            <FieldGroup label="Color">
              <LabelSwatches
                value={view.draft.color}
                onChange={(color) => {
                  model.setColor(color);
                }}
                disabled={view.pending || !editable}
              />
            </FieldGroup>
          </>
        ) : (
          <LabelList
            view={view}
            onSearch={(query) => {
              model.setQuery(query);
            }}
            onOpen={(label) => {
              model.edit(label);
            }}
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

function LabelList({
  view,
  onSearch,
  onOpen,
}: {
  view: ReturnType<ManagerModel["snapshot"]>;
  onSearch: (query: string) => void;
  onOpen: (label: {
    name: string;
    color: ManagedMachine["labels"][number]["color"];
  }) => void;
}) {
  const machine = view.machine;
  if (machine === null) {
    return <EmptyState title="No machines" />;
  }
  if (machine.state === "unsupported") {
    return <p className="label-note">Update this machine to manage labels.</p>;
  }
  if (machine.state === "offline") {
    return <p className="label-note">This machine is offline.</p>;
  }
  return (
    <>
      <TextField
        value={view.query}
        onChange={onSearch}
        placeholder="Search labels"
        icon="search"
        aria-label="Search labels"
        role="searchbox"
      />
      {view.labels.length === 0 ? (
        <p className="label-note">
          {machine.labels.length === 0
            ? "No labels on this machine."
            : "No labels match."}
        </p>
      ) : (
        <div className="label-list">
          {view.labels.map((label) => (
            <button
              key={labelKey(label.name)}
              type="button"
              className="label-row"
              onClick={() => {
                onOpen(label);
              }}
            >
              <LabelDot color={label.color} />
              <span className="label-name">{label.name}</span>
            </button>
          ))}
        </div>
      )}
    </>
  );
}
