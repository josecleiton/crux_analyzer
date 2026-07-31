/**
 * The webview panel: one per window, showing the analyzed workspace through
 * the same built web bundle the static site ships. The panel's job is
 * plumbing — resolve configuration, run the analyzer, hand the model to
 * `buildWebviewHtml` — every mapping decision already lives in the bundle.
 *
 * Warnings go to the output channel, never to a modal: the parser honesty
 * rule makes them routine reading, not exceptions.
 */

import * as crypto from 'node:crypto';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { analyze } from './analyzer';
import { resolveSourceDir } from './sourceDir';
import { buildWebviewHtml } from './webviewHtml';

const WATCH_DEBOUNCE_MS = 400;

export class CruxPanel {
  static current: CruxPanel | undefined;

  static show(context: vscode.ExtensionContext, output: vscode.OutputChannel): void {
    if (CruxPanel.current) {
      CruxPanel.current.panel.reveal();
      void CruxPanel.current.refresh();
      return;
    }
    CruxPanel.current = new CruxPanel(context, output);
  }

  private readonly panel: vscode.WebviewPanel;
  private watcher: vscode.FileSystemWatcher | undefined;
  private watchedSrc: string | undefined;
  private refreshTimer: ReturnType<typeof setTimeout> | undefined;
  private rendered = false;
  private disposed = false;
  private isDirty = false;
  private currentTitle = 'Crux Analyzer';

  private constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly output: vscode.OutputChannel,
  ) {
    this.panel = vscode.window.createWebviewPanel(
      'cruxAnalyzer',
      'Crux Analyzer',
      vscode.ViewColumn.Beside,
      {
        enableScripts: true,
        // the bundle re-runs layout on restore otherwise, losing the reader's
        // viewport and any running simulation
        retainContextWhenHidden: true,
        localResourceRoots: [this.webRoot()],
      },
    );
    this.panel.webview.onDidReceiveMessage((msg: { command?: string; isDirty?: boolean }) => {
      if (msg.command === 'setDirty') {
        this.isDirty = Boolean(msg.isDirty);
        this.updateTitle();
      }
    });
    this.panel.onDidDispose(() => this.dispose());
    void this.refresh();
  }

  private updateTitle(): void {
    const prefix = this.isDirty ? '● ' : '';
    this.panel.title = `${prefix}${this.currentTitle}`;
  }

  async refresh(): Promise<void> {
    const workspace = vscode.workspace.workspaceFolders?.[0];
    if (!workspace) {
      this.showMessage(vscode.l10n.t('Open a folder to analyze.'));
      return;
    }

    const config = vscode.workspace.getConfiguration('cruxAnalyzer');
    const src = resolveSourceDir(config.get('src', ''), workspace.uri.fsPath, (p) =>
      fs.existsSync(p),
    );
    if (!src) {
      this.showMessage(
        vscode.l10n.t(
          'Nothing to analyze: no shared/src or src under {0}. Point cruxAnalyzer.src at the Rust sources.',
          workspace.name,
        ),
      );
      return;
    }

    const name = config.get('projectName', '').trim() || workspace.name;
    this.currentTitle = `${name} — Crux Analyzer`;
    this.updateTitle();
    if (!this.rendered) {
      this.showMessage(vscode.l10n.t('Analyzing…'));
    }

    const result = await analyze({
      binary: config.get('binary', 'crux-analyzer').trim() || 'crux-analyzer',
      src,
      name,
    });
    if (this.disposed) return;

    if (result.kind === 'error') {
      this.output.appendLine(result.message);
      if (result.binaryMissing) {
        void vscode.window.showErrorMessage(
          vscode.l10n.t(
            'crux-analyzer binary not found. Install it (cargo install --path crates/cli) or set cruxAnalyzer.binary.',
          ),
        );
      }
      this.showMessage(
        vscode.l10n.t('Analysis failed — see the Crux Analyzer output channel.'),
      );
    } else {
      // warnings are data about what could not be inferred; always show them
      if (result.warnings !== '') this.output.appendLine(result.warnings);
      this.renderModel(result.model);
      this.rendered = true;
    }

    this.ensureWatcher(src, config.get('watch', true));
  }

  private renderModel(model: unknown): void {
    const webRoot = this.webRoot();
    const indexHtml = fs.readFileSync(path.join(webRoot.fsPath, 'index.html'), 'utf8');
    this.panel.webview.html = buildWebviewHtml({
      indexHtml,
      webRootUri: this.panel.webview.asWebviewUri(webRoot).toString(),
      cspSource: this.panel.webview.cspSource,
      nonce: crypto.randomBytes(16).toString('base64'),
      model,
    });
  }

  /** Chrome-only page for states with nothing to render (localized text). */
  private showMessage(text: string): void {
    const escaped = text.replace(/&/g, '&amp;').replace(/</g, '&lt;');
    this.panel.webview.html = `<!doctype html>
<html>
  <head>
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'" />
    <style>body { font-family: sans-serif; opacity: 0.8; padding: 2rem; }</style>
  </head>
  <body><p>${escaped}</p></body>
</html>`;
  }

  /** Watches the analyzed sources; recreated when the resolved dir changes. */
  private ensureWatcher(src: string, enabled: boolean): void {
    if (!enabled) {
      this.watcher?.dispose();
      this.watcher = undefined;
      this.watchedSrc = undefined;
      return;
    }
    if (this.watcher && this.watchedSrc === src) return;
    this.watcher?.dispose();
    this.watchedSrc = src;
    this.watcher = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(vscode.Uri.file(src), '**/*.rs'),
    );
    const schedule = () => {
      if (this.refreshTimer) clearTimeout(this.refreshTimer);
      this.refreshTimer = setTimeout(() => void this.refresh(), WATCH_DEBOUNCE_MS);
    };
    this.watcher.onDidChange(schedule);
    this.watcher.onDidCreate(schedule);
    this.watcher.onDidDelete(schedule);
  }

  private webRoot(): vscode.Uri {
    return vscode.Uri.joinPath(this.context.extensionUri, 'media', 'web');
  }

  private dispose(): void {
    if (this.isDirty) {
      void vscode.window.showInformationMessage(
        vscode.l10n.t(
          'Crux Analyzer: Proposed changes auto-saved to local draft storage. Reopen panel to continue editing.',
        ),
      );
    }
    this.disposed = true;
    if (this.refreshTimer) clearTimeout(this.refreshTimer);
    this.watcher?.dispose();
    CruxPanel.current = undefined;
  }
}
