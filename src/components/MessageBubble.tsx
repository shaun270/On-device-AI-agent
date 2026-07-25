/**
 * MessageBubble — renders a single chat message.
 *
 * Layout:
 *   - User messages: right-aligned, accent-colored bubble
 *   - Assistant messages: left-aligned, surface-colored bubble
 *   - Timestamp and delete button appear on hover
 */

import type { Message } from "../types";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  message: Message;
  onDelete: (id: string) => void;
}

export function MessageBubble({ message, onDelete }: Props) {
  const time = new Date(message.timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  // Parse <PATH:...> tags
  const parts = message.content.split(/(<PATH:.*?>)/g);

  return (
    <div className={`message message--${message.role}`}>
      <div className="message-bubble">
        <p className="message-content">
          {parts.map((part, i) => {
            if (part.startsWith("<PATH:") && part.endsWith(">")) {
              const path = part.slice(6, -1);
              const filename = path.split('/').pop() || path.split('\\').pop() || path;
              return (
                <button
                  key={i}
                  className="file-link-btn"
                  onClick={() => invoke("open_file", { path })}
                  title={`Open ${path}`}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" style={{ marginRight: 6 }}>
                    <path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"/>
                  </svg>
                  {filename}
                </button>
              );
            }
            return <span key={i}>{part}</span>;
          })}
        </p>
        <div className="message-footer">
          <span className="message-time">{time}</span>
          <button
            className="message-delete-btn"
            onClick={() => onDelete(message.id)}
            title="Delete this message"
            aria-label="Delete message"
          >
            ✕
          </button>
        </div>
      </div>
    </div>
  );
}
