import { invoke } from "@tauri-apps/api/core";

/**
 * Cross-capability entry point: the backend's hierarchical embedding
 * router decides which domain/action a message belongs to, then fills its
 * slots. Domains that need frontend-side formatting (reminders) come back
 * under their own `kind` for the caller to normalize/execute; anything
 * already executed on the backend (files, and any future domain that
 * doesn't need frontend formatting) comes back as `{"kind":"reply"}`.
 * `{"kind":"unhandled"}` means nothing confidently matched, or a domain's
 * slot-filler couldn't safely proceed alone — the caller should fall back
 * to the general chat agent, which has full conversation history.
 */
export type RouteOutcome =
  | { kind: "unhandled" }
  | { kind: "reply"; message: string }
  | { kind: "reminder"; parsed: Record<string, unknown> };

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function isoDate(d: Date): string {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}

export function currentDateContext(): string {
  const now = new Date();
  const tomorrow = new Date(now);
  tomorrow.setDate(tomorrow.getDate() + 1);
  return (
    `now=${isoDate(now)}T${pad2(now.getHours())}:${pad2(now.getMinutes())}; ` +
    `tomorrow=${isoDate(tomorrow)}; year=${now.getFullYear()}`
  );
}

export async function routeMessage(text: string): Promise<RouteOutcome> {
  try {
    const result = await invoke<string>("route_intent", {
      text,
      currentDate: currentDateContext(),
    });

    const parsed = JSON.parse(result) as Record<string, unknown>;
    const kind = String(parsed.kind ?? "");

    if (!kind || kind === "unhandled") return { kind: "unhandled" };
    if (kind === "reply") return { kind: "reply", message: String(parsed.message ?? "") };
    return { kind: "reminder", parsed };
  } catch (error) {
    console.error("Failed to route message:", error);
    return { kind: "unhandled" };
  }
}

/**
 * Same contract as `routeMessage`, but the domain/action is given directly
 * instead of decided by the router — used by the personalization picker's
 * text-entry correction step (see `features/personalization`).
 */
export async function executeForcedAction(
  domain: string,
  action: string,
  text: string,
): Promise<RouteOutcome> {
  try {
    const result = await invoke<string>("execute_forced_action", {
      domain,
      action,
      text,
      currentDate: currentDateContext(),
    });

    const parsed = JSON.parse(result) as Record<string, unknown>;
    const kind = String(parsed.kind ?? "");

    if (!kind || kind === "unhandled") return { kind: "unhandled" };
    if (kind === "reply") return { kind: "reply", message: String(parsed.message ?? "") };
    return { kind: "reminder", parsed };
  } catch (error) {
    console.error("Failed to execute forced action:", error);
    return { kind: "unhandled" };
  }
}
