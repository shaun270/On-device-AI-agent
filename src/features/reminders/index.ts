/**
 * Reminders feature — frontend surface.
 *
 * Copy this folder shape for new features (alarms, music, …).
 * App.tsx owns the top-level routing decision (peek / fast-path /
 * routeMessage / generate_response — see src/router.ts); this module only
 * owns reminders' own deterministic fast path and action execution.
 */

export type { ReminderAction } from "./reminderIntent";
export {
  normalizeReminderAction,
  resolveReminderIntent,
  resolveReminderIntentFromParsed,
  tryDeterministicReminderIntent,
} from "./reminderIntent";
export {
  peekReminderEarlyClarify,
  runReminderAction,
  tryReminderFastPath,
} from "./handleMessage";
