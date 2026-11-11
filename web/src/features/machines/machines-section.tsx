// Settings › Machines: GOAT's environments section. A row per machine,
// "Add machine" at the right of the heading, and the chosen machine's
// detail under the list (the Fleet page and its inspector, folded into
// Settings as agent-orchestrator and GOAT keep machines there).

import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { openAddMachine } from "../../data/add-machine";
import { makeDefault } from "../../data/book";
import { nodeLabel } from "../../data/nodes";
import { nextStep } from "../../data/outcome";
import type { BookNode, NodeKey } from "../../wire/bridge";
import { Banner, Button, EmptyState, RowList, Skeleton } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import { SettingsPage, SettingsSection } from "../settings/settings-page";
import { ForgetDialog } from "./forget-dialog";
import { MachineDetail } from "./machine-detail";
import type { MachineAction } from "./machine-menu";
import { MachineRow } from "./machine-row";
import { RenameDialog } from "./rename-dialog";

type Asking = { kind: "rename" | "forget"; node: BookNode } | null;

export function MachinesSection({ machine }: { machine: NodeKey | undefined }) {
  const fleet = useFleet();
  const navigate = useNavigate();
  const book = useFleetState((state) => state.book);
  const bookError = useFleetState((state) => state.bookError);
  const statuses = useFleetState((state) => state.statuses);
  const chosen = book?.find((node) => node.key === machine) ?? null;
  const [asking, setAsking] = useState<Asking>(null);
  // The machine in view is the selected one (DESIGN.md §2.5), so New
  // opens on it.
  useEffect(() => {
    if (chosen !== null) {
      fleet.select(chosen.key);
    }
  }, [chosen, fleet]);
  const act = (action: MachineAction, node: BookNode) => {
    switch (action) {
      case "rename":
      case "forget":
        setAsking({ kind: action, node });
        return;
      case "default":
        void makeDefault(fleet, node.key).then((outcome) => {
          const label = nodeLabel(node, statuses[node.key]);
          if (outcome.ok) {
            toast(`${label} is the default machine`);
          } else {
            toast(
              `${outcome.problem.message} ${nextStep(outcome.problem)}`,
              "error",
            );
          }
        });
        return;
    }
  };
  return (
    <SettingsPage lead="The machines this page reaches. Sessions run on them, each with the agents installed there.">
      {bookError === null ? null : (
        <Banner
          tone="error"
          action={
            <Button size="sm" onClick={() => void fleet.loadBook()}>
              Try again
            </Button>
          }
        >
          The list of machines could not be read: {bookError}
        </Banner>
      )}
      <SettingsSection
        title="Machines"
        action={
          <Button
            variant="ghost"
            size="xs"
            icon="plus"
            onClick={openAddMachine}
          >
            Add machine
          </Button>
        }
      >
        {book === null ? (
          <Skeleton />
        ) : book.length === 0 ? (
          <EmptyState
            title="No machines yet"
            icon="machine"
            action={
              <Button variant="primary" icon="plus" onClick={openAddMachine}>
                Add a machine
              </Button>
            }
          >
            Add one with a link from that machine.
          </EmptyState>
        ) : (
          <RowList label="Machines">
            {book.map((node) => (
              <MachineRow
                key={node.key}
                node={node}
                selected={node.key === chosen?.key}
                onAction={act}
              />
            ))}
          </RowList>
        )}
      </SettingsSection>
      {chosen === null ? null : <MachineDetail node={chosen} />}
      {asking === null ? null : asking.kind === "rename" ? (
        <RenameDialog
          node={asking.node}
          onClose={() => {
            setAsking(null);
          }}
          onRenamed={(name) => {
            setAsking(null);
            // The open detail follows the machine to its new name.
            if (asking.node.key === chosen?.key) {
              void navigate({
                to: "/settings/$section",
                params: { section: "machines" },
                search: { machine: name },
                replace: true,
              });
            }
          }}
        />
      ) : (
        <ForgetDialog
          node={asking.node}
          onClose={() => {
            setAsking(null);
          }}
        />
      )}
    </SettingsPage>
  );
}
