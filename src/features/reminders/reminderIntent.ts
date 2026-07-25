import { invoke } from "@tauri-apps/api/core";

/**
 * LLM JSON router → validate/normalize → reminder tools.
 * Returns null when the message should go to the chat model instead.
 */

export type ReminderAction =
  | { kind: "list"; range?: string; days_ahead?: number; start?: string; end?: string; search?: string }
  | { kind: "complete"; title: string; match_mode: "exact" | "contains" }
  | { kind: "set"; title: string; due?: string; list_name?: string }
  | { kind: "set_many"; items: { title: string; due?: string; list_name?: string }[] }
  | { kind: "clarify"; message: string };

const MONTHS: Record<string, number> = {
  january: 1, jan: 1,
  february: 2, feb: 2,
  march: 3, mar: 3,
  april: 4, apr: 4,
  may: 5,
  june: 6, jun: 6,
  july: 7, jul: 7,
  august: 8, aug: 8,
  september: 9, sep: 9, sept: 9,
  october: 10, oct: 10,
  november: 11, nov: 11,
  december: 12, dec: 12,
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

function isoFromDate(d: Date): string {
  return isoLocal(d, { h: d.getHours(), m: d.getMinutes() });
}

/** Fixed UTC offsets in minutes (positive = east of UTC). */
const TZ_UTC_OFFSET_MIN: Record<string, number> = {
  ist: 330,
  india: 330,
  "indian standard time": 330,
  gmt: 0,
  utc: 0,
  est: -300,
  edt: -240,
  cst: -360,
  cdt: -300,
  mst: -420,
  mdt: -360,
  pst: -480,
  pdt: -420,
};

function detectTimezone(userText: string): string | null {
  const t = userText.toLowerCase();
  if (/\bist\b/.test(t) || /\bindian\s+standard\s+time\b/.test(t) || /\bin\s+india\b/.test(t)) {
    return "ist";
  }
  const m = t.match(/\b(est|edt|cst|cdt|mst|mdt|pst|pdt|utc|gmt)\b/);
  return m ? m[1] : null;
}

/**
 * Wall-clock time in a named timezone → local device ISO.
 * Uses the user's local calendar day (today/tomorrow) for the date part.
 */
function dueFromForeignTimezone(
  day: Date,
  clock: { h: number; m: number },
  tzKey: string,
): string {
  const offset = TZ_UTC_OFFSET_MIN[tzKey];
  if (offset == null) return isoLocal(day, clock);
  const utcMs =
    Date.UTC(day.getFullYear(), day.getMonth(), day.getDate(), clock.h, clock.m) -
    offset * 60_000;
  return isoFromDate(new Date(utcMs));
}

/** True only when user wants the SAME task created once per day for N days. */
function wantsMultiDayCreate(userText: string): boolean {
  return (
    /\bfor\s+(?:the\s+)?next\s+\d+\s+days?\s+to\b/i.test(userText) ||
    /\b(?:create|add|set)\s+(?:a\s+)?reminder\s+for\s+(?:the\s+)?next\s+\d+\s+days?\b/i.test(
      userText,
    ) ||
    /\bevery\s+day\b/i.test(userText)
  );
}

/** "in 8 days" / "8 days from today" → single due (not 8 reminders). */
function daysFromNowDue(userText: string): string | undefined {
  if (wantsMultiDayCreate(userText)) return undefined;
  const m =
    userText.match(/\b(?:in|after)\s+(\d+)\s+days?(?:\s+from\s+(?:today|now))?\b/i) ||
    userText.match(/\b(\d+)\s+days?\s+from\s+(?:today|now)\b/i);
  if (!m) return undefined;
  const n = Number(m[1]);
  if (!Number.isFinite(n) || n < 0) return undefined;
  const time = timeFromUserText(userText);
  const day = addDays(new Date(), n);
  const tz = detectTimezone(userText);
  if (tz && time) return dueFromForeignTimezone(day, time, tz);
  return isoLocal(day, time);
}

function parseClock(raw: string): { h: number; m: number } | null {
  const m = String(raw).trim().match(/^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$/i);
  if (!m) return null;
  let h = Number(m[1]);
  const min = m[2] ? Number(m[2]) : 0;
  const ap = m[3]?.toLowerCase();
  if (ap === "pm" && h < 12) h += 12;
  if (ap === "am" && h === 12) h = 0;
  if (h > 23 || min > 59) return null;
  return { h, m: min };
}

/** True when the user is asking what's already scheduled — not creating tasks. */
function looksLikeListQuery(text: string): boolean {
  const t = text.toLowerCase();
  if (
    /\b(?:remind\s+me\s+to|create\s+(?:a\s+)?reminder|add\s+(?:a\s+)?reminder|set\s+(?:a\s+)?reminder|check\s*off|mark\s+(?:as\s+)?done)\b/i.test(
      t,
    )
  ) {
    return false;
  }
  if (/\bwhat do i (?:have|need) to do\b/i.test(t)) return true;
  if (/\b(?:what(?:'s|s)|whats)\s+(?:on|due|planned|scheduled)\b/i.test(t)) return true;
  if (
    /\b(what|show|list|see|check|any|have)\b/i.test(t) &&
    /\b(reminders?|todos?|to-?dos?|schedule|due)\b/i.test(t)
  ) {
    return true;
  }
  if (/\b(?:for|this|next)\s+(?:the\s+)?(?:whole\s+(?:of\s+)?)?(?:week|today|tomorrow)\b/i.test(t) &&
      /\b(what|show|list|do i|have|need)\b/i.test(t)) {
    return true;
  }
  return false;
}

function listActionFromUser(userText: string): ReminderAction {
  const t = userText.toLowerCase();
  if (/\btomorrow\b/.test(t)) {
    const day = isoDate(addDays(new Date(), 1));
    return { kind: "list", start: day, end: day };
  }
  if (/\btoday\b/.test(t)) return { kind: "list", range: "today" };
  if (/\b(?:next\s+)?week\b/.test(t) || /\bwhole\s+of\s+next\s+week\b/.test(t)) {
    return { kind: "list", range: "week" };
  }
  return { kind: "list", range: "all" };
}

/** "in 2 hrs" / "in 30 minutes" → absolute local due. */
function relativeDurationDue(userText: string): string | undefined {
  const m = userText.match(/\bin\s+(\d+)\s*(hours?|hrs?|h|minutes?|mins?)\b/i);
  if (!m) return undefined;
  const n = Number(m[1]);
  if (!Number.isFinite(n) || n <= 0) return undefined;
  const unit = m[2].toLowerCase();
  const d = new Date();
  if (unit.startsWith("m")) d.setMinutes(d.getMinutes() + n);
  else d.setHours(d.getHours() + n);
  return isoFromDate(d);
}

/** Accept ISO, or relative junk the model sometimes leaves in `due`. */
function normalizeDue(raw: unknown, userText: string): string | undefined {
  if (raw == null) return undefined;
  const s = String(raw).trim();
  if (!s) return undefined;

  // Prefer user relative duration over a wrong model clock time.
  const relative = relativeDurationDue(userText);
  if (relative && /\bin\s+\d+\s*(hours?|hrs?|h|minutes?|mins?)\b/i.test(userText)) {
    return relative;
  }

  if (/^\d{4}-\d{2}-\d{2}(T\d{2}:\d{2}(:\d{2})?)?$/.test(s)) {
    return s.length === 10 ? s : s.slice(0, 16);
  }

  if (/now_plus|in\s*\d+\s*h/i.test(s)) {
    return relativeDurationDue(userText);
  }

  const timeFromUser =
    parseClock(userText.match(/\b(?:at|around|@)\s+(\d{1,2}(?::\d{2})?\s*(?:am|pm)?)\b/i)?.[1] ?? "") ??
    parseClock(userText.match(/\b(\d{1,2}(?::\d{2})?\s*(?:am|pm))\b/i)?.[1] ?? "") ??
    parseClock(s.match(/T?(\d{1,2}:\d{2})/)?.[1] ?? "") ??
    undefined;

  const lower = s.toLowerCase();
  if (/\btomorrow\b/.test(lower) || lower.includes("use_tomorrow") || lower.includes("tomorrow_t")) {
    return isoLocal(addDays(new Date(), 1), timeFromUser);
  }
  if (/\btoday\b/.test(lower)) {
    return isoLocal(new Date(), timeFromUser);
  }

  const monthFirst = s.match(
    /\b(january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)\s+(\d{1,2})(?:st|nd|rd|th)?(?:\s*,?\s*(\d{4}))?\b/i,
  );
  const dayFirst = s.match(
    /\b(\d{1,2})(?:st|nd|rd|th)?\s+(january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)(?:\s*,?\s*(\d{4}))?\b/i,
  );
  if (monthFirst || dayFirst) {
    const m = monthFirst ?? dayFirst!;
    const monthName = (monthFirst ? m[1] : m[2]).toLowerCase();
    const day = Number(monthFirst ? m[2] : m[1]);
    const year = m[3] ? Number(m[3]) : new Date().getFullYear();
    const month = MONTHS[monthName];
    if (month && day >= 1 && day <= 31) {
      return isoLocal(new Date(year, month - 1, day), timeFromUser);
    }
  }

  const ymd = s.match(/\b(?:YYYY|yyyy|\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}))?\b/);
  if (ymd) {
    const year = new Date().getFullYear();
    const d = new Date(year, Number(ymd[1]) - 1, Number(ymd[2]));
    const t =
      ymd[3] != null
        ? { h: Number(ymd[3]), m: Number(ymd[4]) }
        : timeFromUser;
    return isoLocal(d, t);
  }

  return undefined;
}

function timeFromUserText(userText: string): { h: number; m: number } | undefined {
  return (
    parseClock(
      userText.match(/\b(?:at|around|@)\s+(\d{1,2}(?::\d{2})?\s*(?:am|pm)?)\s*(?:ist|est|edt|pst|pdt|cst|cdt|mst|mdt|utc|gmt)?\b/i)?.[1] ??
        "",
    ) ??
    parseClock(
      userText.match(/\b(\d{1,2}(?::\d{2})?\s*(?:am|pm))\s*(?:ist|est|edt|pst|pdt|cst|cdt|mst|mdt|utc|gmt)?\b/i)?.[1] ??
        "",
    ) ??
    undefined
  );
}

function salvageDueFromUser(userText: string): string | undefined {
  const relative = relativeDurationDue(userText);
  if (relative) return relative;

  const inDays = daysFromNowDue(userText);
  if (inDays) return inDays;

  const time = timeFromUserText(userText);
  const tz = detectTimezone(userText);
  let day: Date | undefined;
  if (/\btomorrow\b/i.test(userText)) day = addDays(new Date(), 1);
  else if (/\btoday\b/i.test(userText)) day = new Date();

  const monthFirst = userText.match(
    /\b(january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)\s+(\d{1,2})(?:st|nd|rd|th)?(?:\s*,?\s*(?:(\d{4})|this\s+year))?\b/i,
  );
  const dayFirst = userText.match(
    /\b(\d{1,2})(?:st|nd|rd|th)?\s+(january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)(?:\s*,?\s*(?:(\d{4})|this\s+year))?\b/i,
  );
  if (monthFirst || dayFirst) {
    const m = monthFirst ?? dayFirst!;
    const monthName = (monthFirst ? m[1] : m[2]).toLowerCase();
    const dayNum = Number(monthFirst ? m[2] : m[1]);
    const yearRaw = m[3];
    const year = yearRaw && /^\d{4}$/.test(yearRaw) ? Number(yearRaw) : new Date().getFullYear();
    const month = MONTHS[monthName];
    if (month && dayNum >= 1 && dayNum <= 31) {
      day = new Date(year, month - 1, dayNum);
    }
  }

  if (tz && time) return dueFromForeignTimezone(day ?? new Date(), time, tz);
  if (day) return isoLocal(day, time);
  if (time) return isoLocal(new Date(), time);
  return undefined;
}

/** Strip when/date/time/tz filler; keep natural phrasing (go to the gym). */
function cleanTaskTitle(raw: string): string {
  return raw
    .replace(/\([^)]*\)/g, " ")
    .replace(/\bin\s+\d+\s*(?:hours?|hrs?|h|minutes?|mins?)\b/gi, " ")
    .replace(/\b(?:in|after)\s+\d+\s+days?(?:\s+from\s+(?:today|now))?\b/gi, " ")
    .replace(/\b\d+\s+days?\s+from\s+(?:today|now)\b/gi, " ")
    .replace(/\b(?:tomorrow|today|tonight)\b/gi, " ")
    .replace(
      /\b(?:at|around|@)\s+\d{1,2}(?::\d{2})?\s*(?:am|pm)?\s*(?:ist|est|edt|pst|pdt|cst|cdt|mst|mdt|utc|gmt|india)?\b/gi,
      " ",
    )
    .replace(
      /\b\d{1,2}(?::\d{2})?\s*(?:am|pm)\s*(?:ist|est|edt|pst|pdt|cst|cdt|mst|mdt|utc|gmt|india)?\b/gi,
      " ",
    )
    .replace(/\b(?:ist|est|edt|pst|pdt|cst|cdt|mst|mdt|utc|gmt)\b/gi, " ")
    .replace(/\b(?:indian\s+standard\s+time|eastern\s+(?:standard|daylight)\s+time)\b/gi, " ")
    .replace(/\b(?:who\s+is\s+)?in\s+india\b/gi, " ")
    .replace(/\b(?:here\s+)?in\s+(?:the\s+)?(?:us|usa|united\s+states)\b/gi, " ")
    .replace(/\bi\s+am\b/gi, " ")
    .replace(/\b(?:india|india'?s)\b/gi, " ")
    .replace(/\baround\b/gi, " ")
    .replace(
      /\b(?:\d{1,2}(?:st|nd|rd|th)?\s+)?(?:january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)(?:\s+\d{1,2}(?:st|nd|rd|th)?)?(?:\s*(?:,?\s*)?(?:\d{4}|this\s+year))?\b/gi,
      " ",
    )
    .replace(/\b(?:this\s+year|next\s+\d+\s+days?|in\s+\d+\s+days?|for\s+next\s+\d+\s+days?)\b/gi, " ")
    .replace(/\b(?:whole\s+of\s+)?(?:next|this)\s+week\b/gi, " ")
    .replace(/[()]/g, " ")
    .replace(/\s{2,}/g, " ")
    .replace(/^[\s,.-]+|[\s,.-]+$/g, "")
    .replace(/^(?:(?:to|for|me)\s+)+/i, "")
    .trim();
}

function salvageCreateTitle(userText: string): string | undefined {
  const m = userText.match(
    /\b(?:remind\s+me\s+to|remind\s+me|create\s+(?:a\s+)?reminder\s+(?:to\s+|for\s+)?|add\s+(?:a\s+)?reminder\s+(?:to\s+|for\s+)?)\s*(.+)$/i,
  );
  if (!m) return undefined;
  const cleaned = cleanTaskTitle(m[1]);
  return cleaned || undefined;
}

function cleanCompleteTitle(raw: string): string {
  return raw
    .replace(/^["']|["']$/g, "")
    .replace(/\bin\s+reminders?\b/gi, " ")
    .replace(/^(?:the\s+)?(?:reminder\s+(?:called|titled|for|on)\s+)?/i, "")
    .replace(/^(?:go\s+and\s+|go\s+)/i, "")
    .replace(/\b(?:reminder|one)\b/gi, " ")
    .replace(/\b(?:please)\b/gi, " ")
    .replace(/\b(?:the|a|an)\b/gi, " ")
    .replace(/\s{2,}/g, " ")
    .replace(/^[\s,.-]+|[\s,.-]+$/g, "")
    .trim();
}

function extractJsonObject(raw: string): string | null {
  const start = raw.indexOf("{");
  const end = raw.lastIndexOf("}");
  if (start < 0 || end <= start) return null;
  return raw.slice(start, end + 1);
}

function asPositiveInt(v: unknown, fallback?: number): number | undefined {
  if (typeof v === "number" && Number.isFinite(v)) return Math.max(0, Math.floor(v));
  if (typeof v === "string" && /^\d+$/.test(v.trim())) return Number(v.trim());
  return fallback;
}

function expandSetMany(
  title: string,
  days: number,
  timeRaw: unknown,
  userText: string,
): ReminderAction {
  const clock =
    (typeof timeRaw === "string" ? parseClock(timeRaw) : null) ??
    parseClock(userText.match(/\b(?:at|around|@)\s+(\d{1,2}(?::\d{2})?\s*(?:am|pm)?)\b/i)?.[1] ?? "") ??
    { h: 9, m: 0 };
  const n = Math.min(14, Math.max(1, days));
  const clean = cleanTaskTitle(title) || title.trim();
  return {
    kind: "set_many",
    items: Array.from({ length: n }, (_, i) => ({
      title: clean,
      due: isoLocal(addDays(new Date(), i), clock),
    })),
  };
}

function isHollowMultiTitle(title: string): boolean {
  const t = title.toLowerCase().trim();
  return !t || /^(next\s+week|this\s+week|week|tomorrow|today)$/i.test(t);
}

/** Turn raw LLM JSON into a safe ReminderAction, or null → chat. */
export function normalizeReminderAction(
  parsed: Record<string, unknown>,
  userText: string,
): ReminderAction | null {
  // Hard override: query phrasing must never create reminders.
  if (looksLikeListQuery(userText)) {
    return listActionFromUser(userText);
  }

  const kind = String(parsed.kind ?? "").toLowerCase().trim();
  if (!kind || kind === "chat") return null;

  if (kind === "clarify") {
    const message = String(parsed.message ?? "").trim();
    if (!message) return null;
    return { kind: "clarify", message };
  }

  if (kind === "list") {
    const range = parsed.range != null ? String(parsed.range).toLowerCase().trim() : undefined;
    const days_ahead = asPositiveInt(parsed.days_ahead);
    const start = parsed.start != null ? normalizeDue(parsed.start, userText) : undefined;
    const end = parsed.end != null ? normalizeDue(parsed.end, userText) : undefined;
    const search = parsed.search != null ? String(parsed.search).trim() : undefined;
    if (range || days_ahead != null || start || end || search) {
      return {
        kind: "list",
        range: range || undefined,
        days_ahead,
        start,
        end,
        search: search || undefined,
      };
    }
    return listActionFromUser(userText);
  }

  if (kind === "complete") {
    let title = cleanCompleteTitle(String(parsed.title ?? ""));
    if (!title) {
      const m = userText.match(
        /(?:check\s*off|mark\s+(?:as\s+)?(?:done|complete)|complete|finish)\s+(.+)$/i,
      );
      title = cleanCompleteTitle(m?.[1] ?? "");
    }
    // Soften "go buy eggs" → also try without leading go
    if (!title) {
      return { kind: "clarify", message: "Which reminder should I check off? e.g. check off Buy milk" };
    }
    const mode = String(parsed.match_mode ?? "contains").toLowerCase();
    return {
      kind: "complete",
      title,
      match_mode: mode === "exact" ? "exact" : "contains",
    };
  }

  if (kind === "set_many") {
    const titleGuess = String(parsed.title ?? "");
    if (isHollowMultiTitle(cleanTaskTitle(titleGuess) || titleGuess)) {
      return listActionFromUser(userText);
    }

    // "in 8 days" is ONE reminder — never expand into 8 creates.
    if (!wantsMultiDayCreate(userText)) {
      const title =
        cleanTaskTitle(titleGuess) ||
        salvageCreateTitle(userText) ||
        titleGuess.trim();
      return {
        kind: "set",
        title: title || "reminder",
        due: daysFromNowDue(userText) ?? salvageDueFromUser(userText),
      };
    }

    if (parsed.title && (parsed.days != null || parsed.count != null)) {
      const days = asPositiveInt(parsed.days ?? parsed.count, 1) ?? 1;
      return expandSetMany(String(parsed.title), days, parsed.time, userText);
    }
    if (Array.isArray(parsed.items) && parsed.items.length > 0) {
      const items = parsed.items
        .map((item) => {
          if (!item || typeof item !== "object") return null;
          const row = item as Record<string, unknown>;
          const title = cleanTaskTitle(String(row.title ?? "")) || String(row.title ?? "").trim();
          if (!title || isHollowMultiTitle(title)) return null;
          return {
            title,
            due: normalizeDue(row.due, userText),
            list_name: row.list_name != null ? String(row.list_name) : undefined,
          };
        })
        .filter((x): x is NonNullable<typeof x> => x != null);
      if (items.length) return { kind: "set_many", items };
      return listActionFromUser(userText);
    }
    return {
      kind: "clarify",
      message: "I couldn't build those reminders. Try: create a reminder for next 3 days to pray at 9 am",
    };
  }

  if (kind === "set") {
    if (
      parsed.days != null ||
      parsed.count != null
    ) {
      const days = asPositiveInt(parsed.days ?? parsed.count, 1) ?? 1;
      if (days > 1 && parsed.title && wantsMultiDayCreate(userText)) {
        return expandSetMany(String(parsed.title), days, parsed.time, userText);
      }
    }

    let title = cleanTaskTitle(String(parsed.title ?? ""));
    const salvaged = salvageCreateTitle(userText);
    if (salvaged && (!title || isHollowMultiTitle(title) || salvaged.length >= title.length)) {
      title = salvaged;
    }
    if (!title) title = String(parsed.title ?? "").trim();
    if (!title || isHollowMultiTitle(title)) {
      return {
        kind: "clarify",
        message: "What should I remind you about? e.g. remind me to go to the gym in 2 hours",
      };
    }

    // User-derived due always wins over a wrong model clock (TZ, in N days, in 2 hrs).
    let due =
      relativeDurationDue(userText) ||
      daysFromNowDue(userText) ||
      salvageDueFromUser(userText);
    if (!due) due = normalizeDue(parsed.due ?? parsed.time, userText);

    return {
      kind: "set",
      title,
      due,
      list_name: parsed.list_name != null ? String(parsed.list_name) : undefined,
    };
  }

  return null;
}

/**
 * Fast paths that never call the LLM (lists, unsupported ops, clear list queries).
 * Returns undefined when the message still needs classify_intent.
 */
export function tryDeterministicReminderIntent(text: string): ReminderAction | null | undefined {
  const t = text.trim();
  if (!t || t.startsWith("/")) return null;

  if (/\bcreate\s+(?:a\s+)?list\b/i.test(t) || /\bnew\s+list\b/i.test(t)) {
    const name = t.match(/\b(?:called|named)\s+["']?(.+?)["']?\s*$/i)?.[1]?.trim();
    return {
      kind: "clarify",
      message: name
        ? `I can't create Reminders lists yet. Create "${name}" in the Reminders app, then I can add items there.`
        : "I can't create Reminders lists yet — only reminders inside an existing list.",
    };
  }

  if (/\b(?:clear|delete|remove|erase)\b/i.test(t) && /\breminders?\b/i.test(t)) {
    return {
      kind: "clarify",
      message:
        "I can't delete reminders yet — only create, list, and check them off. Remove the extras in the Reminders app, or say e.g. check off complete OA.",
    };
  }

  if (looksLikeListQuery(t)) {
    return listActionFromUser(t);
  }

  return undefined;
}

/** Resolve with a caller-supplied classifier JSON (for tests / custom routers). */
export function resolveReminderIntentFromParsed(
  text: string,
  parsed: Record<string, unknown> | null,
): ReminderAction | null {
  const t = text.trim();
  const early = tryDeterministicReminderIntent(t);
  if (early !== undefined) return early;
  if (!parsed) return null;
  return normalizeReminderAction(parsed, t);
}

export async function resolveReminderIntent(text: string): Promise<ReminderAction | null> {
  const t = text.trim();
  const early = tryDeterministicReminderIntent(t);
  if (early !== undefined) return early;

  try {
    const now = new Date();
    const tomorrow = addDays(now, 1);
    const currentDate =
      `now=${isoDate(now)}T${pad2(now.getHours())}:${pad2(now.getMinutes())}; ` +
      `tomorrow=${isoDate(tomorrow)}; year=${now.getFullYear()}`;
    const result = await invoke<string>("classify_intent", {
      text: t,
      currentDate,
    });

    const jsonText = extractJsonObject(result) ?? result.trim();
    let parsed: Record<string, unknown>;
    try {
      parsed = JSON.parse(jsonText) as Record<string, unknown>;
    } catch {
      console.error("Router returned non-JSON:", result);
      return null;
    }

    return normalizeReminderAction(parsed, t);
  } catch (error) {
    console.error("Failed to classify intent with LLM:", error);
    return null;
  }
}
