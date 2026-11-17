// Asking before a label is deleted, with how many workspaces carry it
// (Paseo's "Delete {name}?" with its count, the node's inspection).

import { Button, Dialog } from "../../ui/kit";

export function deleteMessage(count: number): string {
  return count === 1
    ? "This removes the label from 1 workspace on this machine."
    : `This removes the label from ${String(count)} workspaces on this machine.`;
}

export function DeleteLabelDialog({
  name,
  count,
  onAnswer,
}: {
  name: string;
  count: number;
  onAnswer: (yes: boolean) => void;
}) {
  return (
    <Dialog
      title={`Delete ${name}?`}
      onClose={() => {
        onAnswer(false);
      }}
      footer={
        <>
          <Button
            variant="ghost"
            onClick={() => {
              onAnswer(false);
            }}
          >
            Cancel
          </Button>
          <Button
            variant="danger-solid"
            onClick={() => {
              onAnswer(true);
            }}
          >
            Delete
          </Button>
        </>
      }
    >
      <p className="label-note">{deleteMessage(count)}</p>
    </Dialog>
  );
}
