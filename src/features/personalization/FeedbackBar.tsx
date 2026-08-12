/**
 * Feedback bar — shown under every assistant reply. 👎 opens a plain-language
 * picker (domain → action → completion) that both fixes the user's task
 * right now and teaches the router for next time (see commands/route.rs's
 * submit_router_correction). See PERSONALIZATION_CATALOG for the copy.
 */

import { useState } from "react";
import { PERSONALIZATION_CATALOG, type ActionOption, type DomainOption } from "./catalog";
import { completeReminderById, listRemindersForPicker, submitRouterCorrection, type ReminderSummary } from "./api";
import { executeForcedAction } from "../../router";
import { runReminderAction, resolveReminderIntentFromParsed } from "../reminders";

type Step =
  | { kind: "idle" }
  | { kind: "thanked" }
  | { kind: "domain" }
  | { kind: "action"; domain: DomainOption }
  | { kind: "text"; domain: DomainOption; action: ActionOption }
  | { kind: "reminder-picker"; domain: DomainOption; action: ActionOption }
  | { kind: "done"; message: string }
  | { kind: "error"; message: string };

interface Props {
  precedingUserText: string;
  onCorrectionExecuted: (resultText: string) => void;
}

export function FeedbackBar({ precedingUserText, onCorrectionExecuted }: Props) {
  const [step, setStep] = useState<Step>({ kind: "idle" });

  if (step.kind === "idle") {
    return (
      <div className="feedback-bar">
        <span className="feedback-prompt">Help us personalize — was this right?</span>
        <button className="feedback-btn" onClick={() => setStep({ kind: "thanked" })} aria-label="Correct">
          👍
        </button>
        <button className="feedback-btn" onClick={() => setStep({ kind: "domain" })} aria-label="Incorrect">
          👎
        </button>
      </div>
    );
  }

  if (step.kind === "thanked") {
    return <div className="feedback-bar feedback-bar--done">Thanks!</div>;
  }

  if (step.kind === "done") {
    return <div className="feedback-bar feedback-bar--done">Got it — fixed, and I'll route that better next time.</div>;
  }

  if (step.kind === "error") {
    return <div className="feedback-bar feedback-bar--error">{step.message}</div>;
  }

  if (step.kind === "domain") {
    return (
      <div className="feedback-bar feedback-bar--picker">
        <span className="feedback-prompt">What should this have been?</span>
        <div className="feedback-options">
          {PERSONALIZATION_CATALOG.map((d) => (
            <button key={d.domain} className="feedback-option" onClick={() => setStep({ kind: "action", domain: d })}>
              {d.label}
            </button>
          ))}
        </div>
      </div>
    );
  }

  if (step.kind === "action") {
    return (
      <div className="feedback-bar feedback-bar--picker">
        <span className="feedback-prompt">{step.domain.label} — do what?</span>
        <div className="feedback-options">
          {step.domain.actions.map((a) => (
            <button
              key={a.action}
              className="feedback-option"
              onClick={() =>
                setStep(
                  a.completion === "text"
                    ? { kind: "text", domain: step.domain, action: a }
                    : { kind: "reminder-picker", domain: step.domain, action: a },
                )
              }
            >
              {a.label}
            </button>
          ))}
        </div>
      </div>
    );
  }

  if (step.kind === "text") {
    return (
      <TextCompletion
        domain={step.domain}
        action={step.action}
        precedingUserText={precedingUserText}
        onDone={(msg) => {
          setStep({ kind: "done", message: msg });
          onCorrectionExecuted(msg);
        }}
        onError={(msg) => setStep({ kind: "error", message: msg })}
      />
    );
  }

  // reminder-picker
  return (
    <ReminderPickerCompletion
      precedingUserText={precedingUserText}
      onDone={(msg) => {
        setStep({ kind: "done", message: msg });
        onCorrectionExecuted(msg);
      }}
      onError={(msg) => setStep({ kind: "error", message: msg })}
    />
  );
}

function TextCompletion({
  domain,
  action,
  precedingUserText,
  onDone,
  onError,
}: {
  domain: DomainOption;
  action: ActionOption;
  precedingUserText: string;
  onDone: (message: string) => void;
  onError: (message: string) => void;
}) {
  const [value, setValue] = useState("");
  const [submitting, setSubmitting] = useState(false);

  async function submit() {
    const text = value.trim();
    if (!text || submitting) return;
    setSubmitting(true);
    try {
      await submitRouterCorrection(precedingUserText, domain.domain, action.action);

      const outcome = await executeForcedAction(domain.domain, action.action, text);
      if (outcome.kind === "reply") {
        onDone(outcome.message);
        return;
      }
      if (outcome.kind === "reminder") {
        const parsedAction = resolveReminderIntentFromParsed(text, outcome.parsed);
        if (parsedAction) {
          const reply = await runReminderAction(parsedAction);
          onDone(reply);
          return;
        }
      }
      onError("Saved the correction, but couldn't carry out the action from that phrasing — try rewording it.");
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="feedback-bar feedback-bar--picker">
      <span className="feedback-prompt">{action.label} — what should it say?</span>
      <div className="feedback-text-row">
        <input
          className="feedback-text-input"
          value={value}
          placeholder={action.placeholder}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") submit();
          }}
          disabled={submitting}
          autoFocus
        />
        <button className="feedback-option" onClick={submit} disabled={submitting || !value.trim()}>
          {submitting ? "…" : "Go"}
        </button>
      </div>
    </div>
  );
}

function ReminderPickerCompletion({
  precedingUserText,
  onDone,
  onError,
}: {
  precedingUserText: string;
  onDone: (message: string) => void;
  onError: (message: string) => void;
}) {
  const [items, setItems] = useState<ReminderSummary[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);

  if (items === null && loadError === null) {
    listRemindersForPicker()
      .then(setItems)
      .catch((e) => setLoadError(e instanceof Error ? e.message : String(e)));
    return <div className="feedback-bar feedback-bar--picker">Loading your reminders…</div>;
  }

  if (loadError) {
    return <div className="feedback-bar feedback-bar--error">{loadError}</div>;
  }

  async function pick(item: ReminderSummary) {
    if (picking) return;
    setPicking(true);
    try {
      await submitRouterCorrection(precedingUserText, "reminders", "complete");
      const result = await completeReminderById(item.id);
      onDone(result);
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    } finally {
      setPicking(false);
    }
  }

  if (!items || items.length === 0) {
    return <div className="feedback-bar feedback-bar--done">No open reminders to pick from.</div>;
  }

  return (
    <div className="feedback-bar feedback-bar--picker">
      <span className="feedback-prompt">Which one?</span>
      <div className="feedback-options feedback-options--column">
        {items.map((item) => (
          <button key={item.id} className="feedback-option" onClick={() => pick(item)} disabled={picking}>
            {item.title}
            {item.due ? ` (due ${item.due})` : ""}
          </button>
        ))}
      </div>
    </div>
  );
}
