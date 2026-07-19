/**
 * SessionDrawer — slides in from the left showing all chat sessions.
 *
 * Sessions are grouped by date: Today / Yesterday / This Week / Older.
 * Clicking a session switches to it and closes the drawer.
 * Hovering a session shows a delete button.
 */

import type { ChatSession } from "../types";

interface Props {
  sessions: ChatSession[];
  activeSessionId: string;
  onSwitch: (id: string) => void;
  onNew: () => void;
  onDelete: (id: string) => void;
  onClose: () => void;
}

function groupSessions(sessions: ChatSession[]) {
  const now = Date.now();
  const DAY = 86_400_000;
  const groups: { label: string; items: ChatSession[] }[] = [
    { label: "Today", items: [] },
    { label: "Yesterday", items: [] },
    { label: "This Week", items: [] },
    { label: "Older", items: [] },
  ];
  for (const s of sessions) {
    const age = now - s.updatedAt;
    if (age < DAY)           groups[0].items.push(s);
    else if (age < 2 * DAY)  groups[1].items.push(s);
    else if (age < 7 * DAY)  groups[2].items.push(s);
    else                     groups[3].items.push(s);
  }
  return groups.filter((g) => g.items.length > 0);
}

export function SessionDrawer({
  sessions,
  activeSessionId,
  onSwitch,
  onNew,
  onDelete,
  onClose,
}: Props) {
  const groups = groupSessions(sessions);

  function handleSwitch(id: string) {
    onSwitch(id);
    onClose();
  }

  return (
    <>
      {/* Backdrop — click outside to close */}
      <div className="drawer-backdrop" onClick={onClose} />

      <div className="session-drawer" role="dialog" aria-label="Chat history">
        {/* Drawer header */}
        <div className="drawer-header">
          <span className="drawer-title">Chats</span>
          <button className="icon-btn" onClick={onClose} aria-label="Close">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
            </svg>
          </button>
        </div>

        {/* New chat button */}
        <button className="new-chat-btn" onClick={() => { onNew(); onClose(); }}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
          </svg>
          New Chat
        </button>

        {/* Session list */}
        <div className="session-list">
          {groups.map((group) => (
            <div key={group.label} className="session-group">
              <div className="session-group-label">{group.label}</div>
              {group.items.map((session) => (
                <div
                  key={session.id}
                  className={`session-item ${session.id === activeSessionId ? "session-item--active" : ""}`}
                >
                  <button
                    className="session-item-btn"
                    onClick={() => handleSwitch(session.id)}
                    title={session.title}
                  >
                    <span className="session-item-title">{session.title}</span>
                    <span className="session-item-count">
                      {session.messages.length === 0
                        ? "Empty"
                        : `${Math.floor(session.messages.length / 2)} msg${session.messages.length > 2 ? "s" : ""}`}
                    </span>
                  </button>
                  <button
                    className="session-delete-btn"
                    onClick={(e) => { e.stopPropagation(); onDelete(session.id); }}
                    title="Delete session"
                    aria-label="Delete session"
                  >
                    <svg width="9" height="9" viewBox="0 0 24 24" fill="currentColor">
                      <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
                    </svg>
                  </button>
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
