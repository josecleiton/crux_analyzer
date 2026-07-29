/**
 * Entry point. One command, one panel: `Crux Analyzer: Show State Machines`
 * opens (or reveals) the state-machine view for the current workspace.
 */

import * as vscode from 'vscode';
import { CruxPanel } from './panel';

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel('Crux Analyzer');
  context.subscriptions.push(
    output,
    vscode.commands.registerCommand('cruxAnalyzer.show', () =>
      CruxPanel.show(context, output),
    ),
  );
}

export function deactivate(): void {}
