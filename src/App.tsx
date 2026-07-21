import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

import { useChatSessions } from "./hooks/useChatSessions";
import { useSettings } from "./hooks/useSettings";
import { useWindowControls } from "./hooks/useWindowControls";
import { useKeyboard } from "./hooks/useKeyboard";

import { ErrorBoundary } from "./components/ErrorBoundary";
import { BrandBar } from "./components/BrandBar";
import { ChatLog } from "./components/ChatLog";
import { InputArea } from "./components/InputArea";
import { SettingsPanel } from "./components/SettingsPanel";
import { SessionDrawer } from "./components/SessionDrawer";

import type { AppStatus } from "./types";
import "./App.css";

export default function App() {
  const [input, setInput] = useState("");
  const [status, setStatus] = useState<AppStatus>("idle");
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showSessions, setShowSessions] = useState(false);

  const {
    sessions, activeSession, activeSessionId,
    newSession, switchSession, deleteSession,
    addMessage, deleteMessage, clearActiveSession, clearAllSessions,
  } = useChatSessions();

  const { settings, updateSetting } = useSettings();
  const { minimizeWindow } = useWindowControls(settings);

  const handleEsc = useCallback(() => {
    if (showSettings) return setShowSettings(false);
    if (showSessions) return setShowSessions(false);
    if (input.trim()) return setInput("");
    minimizeWindow();
  }, [showSettings, showSessions, input, minimizeWindow]);

  useKeyboard({ inputValue: input, onClearInput: () => setInput(""), onHideWindow: handleEsc });

  async function handleSend(text: string) {
    setInput("");
    setShowSessions(false);
    addMessage("user", text);

    const trimmed = text.trim();

    // Instant clarifying replies — no wait, so no dots.
    if (/^\/done\b/i.test(trimmed) && !trimmed.replace(/^\/done\b/i, "").trim()) {
      addMessage("assistant", "Which reminder should I check off? e.g. /done Buy milk");
      setStatus("idle");
      return;
    }
    if (/^\/remind\b/i.test(trimmed) && !trimmed.replace(/^\/remind\b/i, "").trim()) {
      addMessage("assistant", "What should I remind you about? e.g. /remind Buy milk");
      setStatus("idle");
      return;
    }

    // Show … only while we're waiting for a reply; cleared the moment we have one.
    setStatus("thinking");

    try {
      // Temporary slash commands until Claude tool-calling is wired.
      let reply: string;
      const listMatch = trimmed.match(
        /^\/reminders(?:\s+(all|today|week|\d+|search\s+.+))?$/i,
      );
      if (listMatch) {
        const arg = (listMatch[1] || "all").trim();
        if (/^\d+$/.test(arg)) {
          reply = await invoke<string>("list_reminders", {
            days_ahead: Number(arg),
            limit: 25,
          });
        } else if (/^search\s+/i.test(arg)) {
          reply = await invoke<string>("list_reminders", {
            search: arg.replace(/^search\s+/i, "").trim(),
            limit: 25,
          });
        } else {
          reply = await invoke<string>("list_reminders", {
            range: arg.toLowerCase(),
            limit: 25,
          });
        }
      } else if (/^\/done\b/i.test(trimmed)) {
        const rest = trimmed.replace(/^\/done\b/i, "").trim();
        const [titlePart, matchPart] = rest.split("|").map((s) => s.trim());
        reply = await invoke<string>("complete_reminder", {
          title: titlePart,
          match_mode: matchPart || "exact",
        });
      } else if (/^\/remind\b/i.test(trimmed)) {
        const rest = trimmed.replace(/^\/remind\b/i, "").trim();
        const [titlePart, timePart] = rest.split("|").map((s) => s.trim());
        reply = await invoke<string>("set_reminder", {
          title: titlePart,
          due: timePart || null,
        });
      } else {
        reply =
          "No agent wired yet. Use /remind, /reminders, or /done — or wait for the new agent code.";
      }
      addMessage("assistant", reply);
      setStatus("idle");
    } catch (err) {
      console.error("invoke error:", err);
      let message = "⚠ Something went wrong. Please try again.";
      if (typeof err === "string") {
        message = err;
      } else if (err instanceof Error) {
        message = err.message;
      } else if (err && typeof err === "object" && "message" in err) {
        message = String((err as { message: unknown }).message);
      }
      addMessage("assistant", message);
      setStatus("error");
      setTimeout(() => setStatus("idle"), 2000);
    }
  }

  function handleNewChat() {
    newSession();
    setShowSessions(false);
    setInput("");
    setStatus("idle");
  }

  return (
    <ErrorBoundary>
      <div className="app-shell">
        {/* Session drawer (slides in from left) */}
        {showSessions && (
          <SessionDrawer
            sessions={sessions}
            activeSessionId={activeSessionId}
            onSwitch={switchSession}
            onNew={handleNewChat}
            onDelete={deleteSession}
            onClose={() => setShowSessions(false)}
          />
        )}

        {/* Settings overlay */}
        {showSettings && (
          <SettingsPanel
            settings={settings}
            onUpdateSetting={updateSetting}
            onClearHistory={clearAllSessions}
            onClose={() => setShowSettings(false)}
          />
        )}

        {/* Main window */}
        <div className="bar" data-tauri-drag-region>
          <BrandBar
            agentName={settings.agentName}
            sessionTitle={activeSession.title}
            status={status}
            isCollapsed={isCollapsed}
            isPinned={settings.alwaysOnTop}
            onToggleCollapse={() => setIsCollapsed((c) => !c)}
            onTogglePin={() => updateSetting("alwaysOnTop", !settings.alwaysOnTop)}
            onOpenSettings={() => { setShowSettings((s) => !s); setShowSessions(false); }}
            onOpenSessions={() => { setShowSessions((s) => !s); setShowSettings(false); }}
            onNewChat={handleNewChat}
            onMinimize={minimizeWindow}
          />

          {!isCollapsed && (
            <>
              <ChatLog
                messages={activeSession.messages}
                status={status}
                agentName={settings.agentName}
                onDeleteMessage={deleteMessage}
                onClearHistory={clearActiveSession}
              />
              <InputArea
                value={input}
                status={status}
                agentName={settings.agentName}
                onChange={setInput}
                onSubmit={handleSend}
              />
            </>
          )}
        </div>
      </div>
    </ErrorBoundary>
  );
}
