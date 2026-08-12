import { invoke } from "@tauri-apps/api/core";

export interface ReminderSummary {
  id: string;
  title: string;
  due: string | null;
  completed: boolean;
}

/** Records a correction and rebuilds the router live — see commands/route.rs. */
export async function submitRouterCorrection(
  text: string,
  domain: string,
  action: string,
): Promise<void> {
  await invoke<string>("submit_router_correction", { text, domain, action });
}

/** For the "check off / delete" completion step — a tappable list with stable IDs. */
export async function listRemindersForPicker(): Promise<ReminderSummary[]> {
  return invoke<ReminderSummary[]>("list_reminders_structured", { limit: 50 });
}

/** Completes by exact identifier — no title/date re-matching, no ambiguity. */
export async function completeReminderById(id: string): Promise<string> {
  return invoke<string>("complete_reminder_by_id", { id });
}
