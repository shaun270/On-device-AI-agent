/**
 * ErrorBoundary — catches any React render/lifecycle crash.
 *
 * Must be a class component — React's error boundary API only works in classes.
 * Wraps the entire App so crashes show a readable error card instead of a blank window.
 */

import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[Martha] ErrorBoundary caught:", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="error-boundary">
          <span className="error-icon">⚠</span>
          <p className="error-title">Something went wrong</p>
          <p className="error-message">{this.state.error?.message ?? "Unknown error"}</p>
          <button
            className="error-reset-btn"
            onClick={() => this.setState({ hasError: false, error: null })}
          >
            Try again
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
