/**
 * Automated reminder-intent suite.
 *
 * Simulates adversarial / typical LLM JSON (the failure modes we've seen),
 * then runs tryDeterministic + normalize — the same stack the app uses after
 * classify_intent returns.
 *
 * Run: npm run test:reminders
 */

import {
  resolveReminderIntentFromParsed,
  type ReminderAction,
} from "../src/features/reminders/reminderIntent";

type Expect = {
  kind: ReminderAction["kind"] | "chat";
  titleIncludes?: string[];
  titleEquals?: string;
  titleExcludes?: string[];
  dueApproxMinutesFromNow?: number;
  dueToleranceMinutes?: number;
  dueDateOffsetDays?: number;
  dueLocalTime?: string; // HH:MM
  dueIsoPrefix?: string;
  itemCount?: number;
  range?: string;
  messageIncludes?: string[];
};

type Case = {
  id: string;
  prompt: string;
  /** What a messy/typical router model often emits before normalize. */
  llm: Record<string, unknown> | null;
  expect: Expect;
};

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function addDays(base: Date, n: number): Date {
  const d = new Date(base);
  d.setDate(d.getDate() + n);
  return d;
}

function isoDate(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

function isoLocal(d: Date, time?: { h: number; m: number }): string {
  const date = isoDate(d);
  if (!time) return date;
  return `${date}T${pad2(time.h)}:${pad2(time.m)}`;
}

function dueFromTz(day: Date, h: number, m: number, offsetMin: number): string {
  const utcMs = Date.UTC(day.getFullYear(), day.getMonth(), day.getDate(), h, m) - offsetMin * 60_000;
  const local = new Date(utcMs);
  return isoLocal(local, { h: local.getHours(), m: local.getMinutes() });
}

const tomorrow = () => addDays(new Date(), 1);
const today = () => new Date();

const CASES: Case[] = [
  // A — basic create
  {
    id: "A1",
    prompt: "remind me to buy milk",
    llm: { kind: "set", title: "buy milk" },
    expect: { kind: "set", titleIncludes: ["buy milk"] },
  },
  {
    id: "A2",
    prompt: "remind me to pray tomorrow 9 am",
    llm: { kind: "set", title: "pray tomorrow 9 am", due: "tomorrow" },
    expect: {
      kind: "set",
      titleEquals: "pray",
      dueDateOffsetDays: 1,
      dueLocalTime: "09:00",
    },
  },
  {
    id: "A3",
    prompt: "create a reminder to call mom today at 3pm",
    llm: { kind: "set", title: "call mom today at 3pm", due: "today" },
    expect: {
      kind: "set",
      titleIncludes: ["call mom"],
      dueDateOffsetDays: 0,
      dueLocalTime: "15:00",
    },
  },
  {
    id: "A4",
    prompt: "set a reminder for dentist on August 12",
    llm: { kind: "set", title: "dentist on August 12", due: "August 12" },
    expect: {
      kind: "set",
      titleIncludes: ["dentist"],
      dueIsoPrefix: `${new Date().getFullYear()}-08-12`,
    },
  },
  {
    id: "A5",
    prompt: "add a reminder to submit taxes on 10th august this year",
    llm: { kind: "set", title: "submit taxes on 10th august this year" },
    expect: {
      kind: "set",
      titleIncludes: ["submit taxes"],
      dueIsoPrefix: `${new Date().getFullYear()}-08-10`,
    },
  },
  {
    id: "A6",
    prompt: "remind me to go home around 10th august",
    llm: { kind: "set", title: "go home around 10th august" },
    expect: {
      kind: "set",
      titleEquals: "go home",
      dueIsoPrefix: `${new Date().getFullYear()}-08-10`,
    },
  },

  // B — relative
  {
    id: "B1",
    prompt: "remind me to go to the gym in 2 hrs",
    llm: { kind: "set", title: "go gym", due: "02:00" },
    expect: {
      kind: "set",
      titleIncludes: ["gym"],
      titleExcludes: ["2 hrs", "02:00"],
      dueApproxMinutesFromNow: 120,
      dueToleranceMinutes: 3,
    },
  },
  {
    id: "B2",
    prompt: "remind me to take medicine in 30 minutes",
    llm: { kind: "set", title: "take medicine in 30 minutes", due: "now" },
    expect: {
      kind: "set",
      titleIncludes: ["medicine"],
      dueApproxMinutesFromNow: 30,
      dueToleranceMinutes: 3,
    },
  },
  {
    id: "B3",
    prompt: "create a reminder in 8 days from today to complete my OA",
    llm: { kind: "set_many", title: "complete my OA", days: 8 },
    expect: {
      kind: "set",
      titleIncludes: ["complete my OA"],
      dueDateOffsetDays: 8,
    },
  },
  {
    id: "B4",
    prompt: "remind me to follow up in 3 days",
    llm: { kind: "set", title: "follow up in 3 days" },
    expect: { kind: "set", titleIncludes: ["follow up"], dueDateOffsetDays: 3 },
  },
  {
    id: "B5",
    prompt: "create a reminder after 1 day to email John",
    llm: { kind: "set", title: "email John after 1 day" },
    expect: { kind: "set", titleIncludes: ["email John"], dueDateOffsetDays: 1 },
  },

  // C — set_many vs list
  {
    id: "C1",
    prompt: "create a reminder for next 3 days to pray at 9 am",
    llm: { kind: "set_many", title: "pray", days: 3, time: "9 am" },
    expect: { kind: "set_many", itemCount: 3, titleEquals: "pray", dueLocalTime: "09:00" },
  },
  {
    id: "C2",
    prompt: "create a reminder for next 2 days to stretch at 7am",
    llm: { kind: "set", title: "stretch", days: 2, time: "7am" },
    expect: { kind: "set_many", itemCount: 2, titleIncludes: ["stretch"], dueLocalTime: "07:00" },
  },
  {
    id: "C3",
    prompt: "what do i have to do for the whole of next week",
    llm: { kind: "set_many", title: "next week", days: 7 },
    expect: { kind: "list", range: "week" },
  },
  {
    id: "C4",
    prompt: "show me the list of reminders",
    llm: { kind: "set", title: "list of reminders" },
    expect: { kind: "list", range: "all" },
  },
  {
    id: "C5",
    prompt: "check my reminders",
    llm: { kind: "chat" },
    expect: { kind: "list", range: "all" },
  },
  {
    id: "C6",
    prompt: "reminders for tomorrow",
    llm: { kind: "list", range: "all" },
    expect: { kind: "list" },
  },
  {
    id: "C7",
    prompt: "what's due today",
    llm: { kind: "set", title: "due today" },
    expect: { kind: "list", range: "today" },
  },
  {
    // Regression: looksLikeListQuery's creation-verb bailout didn't include
    // "make", so "make 2 reminders that i HAVE to..." tripped the generic
    // have+reminders list heuristic and silently redirected every creation
    // through this phrasing into a list query (found in live testing).
    id: "C8",
    prompt: "make 2 reminders that i have to play football, 1 for tomorrow 9pm, the other for day after 3 pm",
    llm: {
      kind: "set_many",
      items: [
        { title: "play football", due: `${isoDate(tomorrow())}T21:00` },
        { title: "play football", due: `${isoDate(addDays(new Date(), 2))}T15:00` },
      ],
    },
    expect: { kind: "set_many", itemCount: 2 },
  },
  {
    // Regression: parsed.items (distinct one-off reminders, each own due)
    // was unreachable — the top-level parsed.title hollow-check ran first
    // and always redirected to list, since items-shape has no top-level title.
    id: "C9",
    prompt: "add 2 reminders, one to call mom tomorrow at 5pm and one to email John the day after at noon",
    llm: {
      kind: "set_many",
      items: [
        { title: "call mom", due: `${isoDate(tomorrow())}T17:00` },
        { title: "email John", due: `${isoDate(addDays(new Date(), 2))}T12:00` },
      ],
    },
    expect: { kind: "set_many", itemCount: 2 },
  },

  // D — timezones
  {
    id: "D1",
    prompt: "remind me to call my dad at 5 pm IST tomorrow",
    llm: { kind: "set", title: "call my dad at 5 pm IST tomorrow", due: `${isoDate(tomorrow())}T17:00` },
    expect: {
      kind: "set",
      titleEquals: "call my dad",
      titleExcludes: ["IST", "ist"],
      dueIsoPrefix: dueFromTz(tomorrow(), 17, 0, 330).slice(0, 16),
    },
  },
  {
    id: "D2",
    prompt: "create a reminder for me to call my dad who is in india at 5 pm in india",
    llm: { kind: "set", title: "call my dad who is in india at 5 pm in india", due: "17:00" },
    expect: {
      kind: "set",
      titleIncludes: ["call my dad"],
      titleExcludes: ["india", "IST"],
      dueIsoPrefix: dueFromTz(today(), 17, 0, 330).slice(0, 16),
    },
  },
  {
    id: "D3",
    prompt: "remind me to join the call at 9 am EST tomorrow",
    llm: { kind: "set", title: "join the call at 9 am EST tomorrow" },
    expect: {
      kind: "set",
      titleIncludes: ["join the call"],
      titleExcludes: ["EST"],
      dueIsoPrefix: dueFromTz(tomorrow(), 9, 0, -300).slice(0, 16),
    },
  },
  {
    id: "D4",
    prompt: "remind me at 10 pm PST tomorrow to lock the door",
    llm: { kind: "set", title: "lock the door at 10 pm PST tomorrow" },
    expect: {
      kind: "set",
      titleIncludes: ["lock the door"],
      titleExcludes: ["PST"],
      dueIsoPrefix: dueFromTz(tomorrow(), 22, 0, -480).slice(0, 16),
    },
  },

  // E — complete
  {
    id: "E1",
    prompt: "check off buy eggs",
    llm: { kind: "complete", title: "buy eggs", match_mode: "contains" },
    expect: { kind: "complete", titleIncludes: ["buy eggs"] },
  },
  {
    id: "E2",
    prompt: "check off the get eggs reminder",
    llm: { kind: "complete", title: "the get eggs reminder" },
    expect: { kind: "complete", titleIncludes: ["get eggs"] },
  },
  {
    id: "E3",
    prompt: "Check off the go and get eggs one",
    llm: { kind: "complete", title: "the go and get eggs one" },
    expect: { kind: "complete", titleIncludes: ["get eggs"] },
  },
  {
    id: "E4",
    prompt: "check off go buy eggs in reminders",
    llm: { kind: "complete", title: "go buy eggs in reminders" },
    expect: { kind: "complete", titleIncludes: ["buy eggs"], titleExcludes: ["reminders"] },
  },
  {
    id: "E5",
    prompt: "mark buy milk as done",
    llm: { kind: "complete", title: "buy milk" },
    expect: { kind: "complete", titleIncludes: ["buy milk"] },
  },
  {
    id: "E6",
    prompt: "check off somethingthatdoesnotexist",
    llm: { kind: "complete", title: "somethingthatdoesnotexist" },
    expect: { kind: "complete", titleIncludes: ["somethingthatdoesnotexist"] },
  },
  {
    // Real bug: "off" trailing well after "check" (not fused as "check off")
    // was matching the generic "check" + "reminder" list-query heuristic,
    // overriding the router's correct "complete" decision back to "list".
    id: "E7",
    prompt: "no i mean check the sleep reminder off",
    llm: { kind: "complete", title: "sleep", match_mode: "contains" },
    expect: { kind: "complete", titleIncludes: ["sleep"] },
  },
  {
    id: "E8",
    prompt: "check off the sleep reminder bro",
    llm: { kind: "complete", title: "sleep", match_mode: "contains" },
    expect: { kind: "complete", titleIncludes: ["sleep"] },
  },

  // F — unsupported
  {
    id: "F1",
    prompt: "create a list called AHHA reminders",
    llm: { kind: "list", range: "all" },
    expect: { kind: "clarify", messageIncludes: ["can't create", "list"] },
  },
  {
    id: "F2",
    prompt: "new list named Work",
    llm: { kind: "set", title: "Work" },
    expect: { kind: "clarify", messageIncludes: ["list"] },
  },
  {
    id: "F3",
    prompt: "clear the false reminders you created",
    llm: { kind: "list", range: "all" },
    expect: { kind: "clarify", messageIncludes: ["can't delete"] },
  },
  {
    id: "F4",
    prompt: "delete all reminders",
    llm: { kind: "complete", title: "all" },
    expect: { kind: "clarify", messageIncludes: ["can't delete"] },
  },

  // G — chat
  {
    id: "G1",
    prompt: "why is the sky blue?",
    llm: { kind: "chat" },
    expect: { kind: "chat" },
  },
  {
    id: "G2",
    prompt: "what is the diff of time in IST and EST",
    llm: { kind: "chat" },
    expect: { kind: "chat" },
  },
  {
    id: "G3",
    prompt: "tell me a joke",
    llm: { kind: "chat" },
    expect: { kind: "chat" },
  },
  {
    id: "G4",
    prompt: "how do I use Martha?",
    llm: { kind: "chat" },
    expect: { kind: "chat" },
  },

  // H — hard phrasing
  {
    id: "H1",
    prompt:
      "so then create a reminder for me (i am in US) to call dad in india at 5 pm india tomorrow",
    llm: {
      kind: "set",
      title: "call dad in india at 5 pm india tomorrow",
      due: `${isoDate(tomorrow())}T17:00`,
    },
    expect: {
      kind: "set",
      titleIncludes: ["call dad"],
      titleExcludes: ["india", "US"],
      dueIsoPrefix: dueFromTz(tomorrow(), 17, 0, 330).slice(0, 16),
    },
  },
  {
    id: "H2",
    prompt: "please remind me tomorrow morning to water plants",
    llm: { kind: "set", title: "water plants tomorrow morning" },
    expect: { kind: "set", titleIncludes: ["water plants"], dueDateOffsetDays: 1 },
  },
  {
    id: "H3",
    prompt: "remind me Friday to pay rent",
    llm: { kind: "set", title: "Friday", due: "Friday" },
    expect: { kind: "set", titleIncludes: ["pay rent"] },
  },
  {
    id: "H4",
    prompt: "add reminder: eggs, milk, bread tomorrow",
    llm: { kind: "set", title: "eggs, milk, bread tomorrow" },
    expect: { kind: "set", dueDateOffsetDays: 1 },
  },
  {
    id: "H5",
    prompt: "remind me",
    llm: { kind: "set", title: "" },
    expect: { kind: "clarify", messageIncludes: ["remind"] },
  },
  {
    id: "H6",
    prompt: "check off",
    llm: { kind: "complete", title: "" },
    expect: { kind: "clarify", messageIncludes: ["check off"] },
  },

  // I — regressions
  {
    id: "I1",
    prompt: "create a reminder for next 3 days to pray at 9 am",
    llm: { kind: "set_many", title: "pray", days: 3, time: "9 am" },
    expect: { kind: "set_many", itemCount: 3, dueLocalTime: "09:00" },
  },
  {
    id: "I2",
    prompt: "remind me to pray tomorrow 9 am",
    llm: { kind: "set", title: "pray tomorrow 9 am" },
    expect: { kind: "set", titleEquals: "pray", dueDateOffsetDays: 1, dueLocalTime: "09:00" },
  },
  {
    id: "I3",
    prompt: "remind me to go home around 10th august this year",
    llm: { kind: "set", title: "go home around 10th august this year" },
    expect: {
      kind: "set",
      titleEquals: "go home",
      dueIsoPrefix: `${new Date().getFullYear()}-08-10`,
    },
  },
  {
    id: "I4",
    prompt: "create a list called AHHA",
    llm: { kind: "list", range: "all" },
    expect: { kind: "clarify", messageIncludes: ["list"] },
  },
  {
    id: "I5",
    prompt: "what do i have to do for the whole of next week",
    llm: { kind: "set_many", title: "next week", days: 7 },
    expect: { kind: "list", range: "week" },
  },
  {
    id: "I6",
    prompt: "remind me to go to the gym in 2 hrs",
    llm: { kind: "set", title: "go gym", due: "02:00" },
    expect: {
      kind: "set",
      titleIncludes: ["gym"],
      dueApproxMinutesFromNow: 120,
      dueToleranceMinutes: 3,
    },
  },
  {
    id: "I7",
    prompt: "remind me to call my dad at 5 pm IST tomorrow",
    llm: { kind: "set", title: "call my dad at 5 pm IST tomorrow", due: "17:00" },
    expect: {
      kind: "set",
      titleEquals: "call my dad",
      dueIsoPrefix: dueFromTz(tomorrow(), 17, 0, 330).slice(0, 16),
    },
  },
  {
    id: "I8",
    prompt: "create a reminder in 8 days from today to complete my OA",
    llm: { kind: "set_many", title: "complete my OA", days: 8 },
    expect: { kind: "set", titleIncludes: ["OA"], dueDateOffsetDays: 8 },
  },
  {
    id: "I9",
    prompt: "check off buy eggs",
    llm: { kind: "complete", title: "Buy eggs" },
    expect: { kind: "complete", titleIncludes: ["buy eggs"] },
  },
  {
    id: "I10",
    prompt: "clear the false reminders",
    llm: { kind: "list", range: "all" },
    expect: { kind: "clarify", messageIncludes: ["can't delete"] },
  },
];

function titleOf(action: ReminderAction): string {
  if (action.kind === "set" || action.kind === "complete") return action.title;
  if (action.kind === "set_many") return action.items[0]?.title ?? "";
  return "";
}

function dueOf(action: ReminderAction): string | undefined {
  if (action.kind === "set") return action.due;
  if (action.kind === "set_many") return action.items[0]?.due;
  return undefined;
}

function check(action: ReminderAction | null, exp: Expect): string[] {
  const errors: string[] = [];
  if (exp.kind === "chat") {
    if (action != null) errors.push(`expected chat(null), got ${JSON.stringify(action)}`);
    return errors;
  }
  if (!action) {
    errors.push(`expected ${exp.kind}, got null/chat`);
    return errors;
  }
  if (action.kind !== exp.kind) {
    errors.push(`kind: want ${exp.kind}, got ${action.kind}`);
    return errors;
  }

  const title = titleOf(action).toLowerCase();
  if (exp.titleEquals != null && titleOf(action).toLowerCase() !== exp.titleEquals.toLowerCase()) {
    errors.push(`titleEquals: want "${exp.titleEquals}", got "${titleOf(action)}"`);
  }
  for (const frag of exp.titleIncludes ?? []) {
    if (!title.includes(frag.toLowerCase())) {
      errors.push(`title missing "${frag}" (got "${titleOf(action)}")`);
    }
  }
  for (const frag of exp.titleExcludes ?? []) {
    if (title.includes(frag.toLowerCase())) {
      errors.push(`title should not contain "${frag}" (got "${titleOf(action)}")`);
    }
  }

  const due = dueOf(action);
  if (exp.dueApproxMinutesFromNow != null) {
    if (!due || !due.includes("T")) {
      errors.push(`dueApprox: missing timed due (got ${due})`);
    } else {
      const [datePart, timePart] = due.split("T");
      const [yy, mm, dd] = datePart.split("-").map(Number);
      const [hh, mi] = timePart.split(":").map(Number);
      const got = new Date(yy, mm - 1, dd, hh, mi).getTime();
      const want = Date.now() + exp.dueApproxMinutesFromNow * 60_000;
      const tol = (exp.dueToleranceMinutes ?? 3) * 60_000;
      if (Math.abs(got - want) > tol) {
        errors.push(
          `dueApprox: want ~${exp.dueApproxMinutesFromNow}m from now, got ${due} (Δ ${Math.round((got - want) / 60000)}m)`,
        );
      }
    }
  }

  if (exp.dueDateOffsetDays != null) {
    const wantDate = isoDate(addDays(new Date(), exp.dueDateOffsetDays));
    if (!due || !due.startsWith(wantDate)) {
      errors.push(`dueDate: want ${wantDate}..., got ${due}`);
    }
  }

  if (exp.dueLocalTime != null) {
    if (!due || !due.includes(`T${exp.dueLocalTime}`)) {
      errors.push(`dueTime: want T${exp.dueLocalTime}, got ${due}`);
    }
  }

  if (exp.dueIsoPrefix != null) {
    if (!due || !due.startsWith(exp.dueIsoPrefix)) {
      errors.push(`duePrefix: want ${exp.dueIsoPrefix}..., got ${due}`);
    }
  }

  if (exp.itemCount != null) {
    if (action.kind !== "set_many" || action.items.length !== exp.itemCount) {
      errors.push(
        `itemCount: want ${exp.itemCount}, got ${action.kind === "set_many" ? action.items.length : action.kind}`,
      );
    } else if (exp.dueLocalTime) {
      for (const [i, item] of action.items.entries()) {
        if (!item.due?.includes(`T${exp.dueLocalTime}`)) {
          errors.push(`item[${i}].due time: want T${exp.dueLocalTime}, got ${item.due}`);
        }
      }
    }
  }

  if (exp.range != null && action.kind === "list" && action.range !== exp.range) {
    // tomorrow list uses start/end instead of range
    if (!(exp.range === "tomorrow" || (action.start && action.end))) {
      errors.push(`range: want ${exp.range}, got ${action.range ?? `start=${action.start}`}`);
    }
  }

  if (exp.messageIncludes && action.kind === "clarify") {
    const msg = action.message.toLowerCase();
    for (const frag of exp.messageIncludes) {
      if (!msg.includes(frag.toLowerCase())) {
        errors.push(`clarify missing "${frag}" (got "${action.message}")`);
      }
    }
  }

  return errors;
}

function main() {
  const passed: string[] = [];
  const failed: { id: string; prompt: string; errors: string[]; got: string }[] = [];

  for (const c of CASES) {
    const action = resolveReminderIntentFromParsed(c.prompt, c.llm);
    const errors = check(action, c.expect);
    if (errors.length === 0) {
      passed.push(c.id);
    } else {
      failed.push({
        id: c.id,
        prompt: c.prompt,
        errors,
        got: JSON.stringify(action),
      });
    }
  }

  console.log("\n=== Reminder intent automated suite ===\n");
  console.log(`Total:  ${CASES.length}`);
  console.log(`Passed: ${passed.length}`);
  console.log(`Failed: ${failed.length}`);
  console.log(`Rate:   ${((passed.length / CASES.length) * 100).toFixed(1)}%\n`);

  if (failed.length) {
    console.log("--- Failures ---\n");
    for (const f of failed) {
      console.log(`${f.id}: "${f.prompt}"`);
      for (const e of f.errors) console.log(`  ✗ ${e}`);
      console.log(`  got: ${f.got}`);
      console.log();
    }
  } else {
    console.log("All cases passed.\n");
  }

  process.exit(failed.length ? 1 : 0);
}

main();
