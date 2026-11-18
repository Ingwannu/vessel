// A question the agent asks, over the composer (agent-orchestrator's
// `ElicitationDock`): "Question · 1 of 3", what is asked, the step's
// answers drawn by kind, then Back, the steps as dots, Skip and Next (or
// Submit answer on the last). One question at a time, oldest first. The
// page checks what the node would refuse before it sends; a refusal that
// still comes is shown at the field it names. The dock goes when the
// node logs the question settled (here or on any other device).

import "./question-dock.css";

import { useState } from "react";

import { useFleet } from "../../context";
import { answerQuestion } from "../../data/conversation-calls";
import { errorCode } from "../../data/outcome";
import {
  answerable,
  answerOf,
  type Draft,
  type Drafts,
  refusedField,
  startingDrafts,
  stepProblem,
  stepsOf,
} from "../../data/question-form";
import { Button, IconButton } from "../../ui/kit";
import { toast } from "../../ui/toasts";
import type { NodeKey } from "../../wire/bridge";
import type { FieldName, Question, SessionId } from "../../wire/proto";
import { QuestionStep } from "./question-field";

export function QuestionDock({
  node,
  session,
  question,
  waiting,
}: {
  node: NodeKey;
  session: SessionId;
  question: Question;
  /** How many more questions wait after this one. */
  waiting: number;
}) {
  const fleet = useFleet();
  const steps = stepsOf(question);
  const [index, setIndex] = useState(0);
  const [drafts, setDrafts] = useState<Drafts>(() => startingDrafts(question));
  const [problem, setProblem] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [folded, setFolded] = useState(false);
  const step = steps[index];
  const last = index >= steps.length - 1;
  const prompt =
    step?.field.description ?? step?.field.title ?? question.message;
  const setDraft = (name: FieldName, draft: Draft) => {
    setProblem(null);
    setDrafts((current) => ({ ...current, [name]: draft }));
  };
  const send = (answer: "accept" | "decline") => {
    setSending(true);
    void answerQuestion(
      fleet,
      node,
      session,
      question.id,
      answer === "accept" ? answerOf(question, drafts) : { answer: "decline" },
    ).then((outcome) => {
      setSending(false);
      if (outcome.ok) {
        return;
      }
      const code = errorCode(outcome.problem);
      if (code === "invalid_request") {
        const field = refusedField(question, outcome.problem.message);
        const at = steps.findIndex(
          (candidate) =>
            candidate.field.name === field || candidate.other?.name === field,
        );
        if (at >= 0) {
          setIndex(at);
        }
        setProblem(outcome.problem.message);
      } else if (code === "conflict" || code === "not_found") {
        toast("This question was already answered elsewhere.", "default");
      } else {
        toast(outcome.problem.message, "error");
      }
    });
  };
  const next = () => {
    if (step === undefined || sending) {
      return;
    }
    const wrong = stepProblem(step, drafts);
    if (wrong !== null) {
      setProblem(wrong);
      return;
    }
    if (last) {
      send("accept");
    } else {
      setIndex(index + 1);
    }
  };
  return (
    <section
      className="question-dock"
      aria-label="Question from the agent"
      aria-live="polite"
      onKeyDown={(event) => {
        if (event.key === "Enter" && !event.shiftKey) {
          event.preventDefault();
          next();
        }
      }}
    >
      <header className="question-head">
        <span className="question-count">
          Question · {index + 1} of {Math.max(1, steps.length)}
          {waiting > 0 ? ` · ${String(waiting)} more waiting` : ""}
        </span>
        <span
          className="question-chevron"
          data-folded={folded ? "" : undefined}
        >
          <IconButton
            label={folded ? "Expand question" : "Collapse question"}
            icon="chevron-down"
            size="xs"
            aria-expanded={!folded}
            onClick={() => {
              setFolded(!folded);
            }}
          />
        </span>
      </header>
      <div className="question-ask">
        {prompt === question.message ? null : (
          <p className="question-message">{question.message}</p>
        )}
        <p className="question-prompt">{prompt}</p>
      </div>
      {folded || step === undefined ? null : (
        <QuestionStep
          step={step}
          drafts={drafts}
          prompt={prompt}
          setDraft={setDraft}
        />
      )}
      {problem === null ? null : (
        <p className="question-problem" role="alert">
          {problem}
        </p>
      )}
      <footer className="question-foot">
        {index > 0 ? (
          <Button
            size="xs"
            variant="ghost"
            icon="back"
            onClick={() => {
              setProblem(null);
              setIndex(index - 1);
            }}
          >
            Back
          </Button>
        ) : null}
        {steps.length > 1 ? (
          <span className="question-dots" aria-hidden="true">
            {steps.map((candidate, at) => (
              <span
                key={candidate.field.name}
                className="question-dot"
                data-state={
                  at === index ? "current" : at < index ? "done" : undefined
                }
              />
            ))}
          </span>
        ) : null}
        <span className="question-foot-end">
          <Button
            size="xs"
            variant="ghost"
            disabled={sending}
            onClick={() => {
              send("decline");
            }}
          >
            Skip
          </Button>
          <Button
            size="xs"
            variant="primary"
            shape="round"
            busy={sending}
            disabled={!answerable(question)}
            onClick={next}
          >
            {last ? "Submit answer" : "Next"}
          </Button>
        </span>
      </footer>
    </section>
  );
}
