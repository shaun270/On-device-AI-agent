/**
 * InputArea — the text input row at the bottom.
 *
 * - <textarea> grows up to 4 lines as you type
 * - Enter → submit message
 * - Shift+Enter → insert a newline (multi-line input)
 * - Stays usable while Martha is thinking (dots live in ChatLog only)
 *
 * Note: Esc is handled globally in useKeyboard, not here.
 */

import { useRef, useEffect, type KeyboardEvent } from "react";
import type { AppStatus } from "../types";

interface Props {
  value: string;
  status: AppStatus;
  agentName: string;
  onChange: (value: string) => void;
  onSubmit: (value: string) => void;
}

export function InputArea({ value, status: _status, agentName, onChange, onSubmit }: Props) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-resize: grow the textarea to fit content, max ~4 lines.
  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 100) + "px";
  }, [value]);

  function handleKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault(); // don't add a newline on plain Enter
      handleSubmit();
    }
  }

  function handleSubmit() {
    const trimmed = value.trim();
    if (!trimmed) return;
    onSubmit(trimmed);
  }

  return (
    <form
      className="input-area"
      onSubmit={(e) => {
        e.preventDefault();
        handleSubmit();
      }}
    >
      <textarea
        ref={textareaRef}
        className="input-textarea"
        value={value}
        rows={1}
        placeholder={`Message ${agentName}…`}
        aria-label="Message input"
        onChange={(e) => onChange(e.currentTarget.value)}
        onKeyDown={handleKeyDown}
        autoFocus
      />
      <button
        type="submit"
        className="send-btn"
        disabled={!value.trim()}
        aria-label="Send message"
        title="Send (Enter)"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
          <path d="M2 21l21-9L2 3v7l15 2-15 2v7z" />
        </svg>
      </button>
    </form>
  );
}
