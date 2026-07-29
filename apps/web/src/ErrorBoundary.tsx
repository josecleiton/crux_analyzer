/**
 * Last line of defence around the app.
 *
 * Every string this UI renders — state, event and effect names, tags, prose —
 * comes out of the analyzed application, and the model is validated but not
 * exhaustively: a shape nobody anticipated should degrade to a message, not to
 * a white screen with the explanation hidden in a console nobody opens. That
 * matters most in the VS Code webview, where there is no console in view at all.
 *
 * Deliberately not localized through `useTranslate`: if rendering is what broke,
 * the provider that supplies translations may be part of what broke.
 */

import { Component, type ErrorInfo, type ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

interface State {
  message: string | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { message: null };

  static getDerivedStateFromError(error: unknown): State {
    return { message: error instanceof Error ? error.message : String(error) };
  }

  componentDidCatch(error: unknown, info: ErrorInfo) {
    console.error('crux_analyzer: render failed', error, info.componentStack);
  }

  render() {
    if (this.state.message === null) return this.props.children;
    return (
      <div className="fatal-error" role="alert">
        <h1>Something went wrong rendering this model.</h1>
        <p>
          This is a bug in crux_analyzer, not in the analyzed application. The
          details are in the browser console.
        </p>
        <pre>{this.state.message}</pre>
      </div>
    );
  }
}
