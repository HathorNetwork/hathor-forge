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
    console.error("ErrorBoundary caught an error:", error, info);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center h-full gap-6 p-8">
          <div className="text-6xl text-destructive">&#9888;</div>
          <h2 className="text-2xl font-bold text-foreground">
            Something went wrong
          </h2>
          {this.state.error && (
            <pre className="max-w-xl w-full overflow-auto rounded-lg bg-card border border-border p-4 text-sm text-muted-foreground font-mono">
              {this.state.error.message}
            </pre>
          )}
          <button
            onClick={() => window.location.reload()}
            className="px-6 py-2 rounded-lg bg-primary text-primary-foreground font-semibold hover:opacity-90 transition-opacity cursor-pointer"
          >
            Reload
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
