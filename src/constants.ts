import type { AppSettings } from "./types";

export const APP_NAME = "Martha";
export const APP_PLACEHOLDER = `Message ${APP_NAME}…`;

export const STORAGE_KEYS = {
  SESSIONS: "martha_sessions",     // { sessions[], activeSessionId }
  SETTINGS: "martha_settings",
} as const;

export const DEFAULT_SETTINGS: AppSettings = {
  opacity: 0.97,
  alwaysOnTop: true,
  agentName: APP_NAME,
  globalHotkey: "CommandOrControl+M",
};
