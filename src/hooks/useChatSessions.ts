/**
 * useChatSessions — manages multiple chat sessions, each with its own message list.
 *
 * Persisted as one JSON blob in localStorage:
 *   { sessions: ChatSession[], activeSessionId: string }
 *
 * Session title is auto-set from the first user message (max 45 chars).
 */

import { useState, useEffect } from "react";
import type { Message, ChatSession } from "../types";
import { STORAGE_KEYS } from "../constants";

interface SessionStore {
  sessions: ChatSession[];
  activeSessionId: string;
}

function makeSession(): ChatSession {
  return {
    id: crypto.randomUUID(),
    title: "New Chat",
    messages: [],
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
}

function loadStore(): SessionStore {
  try {
    const raw = localStorage.getItem(STORAGE_KEYS.SESSIONS);
    if (raw) {
      const parsed = JSON.parse(raw) as SessionStore;
      if (parsed.sessions?.length > 0) return parsed;
    }
  } catch {
    /* ignore parse errors */
  }
  const initial = makeSession();
  return { sessions: [initial], activeSessionId: initial.id };
}

export function useChatSessions() {
  const [store, setStore] = useState<SessionStore>(loadStore);

  // Persist every time store changes
  useEffect(() => {
    localStorage.setItem(STORAGE_KEYS.SESSIONS, JSON.stringify(store));
  }, [store]);

  const activeSession =
    store.sessions.find((s) => s.id === store.activeSessionId) ??
    store.sessions[0];

  // ── Session management ──────────────────────────────────────

  function newSession() {
    const s = makeSession();
    setStore((prev) => ({ sessions: [s, ...prev.sessions], activeSessionId: s.id }));
  }

  function switchSession(id: string) {
    setStore((prev) => ({ ...prev, activeSessionId: id }));
  }

  function deleteSession(id: string) {
    setStore((prev) => {
      const remaining = prev.sessions.filter((s) => s.id !== id);
      if (remaining.length === 0) {
        const fresh = makeSession();
        return { sessions: [fresh], activeSessionId: fresh.id };
      }
      const nextId =
        prev.activeSessionId === id ? remaining[0].id : prev.activeSessionId;
      return { sessions: remaining, activeSessionId: nextId };
    });
  }

  function renameSession(id: string, title: string) {
    setStore((prev) => ({
      ...prev,
      sessions: prev.sessions.map((s) => (s.id === id ? { ...s, title } : s)),
    }));
  }

  // ── Message management (always on the active session) ────────

  function addMessage(role: Message["role"], content: string, precedingUserText?: string): string {
    const msg: Message = {
      id: crypto.randomUUID(),
      role,
      content,
      timestamp: Date.now(),
      ...(precedingUserText !== undefined ? { precedingUserText } : {}),
    };
    setStore((prev) => ({
      ...prev,
      sessions: prev.sessions.map((s) => {
        if (s.id !== prev.activeSessionId) return s;
        // Auto-title from first user message
        const isFirst = s.messages.length === 0 && role === "user";
        return {
          ...s,
          messages: [...s.messages, msg],
          title: isFirst ? content.slice(0, 45) : s.title,
          updatedAt: Date.now(),
        };
      }),
    }));
    return msg.id;
  }

  function deleteMessage(id: string) {
    setStore((prev) => ({
      ...prev,
      sessions: prev.sessions.map((s) =>
        s.id !== prev.activeSessionId
          ? s
          : { ...s, messages: s.messages.filter((m) => m.id !== id) }
      ),
    }));
  }

  function clearActiveSession() {
    setStore((prev) => ({
      ...prev,
      sessions: prev.sessions.map((s) =>
        s.id !== prev.activeSessionId ? s : { ...s, messages: [], title: "New Chat" }
      ),
    }));
  }

  function clearAllSessions() {
    const initial = makeSession();
    setStore({ sessions: [initial], activeSessionId: initial.id });
  }

  return {
    sessions: store.sessions,
    activeSession,
    activeSessionId: store.activeSessionId,
    newSession,
    switchSession,
    deleteSession,
    renameSession,
    addMessage,
    deleteMessage,
    clearActiveSession,
    clearAllSessions,
  };
}
