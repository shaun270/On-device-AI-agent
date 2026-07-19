/**
 * BrandBar — top drag strip.
 * Left: collapse ▾, sessions history button, current session name
 * Right: New Chat, Pin, Settings, Minimize
 */

import type { AppStatus } from "../types";
import { StatusDot } from "./StatusDot";

interface Props {
  agentName: string;
  sessionTitle: string;
  status: AppStatus;
  isCollapsed: boolean;
  isPinned: boolean;
  onToggleCollapse: () => void;
  onTogglePin: () => void;
  onOpenSettings: () => void;
  onOpenSessions: () => void;
  onNewChat: () => void;
  onMinimize: () => void;
}

export function BrandBar({
  agentName,
  sessionTitle,
  status,
  isCollapsed,
  isPinned,
  onToggleCollapse,
  onTogglePin,
  onOpenSettings,
  onOpenSessions,
  onNewChat,
  onMinimize,
}: Props) {
  return (
    <header className="brand-bar" data-tauri-drag-region>
      {/* Collapse chevron */}
      <button className="icon-btn collapse-btn" onClick={onToggleCollapse} title={isCollapsed ? "Expand" : "Collapse"}>
        <svg width="11" height="11" viewBox="0 0 12 12" fill="currentColor"
          style={{ transform: isCollapsed ? "rotate(-90deg)" : "rotate(0deg)", transition: "transform 0.2s" }}>
          <path d="M6 8L1 3h10L6 8z" />
        </svg>
      </button>

      {/* Agent name + session title */}
      <div className="brand-identity" data-tauri-drag-region>
        <StatusDot status={status} />
        <span className="brand-name" data-tauri-drag-region>{agentName}</span>
        {!isCollapsed && sessionTitle !== "New Chat" && (
          <span className="brand-session" data-tauri-drag-region title={sessionTitle}>
            / {sessionTitle}
          </span>
        )}
      </div>

      {/* Action buttons */}
      <div className="brand-actions">
        {/* Chat history */}
        <button className="icon-btn" onClick={onOpenSessions} title="Chat history" aria-label="Open chat history">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm-2 12H6v-2h12v2zm0-3H6V9h12v2zm0-3H6V6h12v2z"/>
          </svg>
        </button>

        {/* New chat */}
        <button className="icon-btn" onClick={onNewChat} title="New Chat" aria-label="Start new chat">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
          </svg>
        </button>

        {/* Pin toggle */}
        <button className={`icon-btn ${isPinned ? "icon-btn--active" : ""}`}
          onClick={onTogglePin} title={isPinned ? "Unpin" : "Pin on top"}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
            <path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5.2v6h1.6v-6H18v-2l-2-2z" />
          </svg>
        </button>

        {/* Settings */}
        <button className="icon-btn" onClick={onOpenSettings} title="Settings">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19.14 12.94c.04-.3.06-.61.06-.94s-.02-.64-.07-.94l2.03-1.58a.49.49 0 00.12-.61l-1.92-3.32a.49.49 0 00-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 00-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96a.48.48 0 00-.59.22L2.74 8.87a.47.47 0 00.12.61l2.03 1.58c-.05.3-.07.62-.07.94s.02.64.07.94l-2.03 1.58a.47.47 0 00-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32a.47.47 0 00-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
          </svg>
        </button>

        {/* Minimize */}
        <button className="icon-btn icon-btn--close" onClick={onMinimize} title="Minimize">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 13H5v-2h14v2z"/>
          </svg>
        </button>
      </div>
    </header>
  );
}
