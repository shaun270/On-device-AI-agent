/**
 * StatusDot — a small colored circle that communicates the app's current state.
 *
 * idle    → green (steady)
 * thinking→ amber (pulsing animation)
 * error   → red (steady)
 */

import type { AppStatus } from "../types";

interface Props {
  status: AppStatus;
}

export function StatusDot({ status }: Props) {
  return (
    <span
      className={`status-dot status-dot--${status}`}
      aria-label={`Status: ${status}`}
      title={status.charAt(0).toUpperCase() + status.slice(1)}
    />
  );
}
