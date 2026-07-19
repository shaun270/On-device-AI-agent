/**
 * TypingIndicator — three animated bouncing dots shown while Martha is "thinking".
 * The stagger delay between dots creates a natural wave effect.
 */

interface Props {
  agentName: string;
}

export function TypingIndicator({ agentName }: Props) {
  return (
    <div className="typing-indicator" aria-label={`${agentName} is thinking`} role="status">
      <span className="typing-dot" style={{ animationDelay: "0ms" }} />
      <span className="typing-dot" style={{ animationDelay: "160ms" }} />
      <span className="typing-dot" style={{ animationDelay: "320ms" }} />
    </div>
  );
}
