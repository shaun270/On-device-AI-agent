/**
 * Reminders feature — frontend surface.
 *
 * Copy this folder shape for new features (alarms, music, …).
 * App.tsx should only call peek / tryHandle / generate_response.
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
  tryHandleReminderMessage,
} from "./handleMessage";
