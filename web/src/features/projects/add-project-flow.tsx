// The Add project sheet (TRACKER 234-238, 242-243), Paseo's
// `add-project-flow.tsx` (pinned 0.15.4): a page stack over a machine,
// its methods, a directory search, a clone's repository and destination,
// and a new directory's parent and name. The behaviour is the model's
// (`data/workspace/add-project/`); this draws its last page and runs the
// chosen row's call. Plan and differences:
// docs/plans/wp-add-project-flow.md.
//
// [Decision Log]
// - Purpose: one Add project for every place that adds a project (the
//   sidebar's "+", Settings > Projects), replacing the three dialogs.
// - Constraint: the page is a browser page: no Browse (Paseo offers it in
//   its desktop app only); a machine below `folder.suggest` gets no
//   folder rows; one below ADR 0113 gets no `~` path and no New directory.
// - Choice: the kit's FlowSheet (Paseo's panel; Esc goes one page back,
//   closing on the first; an outside press closes) and Paseo's rows in
//   this page's colours. Machines arriving or going adjust the state
//   while rendering (React's pattern for state from props), not in an
//   effect. The parts: the machines (`add-project-machines.ts`), the
//   field's folders (`use-machine-folders.ts`), the rows' actions
//   (`add-project-choose.ts`), the calls (`use-add-project-submit.ts`),
//   the list (`add-project-results.tsx`).
// - Cost: a clone closes the sheet when the node answers `cloning`
//   (difference 7); its progress is the project's row in Settings.

import "./add-project-flow.css";

