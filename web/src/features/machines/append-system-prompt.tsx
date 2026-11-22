import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import {
  readAppendSystemPrompt,
  saveAppendSystemPrompt,
} from "../../data/host-agent-settings";
import { promptTextError } from "../../data/prompt-text";
import { Alert, Button, Dialog, Row, Skeleton, TextArea } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";

export function AppendSystemPromptRow({ machine }: { machine: NodeKey }) {
  const [editing, setEditing] = useState(false);
  return (
    <>
      <Row
        title="System prompt"
        detail={{
          description:
            "Adds a system prompt to Codex app-server launches. ACP is unsupported.",
        }}
        control={
          <Button
            size="sm"
            onClick={() => {
              setEditing(true);
            }}
          >
            Edit
          </Button>
        }
      />
      {editing ? (
        <AppendPromptDialog
          machine={machine}
          close={() => {
            setEditing(false);
          }}
        />
      ) : null}
    </>
  );
}

type PromptDraft =
  | { kind: "loading" }
  | { kind: "failed"; message: string }
  | { kind: "ready"; kept: string; draft: string };

function AppendPromptDialog({
  machine,
  close,
}: {
  machine: NodeKey;
  close: () => void;
}) {
  const fleet = useFleet();
  const [value, setValue] = useState<PromptDraft>({ kind: "loading" });
  const [saving, setSaving] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void readAppendSystemPrompt(fleet, machine).then((result) => {
      if (current) {
        setValue(
          result.ok
            ? { kind: "ready", kept: result.value, draft: result.value }
            : { kind: "failed", message: result.problem.message },
        );
      }
    });
    return () => {
      current = false;
    };
  }, [fleet, machine]);

  const changed = value.kind === "ready" && value.kept !== value.draft;
  const invalid = value.kind === "ready" ? promptTextError(value.draft) : null;
  const save = async () => {
    if (value.kind !== "ready" || !changed || invalid !== null || saving)
      return;
    setSaving(true);
    setFailure(null);
    const outcome = await saveAppendSystemPrompt(fleet, machine, value.draft);
    if (outcome.ok) {
      close();
    } else {
      setFailure(outcome.problem.message);
      setSaving(false);
    }
  };
  const update = (draft: string) => {
    if (value.kind === "ready") {
      setValue({ ...value, draft });
      setFailure(null);
    }
  };
  return (
    <Dialog
      title="Append system prompt"
      onClose={() => {
        if (!saving) close();
      }}
      footer={
        <>
          <Button
            variant="ghost"
            disabled={!changed || saving}
            onClick={() => {
              if (value.kind === "ready") update(value.kept);
            }}
          >
            Reset
          </Button>
          <Button
            variant="primary"
            busy={saving}
            disabled={!changed || invalid !== null || saving}
            onClick={() => {
              void save();
            }}
          >
            Save
          </Button>
        </>
      }
    >
      {value.kind === "loading" ? (
        <Skeleton />
      ) : value.kind === "failed" ? (
        <Alert
          variant="error"
          title="Unable to read the system prompt"
          description={value.message}
        />
      ) : (
        <TextArea
          aria-label="Append system prompt"
          value={value.draft}
          onChange={update}
          placeholder="Always keep replies concise."
          minRows={6}
          maxRows={10}
          readOnly={saving}
        />
      )}
      {invalid === null ? null : (
        <Alert variant="error" size="sm" description={invalid} />
      )}
      {failure === null ? null : (
        <Alert
          variant="error"
          size="sm"
          title="Unable to save the system prompt"
          description={failure}
        />
      )}
    </Dialog>
  );
}
