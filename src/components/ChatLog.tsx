/**
 * ChatLog — scrollable list of messages with auto-scroll and clear button.
 *
 * - useRef + scrollIntoView keeps the latest message visible automatically.
 * - Shows TypingIndicator while status === 'thinking'.
 * - Shows a "Clear history" button at the top when there are messages.
 */

import { useRef, useEffect } from "react";
import type { Message, AppStatus } from "../types";
import { MessageBubble } from "./MessageBubble";
import { TypingIndicator } from "./TypingIndicator";

interface Props {
  messages: Message[];
  status: AppStatus;
  agentName: string;
  onDeleteMessage: (id: string) => void;
  onClearHistory: () => void;
}

export function ChatLog({
  messages,
  status,
  agentName,
  onDeleteMessage,
  onClearHistory,
}: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);

  // Scroll to the latest message whenever messages change or thinking starts.
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length, status]);

  return (
    <div className="chat-log" role="log" aria-live="polite">
      {/* Clear history button — only shown when there are messages */}
      {messages.length > 0 && (
        <div className="chat-log-toolbar">
          <button
            className="clear-history-btn"
            onClick={onClearHistory}
            title="Delete all messages"
          >
            Clear history
          </button>
        </div>
      )}

      {/* Empty state */}
      {messages.length === 0 && status === "idle" && (
        <div className="chat-empty">
          <div className="chat-empty-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
              <path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z"/>
            </svg>
          </div>
          <p className="chat-empty-title">Start a conversation</p>
          <p className="chat-empty-sub">Type a message below and press Enter to send</p>
        </div>
      )}

      {/* Message list */}
      {messages.map((msg) => (
        <MessageBubble key={msg.id} message={msg} onDelete={onDeleteMessage} />
      ))}

      {/* Typing indicator */}
      {status === "thinking" && <TypingIndicator agentName={agentName} />}

      {/* Invisible div at the bottom — scrollIntoView target */}
      <div ref={bottomRef} />
    </div>
  );
}
