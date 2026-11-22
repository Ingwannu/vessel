import { useEffect, useState } from "react";

import { useFleet } from "../../context";
import { promptTextError } from "../../data/prompt-text";
import {
  DEFAULT_RESUME_PROMPT,
  readRestartRecovery,
  saveRestartRecovery,
} from "../../data/restart-recovery";
import {
  Alert,
  Button,
  Dialog,
  IconButton,
  Popover,
  Row,
  RowList,
  Switch,
  TextArea,
} from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { RestartRecovery, SetRestartRecovery } from "../../wire/proto";

const TITLE = "Resume agents after restart";

/** Machine details key the owning Agents section by the connected machine. */
export function useRestartRecovery(machine: NodeKey) {
  const fleet = useFleet();
  const [policy, setPolicy] = useState<RestartRecovery | null>(null);

  useEffect(() => {
    let current = true;
    void readRestartRecovery(fleet, machine).then((result) => {
      if (current) setPolicy(result.ok ? result.value : null);
    });
    return () => {
      current = false;
    };
  }, [fleet, machine]);

  return { policy, setPolicy };
}

export function RestartRecoveryCard({
  machine,
  policy,
  setPolicy,
}: {
  machine: NodeKey;
  policy: RestartRecovery;
  setPolicy: (policy: RestartRecovery) => void;
}) {
  const fleet = useFleet();
  const [draft, setDraft] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);

  const save = async (change: SetRestartRecovery) => {
    if (saving) return;
    setSaving(true);
    setFailure(null);
    const outcome = await saveRestartRecovery(fleet, machine, change);
    if (outcome.ok) {
      setPolicy(outcome.value);
      setDraft(null);
    } else {
      setFailure(outcome.problem.message);
      // A saved Off can report a cleanup failure. Read back the confirmed
      // state while retaining the draft and the original error.
      const confirmed = await readRestartRecovery(fleet, machine);
      if (confirmed.ok) setPolicy(confirmed.value);
    }
    setSaving(false);
  };

  const close = () => {
    if (!saving) {
      setDraft(null);
      setFailure(null);
    }
  };
  const invalid = draft === null ? null : promptTextError(draft);
  const canSave =
    draft !== null &&
    draft.trim() !== "" &&
    draft !== policy.prompt &&
    invalid === null &&
    !saving;
  const saveError =
    failure === null ? null : (
      <Alert variant="error" size="sm" description={failure} />
    );

  return (
    <>
      <RowList label={TITLE}>
        <Row
          title={
            <>
              {TITLE}{" "}
              <Popover
                title={TITLE}
                showTitle={false}
                trigger={
                  <IconButton
                    icon="info"
                    label="About restart recovery"
                    size="xs"
                  />
                }
              >
                Reopen agents that were working when the host stopped and send
                the resume prompt. Applies after graceful restarts and crashes.
              </Popover>
            </>
          }
          detail={{ description: "Continue interrupted work automatically." }}
          control={
            <>
              <Button
                variant="outline"
                size="sm"
                aria-label="Edit prompt"
                disabled={!policy.enabled || saving}
                onClick={() => {
                  setFailure(null);
                  setDraft(policy.prompt);
                }}
              >
                Edit
              </Button>
              <Switch
                label={TITLE}
                checked={policy.enabled}
                disabled={saving}
                onChange={(enabled) => {
                  void save({ enabled });
                }}
              />
            </>
          }
        />
      </RowList>
      {draft === null ? (
        saveError
      ) : (
        <Dialog
          title="Resume prompt"
          onClose={close}
          footer={
            <>
              <Button variant="ghost" disabled={saving} onClick={close}>
                Cancel
              </Button>
              <Button
                variant="primary"
                busy={saving}
                disabled={!canSave}
                onClick={() => {
                  if (canSave) void save({ prompt: draft });
                }}
              >
                Save
              </Button>
            </>
          }
        >
          <TextArea
            aria-label="Resume prompt"
            value={draft}
            onChange={setDraft}
            placeholder={DEFAULT_RESUME_PROMPT}
            minRows={6}
            maxRows={10}
            readOnly={saving}
          />
          {invalid === null ? null : (
            <Alert variant="error" size="sm" description={invalid} />
          )}
          {saveError}
        </Dialog>
      )}
    </>
  );
}