import {
  type KeyboardEvent,
  type ReactElement,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { useFleet, useFleetState } from "../../context";
import { useStore } from "../../data/store";
import { PROJECT_DIRECTORY_PROTOCOL } from "../../data/workspace/add-project/calls";
import {
  moveAddProjectSelection,
  setAddProjectActiveIndex,
  setAddProjectPageInput,
  setNewDirectoryName,
} from "../../data/workspace/add-project/editing";
import {
  type AddProjectFlowState,
  applyAvailableMachines,
  backAddProjectPage,
  currentAddProjectPage,
  openAddProjectFlow,
} from "../../data/workspace/add-project/model";
import {
  addProjectRequest,
  closeAddProject,
} from "../../data/workspace/add-project/request";
import {
  buildRows,
  type FlowRow,
  pageInput,
  pagePlaceholder,
  pageTitle,
  searchesDirectories,
} from "../../data/workspace/add-project/rows";
import { useProjects } from "../../data/workspace/project-store";
import { FlowHint, FlowSheet } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import {
  type ChoiceContext,
  chooseRow,
  createDirectory,
} from "./add-project-choose";
import { useAddProjectMachines } from "./add-project-machines";
import { AddProjectResults } from "./add-project-results";
import { useAddProjectSubmit } from "./use-add-project-submit";
import { useMachineFolders } from "./use-machine-folders";

/** Where the last clone on each machine went (Paseo's
 * `lastCloneParentByHost`, kept for the page's life). */
const lastCloneParent = new Map<NodeKey, string>();

/** The open sheet, keyed by its request so a new opening starts fresh. */
export function AddProjectFlowHost(): ReactElement | null {
  const request = useStore(addProjectRequest, (value) => value);
  if (request === null) return null;
  return (
    <AddProjectFlow
      key={request.id}
      preferred={request.preferred}
      onClose={closeAddProject}
    />
  );
}

function AddProjectFlow({
  preferred,
  onClose,
}: {
  preferred: NodeKey | undefined;
  onClose: () => void;
}): ReactElement {
  const fleet = useFleet();
  const statuses = useFleetState((state) => state.statuses);
  const machines = useAddProjectMachines();
  const [state, setState] = useState<AddProjectFlowState>(() =>
    openAddProjectFlow({
      machines,
      ...(preferred === undefined ? {} : { preferred }),
    }),
  );
  const [seen, setSeen] = useState(machines);
  if (seen !== machines) {
    setSeen(machines);
    setState(applyAvailableMachines(state, machines, preferred));
  }

  const page = currentAddProjectPage(state);
  const node = page.kind === "host" ? null : page.node;
  const machine = state.machines.find((each) => each.node === node) ?? null;
  const status = node === null ? undefined : statuses[node];
  const protocol = status?.state === "connected" ? status.protocol : 0;
  const projects = useProjects(fleet, node);
  const recentPaths = useMemo(
    () =>
      projects.kind === "ready"
        ? projects.list.projects.map((project) => project.root)
        : [],
    [projects],
  );
  const query =
    page.kind === "method" || page.kind === "new-directory-name"
      ? ""
      : page.query;
  const folders = useMachineFolders(
    fleet,
    node,
    query,
    searchesDirectories(page),
    protocol,
  );
  const rows = buildRows({
    page,
    machines: state.machines,
    machine,
    recentPaths,
    machinePaths: folders.paths,
    shown: folders.shown,
    lastCloneParent: node === null ? null : (lastCloneParent.get(node) ?? null),
    tilde: protocol >= PROJECT_DIRECTORY_PROTOCOL,
  });
  const activeIndex =
    rows.length === 0 ? 0 : Math.min(page.activeIndex, rows.length - 1);
  const submit = useAddProjectSubmit(fleet, node, setState, onClose);

  const back = (): void => {
    const previous = backAddProjectPage(state);
    if (previous === null) onClose();
    else setState(previous);
  };

  const context: ChoiceContext = {
    fleet,
    node,
    page,
    setState,
    submit,
    onClose,
    lastCloneParent,
  };
  const choose = (row: FlowRow): void => {
    chooseRow(row, context);
  };

  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key === "Enter") {
      event.preventDefault();
      const row = rows[activeIndex];
      if (page.kind === "new-directory-name") createDirectory(context);
      else if (row !== undefined) choose(row);
      return;
    }
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const next = moveAddProjectSelection(
      activeIndex,
      rows.map((row) => !row.disabled),
      event.key === "ArrowDown" ? "next" : "previous",
    );
    setState((current) => setAddProjectActiveIndex(current, next));
  };

  // The field takes focus on each page; the active row stays in view.
  const field = useRef<HTMLInputElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const timer = setTimeout(() => {
      (field.current ?? panel.current)?.focus();
    }, 0);
    return () => {
      clearTimeout(timer);
    };
  }, [page.kind]);
  const active = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    active.current?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, rows.length]);

  const placeholder = pagePlaceholder(page);
  return (
    <FlowSheet
      label={`Add project: ${page.kind}`}
      title={pageTitle(page)}
      {...(machine === null ? {} : { context: machine.label })}
      {...(state.pages.length > 1 ? { onBack: back } : {})}
      onEscape={back}
      onClose={onClose}
      onKeyDown={onKeyDown}
      popupRef={panel}
      fieldRef={field}
      initialFocus={() => field.current ?? panel.current}
      field={
        placeholder === null
          ? undefined
          : {
              value: pageInput(page),
              placeholder,
              disabled: "isSubmitting" in page && page.isSubmitting,
              onChange: (value) => {
                setState((current) =>
                  currentAddProjectPage(current).kind === "new-directory-name"
                    ? setNewDirectoryName(current, value)
                    : setAddProjectPageInput(current, value),
                );
              },
            }
      }
      footer={
        <>
          <FlowHint keys={["Up", "Down"]} action="Navigate" />
          <FlowHint keys={["Enter"]} action="Select" />
          <FlowHint
            keys={["Esc"]}
            action={state.pages.length > 1 ? "Back" : "Close"}
          />
        </>
      }
    >
      <AddProjectResults
        page={page}
        machine={machine}
        rows={rows}
        activeIndex={activeIndex}
        activeRef={active}
        loading={folders.loading}
        queryError={folders.error}
        onChoose={choose}
      />
    </FlowSheet>
  );
}
