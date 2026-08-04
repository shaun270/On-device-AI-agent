import { invoke } from "@tauri-apps/api/core";

import { tryDeterministicReminderIntent, type ReminderAction } from "./reminderIntent";

export async function runReminderAction(action: ReminderAction): Promise<string> {
  if (action.kind === "clarify") return action.message;
  if (action.kind === "list") {
    return invoke<string>("list_reminders", {
      range: action.range ?? null,
      days_ahead: action.days_ahead ?? null,
      start: action.start ?? null,
      end: action.end ?? null,
      search: action.search ?? null,
      limit: 25,
    });
  }
  if (action.kind === "complete") {
    return invoke<string>("complete_reminder", {
      title: action.title,
      match_mode: action.match_mode,
    });
  }
  if (action.kind === "set_many") {
    const lines: string[] = [];
    for (const item of action.items) {
      lines.push(
        await invoke<string>("set_reminder", {
          title: item.title,
          due: item.due ?? null,
          list_name: item.list_name ?? null,
        }),
      );
    }
    return lines.join("\n");
  }
  return invoke<string>("set_reminder", {
    title: action.title,
    due: action.due ?? null,
    list_name: action.list_name ?? null,
  });
}

/** Instant clarifies that should skip the thinking spinner. */
export function peekReminderEarlyClarify(trimmed: string): string | null {
  if (/^\/done\b/i.test(trimmed) && !trimmed.replace(/^\/done\b/i, "").trim()) {
    return "Which reminder should I check off? e.g. /done Buy milk";
  }
  if (/^\/remind\b/i.test(trimmed) && !trimmed.replace(/^\/remind\b/i, "").trim()) {
    return "What should I remind you about? e.g. /remind Buy milk";
  }
  return null;
}

/**
 * Slash commands + zero-LLM deterministic reminders shortcuts — no router
 * or LLM call happens here. Returns a reply, or null when the caller
 * should try the cross-capability router next (see `src/router.ts`).
 */
export async function tryReminderFastPath(trimmed: string): Promise<string | null> {
  const early = peekReminderEarlyClarify(trimmed);
  if (early) return early;

  const listMatch = trimmed.match(
    /^\/reminders(?:\s+(all|today|week|\d+|search\s+.+))?$/i,
  );
  if (listMatch) {
    const arg = (listMatch[1] || "all").trim();
    if (/^\d+$/.test(arg)) {
      return invoke<string>("list_reminders", {
        days_ahead: Number(arg),
        limit: 25,
      });
    }
    if (/^search\s+/i.test(arg)) {
      return invoke<string>("list_reminders", {
        search: arg.replace(/^search\s+/i, "").trim(),
        limit: 25,
      });
    }
    return invoke<string>("list_reminders", {
      range: arg.toLowerCase(),
      limit: 25,
    });
  }

  if (/^\/done\b/i.test(trimmed)) {
    const rest = trimmed.replace(/^\/done\b/i, "").trim();
    const [titlePart, matchPart] = rest.split("|").map((s) => s.trim());
    return invoke<string>("complete_reminder", {
      title: titlePart,
      match_mode: matchPart || "exact",
    });
  }

  if (/^\/remind\b/i.test(trimmed)) {
    const rest = trimmed.replace(/^\/remind\b/i, "").trim();
    const [titlePart, timePart] = rest.split("|").map((s) => s.trim());
    return invoke<string>("set_reminder", {
      title: titlePart,
      due: timePart || null,
    });
  }

  const det = tryDeterministicReminderIntent(trimmed);
  if (det) return runReminderAction(det);
  return null;
}
