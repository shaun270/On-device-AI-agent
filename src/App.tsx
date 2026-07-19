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
    setStatus("thinking");
    try {
      const reply = await invoke<string>("echo_message", { message: text });
      addMessage("assistant", reply);
      setStatus("idle");
    } catch (err) {
      console.error("invoke error:", err);
      addMessage("assistant", "⚠ Something went wrong. Please try again.");
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
