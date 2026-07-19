/**
 * SettingsPanel — slides in from the right when the user clicks ⚙.
 *
 * Contains:
 *   - Agent name input (updates APP_NAME display live)
 *   - Opacity slider (controls background transparency)
 *   - Hotkey recorder (updates global shortcut instantly)
 *   - "Clear all chat history" danger button
 */

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";

interface Props {
  settings: AppSettings;
  onUpdateSetting: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => void;
  onClearHistory: () => void;
  onClose: () => void;
}

export function SettingsPanel({
  settings,
  onUpdateSetting,
  onClearHistory,
  onClose,
}: Props) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;

    function handleKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();

      const key = e.key;
      // Ignore if it's just a bare modifier key
      if (["Control", "Shift", "Alt", "Meta", "Escape"].includes(key)) {
        if (key === "Escape") setRecording(false);
        return;
      }

      const mods = [];
      if (e.metaKey) mods.push("Super");
      if (e.ctrlKey) mods.push("Control");
      if (e.altKey) mods.push("Alt");
      if (e.shiftKey) mods.push("Shift");

      let keyName = key.toUpperCase();
      if (key === " ") keyName = "Space";
      
      const hotkeyStr = [...mods, keyName].join("+");
      onUpdateSetting("globalHotkey", hotkeyStr);
      setRecording(false);
    }

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [recording, onUpdateSetting]);

  function formatHotkey(hotkey: string) {
    if (!hotkey) return "None";
    const isMac = navigator.userAgent.includes("Mac");
    return hotkey
      .split("+")
      .map((part) => {
        if (isMac) {
          if (part === "Super" || part === "CommandOrControl") return "⌘";
          if (part === "Alt") return "⌥";
          if (part === "Shift") return "⇧";
          if (part === "Control") return "⌃";
        }
        return part;
      })
      .join(isMac ? " " : " + ");
  }

  return (
    <div className="settings-panel" role="dialog" aria-label="Settings">
      <div className="settings-header">
        <span className="settings-title">Settings</span>
        <button className="icon-btn" onClick={onClose} aria-label="Close settings">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
          </svg>
        </button>
      </div>

      <div className="settings-body">
        {/* Agent name */}
        <label className="setting-row">
          <span className="setting-label">Agent name</span>
          <input
            type="text"
            className="setting-input"
            value={settings.agentName}
            maxLength={24}
            onChange={(e) => onUpdateSetting("agentName", e.currentTarget.value)}
          />
        </label>

        {/* Opacity */}
        <label className="setting-row">
          <span className="setting-label">
            Opacity
            <span className="setting-value">{Math.round(settings.opacity * 100)}%</span>
          </span>
          <input
            type="range"
            className="setting-slider"
            min={0.3}
            max={1}
            step={0.01}
            value={settings.opacity}
            onChange={(e) => onUpdateSetting("opacity", parseFloat(e.currentTarget.value))}
          />
        </label>

        {/* Hotkey Recorder */}
        <div className="setting-row">
          <span className="setting-label">Global hotkey</span>
          <button
            className="setting-input"
            style={{ textAlign: "left", cursor: "pointer", display: "flex", justifyContent: "space-between", alignItems: "center" }}
            onClick={() => setRecording(true)}
          >
            <kbd className="setting-kbd" style={{ borderColor: recording ? "var(--accent)" : "var(--b2)", color: recording ? "var(--accent)" : "var(--t2)" }}>
              {recording ? "Listening..." : formatHotkey(settings.globalHotkey)}
            </kbd>
            {recording && <span style={{ fontSize: "10px", color: "var(--t3)" }}>Press Esc to cancel</span>}
          </button>
        </div>

        {/* Danger zone */}
        <div className="setting-divider" />
        <button
          className="danger-btn"
          onClick={() => {
            onClearHistory();
            onClose();
          }}
        >
          Clear all chat history
        </button>

        <button
          className="danger-btn"
          onClick={() => invoke("quit_app")}
        >
          Quit {settings.agentName}
        </button>
      </div>
    </div>
  );
}
