// One machine in Settings › Machines: GOAT's environment row. A machine
// icon, the name (a link that opens its detail below the list, or closes
// it), one muted line (what it runs on, how it is reached, its state, its
// live sessions), the badges that set it apart, and its "…" menu.

import { Link } from "@tanstack/react-router";

import { useFleetState } from "../../context";
import { isHere, machineLine } from "../../data/machines";
import { nodeLabel } from "../../data/nodes";
import type { BookNode } from "../../wire/bridge";
import { Badge, Row } from "../../ui/kit";
import { MachineMenu, type MachineAction } from "./machine-menu";

export function MachineRow({
  node,
  selected,
  onAction,
}: {
  node: BookNode;
  selected: boolean;
  onAction: (action: MachineAction, node: BookNode) => void;
}) {
  const status = useFleetState((state) => state.statuses[node.key]);
  const sessions = useFleetState((state) => state.sessions[node.key]);
  const label = nodeLabel(node, status);
  const here = isHere(node.key);
  return (
    <Row
      selected={selected}
      mark={{ icon: "server" }}
      title={
        <Link
          to="/settings/$section"
          params={{ section: "machines" }}
          search={selected ? {} : { machine: node.key }}
          aria-current={selected ? "true" : undefined}
          aria-label={`${label}${selected ? ", open" : ""}`}
        >
          {label}
        </Link>
      }
      detail={{ facts: machineLine(node, status, sessions) }}
      control={
        <>
          {here ? <Badge>This machine</Badge> : null}
          {node.default ? <Badge look="accent">Default</Badge> : null}
        </>
      }
      menu={
        <MachineMenu
          node={node}
          label={label}
          onAction={(action) => {
            onAction(action, node);
          }}
        />
      }
    />
  );
}
