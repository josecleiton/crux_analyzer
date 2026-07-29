/**
 * Runs the crux-analyzer CLI and hands back the model JSON. The extension is
 * a client of the same contract as every other client: it never parses Rust,
 * it spawns the analyzer and consumes `generate`'s stdout.
 *
 * Warnings are part of the result, not noise: the parser honesty rule prints
 * what it could not infer to stderr, and the extension surfaces that in its
 * output channel instead of dropping it.
 */

import { execFile } from 'node:child_process';

export interface AnalyzeRequest {
  /** Binary name or path (config `cruxAnalyzer.binary`). */
  binary: string;
  /** Directory with the Rust sources. */
  src: string;
  /** Project name for the model. */
  name: string;
}

export type AnalyzeResult =
  | { kind: 'model'; model: unknown; warnings: string }
  | { kind: 'error'; message: string; binaryMissing: boolean };

/** A model can be large, but not this large. */
const MAX_OUTPUT_BYTES = 64 * 1024 * 1024;

export function analyze(request: AnalyzeRequest): Promise<AnalyzeResult> {
  return new Promise((resolve) => {
    execFile(
      request.binary,
      ['generate', '--src', request.src, '--name', request.name],
      { maxBuffer: MAX_OUTPUT_BYTES },
      (error, stdout, stderr) => {
        if (error) {
          const binaryMissing = (error as NodeJS.ErrnoException).code === 'ENOENT';
          resolve({
            kind: 'error',
            message: binaryMissing ? String(error.message) : stderr.trim() || String(error.message),
            binaryMissing,
          });
          return;
        }
        try {
          resolve({ kind: 'model', model: JSON.parse(stdout), warnings: stderr.trim() });
        } catch {
          resolve({
            kind: 'error',
            message: `unparseable analyzer output: ${stdout.slice(0, 200)}`,
            binaryMissing: false,
          });
        }
      },
    );
  });
}
