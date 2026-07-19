// All shared TypeScript types for Martha.

export interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
}

/** A full conversation thread. Title is auto-set from the first user message. */
export interface ChatSession {
  id: string;
  title: string;
  messages: Message[];
  createdAt: number;
  updatedAt: number;
}

export type AppStatus = "idle" | "thinking" | "error";

export interface AppSettings {
  opacity: number;
  alwaysOnTop: boolean;
  agentName: string;
  globalHotkey: string;
}
