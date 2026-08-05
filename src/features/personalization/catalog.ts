/**
 * Plain-language domain/action catalog for the feedback-bar picker — no
 * one using the app knows "domain: reminders, action: complete", so this
 * is the one place that translates real router labels into copy a user
 * would actually recognize.
 *
 * Keep this in sync with `src-tauri/src/capabilities/*\/exemplars.toml`'s
 * action set by hand for now — small enough (2 domains, a handful of
 * actions each) that a generated single-source-of-truth isn't worth the
 * plumbing yet. Revisit once a third domain (e.g. alarms) ships.
 */

export type CompletionKind = "text" | "reminder-picker";

export interface ActionOption {
  /** Router action label, e.g. "set" — sent to the backend as-is. */
  action: string;
  /** What the user sees. */
  label: string;
  /** Placeholder shown for text-entry actions. */
  placeholder?: string;
  completion: CompletionKind;
}

export interface DomainOption {
  /** Router domain label, e.g. "reminders" — sent to the backend as-is. */
  domain: string;
  /** What the user sees. */
  label: string;
  actions: ActionOption[];
}

export const PERSONALIZATION_CATALOG: DomainOption[] = [
  {
    domain: "reminders",
    label: "Reminders",
    actions: [
      { action: "set", label: "Set a reminder", placeholder: "e.g. remind me to call the dentist tomorrow", completion: "text" },
      { action: "set_many", label: "Set several reminders", placeholder: "e.g. remind me every day this week to stretch", completion: "text" },
      { action: "list", label: "List my reminders", placeholder: "e.g. what's on my list today", completion: "text" },
      // Delete isn't supported yet — folded into "complete" so it maps to a
      // real, executable action instead of a phantom one. See system.rs.
      { action: "complete", label: "Check off (or delete) a reminder", completion: "reminder-picker" },
    ],
  },
  {
    domain: "files",
    label: "Files",
    actions: [
      { action: "search", label: "Find a file", placeholder: "e.g. find my resume", completion: "text" },
      { action: "read", label: "Read a file", placeholder: "e.g. what does my cover letter say", completion: "text" },
      { action: "write", label: "Write or edit a file", placeholder: "e.g. save this as draft.txt on my desktop", completion: "text" },
    ],
  },
];
