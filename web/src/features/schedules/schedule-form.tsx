import { useId, useState } from "react";

import { useFleet, useFleetState } from "../../context";
import { nodeLabel } from "../../data/nodes";
import {
  createRequest,
  editRequest,
  maxRuns,
  nameError,
  promptError,
  type ScheduleEdit,
} from "../../data/schedules/form";
import { supportsSchedules } from "../../data/schedules/machine-schedules";
import {
  Button,
  Dialog,
  Field,
  FieldGroup,
  Select,
  Switch,
  TextArea,
  TextField,
} from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { Cadence } from "../../wire/proto";
import {
  DEFAULT_CRON,
  ScheduleCadence,
  scheduleZone,
  useCadencePreview,
} from "./schedule-cadence";
import { ScheduleIsolation } from "./schedule-isolation";
import { useIsolationChoice } from "./use-isolation-choice";
import { ScheduleTargetFields } from "./schedule-target";
import { useScheduleTarget } from "./use-schedule-target";

export function ScheduleForm({
  initialNode,
  edit,
  close,
  saved,
}: {
  initialNode: NodeKey;
  edit?: ScheduleEdit;
  close: () => void;
  saved: (node: NodeKey) => void;
}) {
  const fleet = useFleet();
  const book = useFleetState((state) => state.book);
  const connections = useFleetState((state) => state.statuses);
  const [node, setNode] = useState(initialNode);
  const connection = connections[node];
  const original = edit?.definition.target.spec;
  const [draft, setDraft] = useState({
    name: edit?.definition.name ?? "",
    prompt: edit?.definition.prompt ?? "",
    cron:
      edit?.definition.cadence.kind === "cron"
        ? edit.definition.cadence.expr
        : DEFAULT_CRON,
    maxRuns: edit?.definition.max_runs?.toString() ?? "",
    archive: edit?.definition.target.after !== "keep",
  });
  const [busy, setBusy] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const formId = useId();
  const cadence: Cadence =
    edit?.definition.cadence.kind === "every"
      ? edit.definition.cadence
      : {
          kind: "cron",
          expr: draft.cron,
          zone:
            edit?.definition.cadence.kind === "cron"
              ? edit.definition.cadence.zone
              : scheduleZone(connection),
        };
  const preview = useCadencePreview(
    node,
    connection,
    draft.cron,
    cadence.kind === "cron" ? cadence.zone : "",
    cadence.kind === "cron",
  );
  const target = useScheduleTarget(node, original);
  const targetSpec = target.spec;
  const isolation = useIsolationChoice(connection, edit, target);
  const nameProblem = nameError(draft.name);
  const promptProblem = draft.prompt === "" ? null : promptError(draft.prompt);
  const cap = maxRuns(draft.maxRuns);
  const valid =
    supportsSchedules(connection) &&
    isolation.valid &&
    nameProblem === null &&
    promptError(draft.prompt) === null &&
    cap !== "invalid" &&
    targetSpec !== null &&
    target.options.kind !== "refused" &&
    target.options.kind !== "loading" &&
    preview.kind !== "invalid" &&
    preview.kind !== "loading";

  const change = (next: Partial<typeof draft>) => {
    setSubmitError(null);
    setDraft((current) => ({ ...current, ...next }));
  };
  const save = async () => {
    if (busy || !valid) return;
    const wireCadence =
      preview.kind === "ready"
        ? preview.result.cadence
        : cadence.kind === "cron"
          ? { ...cadence, expr: cadence.expr.trim().replace(/\s+/gu, " ") }
          : cadence;
    const name = draft.name.trim();
    const spec =
      edit === undefined && name !== ""
        ? { ...targetSpec, title: name }
        : targetSpec;
    const value = {
      name,
      prompt:
        edit?.definition.prompt === draft.prompt
          ? draft.prompt
          : draft.prompt.trim(),
      cadence: wireCadence,
      spec,
      archive: draft.archive,
      ...(isolation.policy === undefined
        ? {}
        : { isolation: isolation.policy }),
      cap,
    };
    setSubmitError(null);
    setBusy(true);
    try {
      if (edit === undefined)
        await fleet.bridge.call(node, "schedule.create", createRequest(value));
      else
        await fleet.bridge.call(
          node,
          "schedule.update",
          editRequest(edit, value),
        );
      saved(node);
    } catch (error: unknown) {
      setSubmitError(
        error instanceof Error ? error.message : "Could not save schedule.",
      );
      setBusy(false);
    }
  };
  const hosts = (book ?? []).map((host) => ({
    value: host.key,
    label: nodeLabel(host, connections[host.key]),
    disabled: !supportsSchedules(connections[host.key]),
  }));

  return (
    <Dialog
      title={edit === undefined ? "New schedule" : "Edit schedule"}
      onClose={() => {
        if (!busy) close();
      }}
      footer={
        <>
          <Button disabled={busy} onClick={close}>
            Cancel
          </Button>
          <Button
            type="submit"
            form={formId}
            variant="primary"
            busy={busy}
            disabled={busy || !valid}
          >
            {edit === undefined ? "Create schedule" : "Save changes"}
          </Button>
        </>
      }
    >
      <form
        id={formId}
        className="schedule-form"
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <Field label="Name" error={nameProblem}>
          <TextField
            value={draft.name}
            onChange={(name) => {
              change({ name });
            }}
            placeholder="Optional"
            disabled={busy}
          />
        </Field>
        <Field label="Prompt" error={promptProblem}>
          <TextArea
            value={draft.prompt}
            onChange={(prompt) => {
              change({ prompt });
            }}
            placeholder="What should the agent do each run?"
            minRows={4}
            maxRows={6}
            disabled={busy}
          />
        </Field>
        {edit !== undefined || hosts.length > 1 ? (
          <FieldGroup label="Host">
            <Select
              aria-label="Host"
              value={node}
              options={hosts}
              disabled={busy || edit !== undefined}
              onChange={(next) => {
                setSubmitError(null);
                target.reset();
                isolation.reset();
                setNode(next);
              }}
            />
          </FieldGroup>
        ) : null}
        <ScheduleTargetFields target={target} original={original} busy={busy} />
        {isolation.visible ? (
          <ScheduleIsolation {...isolation.field} disabled={busy} />
        ) : null}
        {targetSpec === null ? null : (
          <Switch
            checked={draft.archive}
            onChange={(archive) => {
              change({ archive });
            }}
            label="Archive on finish"
            showLabel
            disabled={busy}
          />
        )}
        <ScheduleCadence
          cadence={cadence}
          preview={preview}
          change={(cron) => {
            change({ cron });
          }}
          disabled={busy}
        />
        <Field
          label="Max runs"
          error={
            cap === "invalid"
              ? "Enter a whole number from 1 to 1,000,000."
              : null
          }
        >
          <TextField
            value={draft.maxRuns}
            onChange={(maxRuns) => {
              change({ maxRuns });
            }}
            placeholder="Unlimited"
            disabled={busy}
          />
        </Field>
        {edit === undefined ? null : (
          <p className="schedule-form-note">
            Future runs will use your access after you save.
          </p>
        )}
        {target.options.kind === "refused" ? (
          <p className="schedule-form-error" role="alert">
            {target.options.message}
          </p>
        ) : null}
        {submitError === null ? null : (
          <p className="schedule-form-error" role="alert">
            {submitError}
          </p>
        )}
      </form>
    </Dialog>
  );
}
