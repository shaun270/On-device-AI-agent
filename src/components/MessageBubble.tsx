/**
 * MessageBubble — renders a single chat message.
 *
 * Layout:
 *   - User messages: right-aligned, accent-colored bubble
 *   - Assistant messages: left-aligned, surface-colored bubble
 *   - Timestamp and delete button appear on hover
 */

import type { Message } from "../types";

interface Props {
  message: Message;
  onDelete: (id: string) => void;
}

export function MessageBubble({ message, onDelete }: Props) {
  const time = new Date(message.timestamp).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  return (
    <div className={`message message--${message.role}`}>
      <div className="message-bubble">
        <p className="message-content">{message.content}</p>
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
