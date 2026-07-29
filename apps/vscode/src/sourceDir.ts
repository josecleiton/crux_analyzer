/**
 * Which directory to analyze. Pure — the caller supplies the existence
 * check — so the fallback order is unit-testable.
 *
 * An explicit setting always wins, even if the directory is missing: the
 * analyzer's own error is more honest than silently analyzing somewhere
 * else. With no setting, the conventional Crux layout (`shared/src`) is
 * tried first, then a plain `src`.
 *
 * The setting is *contained* to the workspace root. `cruxAnalyzer.src` is
 * workspace-scoped, so a cloned repository's `.vscode/settings.json` chooses its
 * value; `../../../etc` would point the analyzer — and the file watcher — at a
 * directory the user never opened. Escaping the root returns `null`, which the
 * caller reports like any other unresolved directory. See `docs/security.md`.
 */

/** Splits on both separators and resolves `.` / `..` without touching the disk. */
function normalizeSegments(path: string): string[] {
  const out: string[] = [];
  for (const segment of path.split(/[/\\]+/)) {
    if (segment === '' || segment === '.') continue;
    if (segment === '..') {
      // An escape past the root is not representable, and the caller treats
      // `null` as "no source directory".
      if (out.length === 0) return [];
      out.pop();
      continue;
    }
    out.push(segment);
  }
  return out;
}

/**
 * Whether `configured` stays inside the workspace.
 *
 * A leading separator is *not* an escape: `/lib/core` has always been read as
 * workspace-relative, and re-rooting it keeps it inside. Only `..` can leave,
 * so only `..` is rejected — and only when it actually escapes, so
 * `shared/../src` stays valid.
 */
export function isContained(configured: string): boolean {
  let depth = 0;
  for (const segment of configured.trim().split(/[/\\]+/)) {
    if (segment === '' || segment === '.') continue;
    if (segment === '..') {
      depth -= 1;
      if (depth < 0) return false;
      continue;
    }
    depth += 1;
  }
  return true;
}

export function resolveSourceDir(
  configured: string,
  workspaceRoot: string,
  exists: (path: string) => boolean,
): string | null {
  const base = workspaceRoot.replace(/[/\\]+$/, '');
  const join = (relative: string) => {
    const segments = normalizeSegments(relative);
    return segments.length === 0 ? base : `${base}/${segments.join('/')}`;
  };

  const explicit = configured.trim();
  if (explicit !== '') {
    return isContained(explicit) ? join(explicit) : null;
  }

  for (const candidate of ['shared/src', 'src']) {
    const path = join(candidate);
    if (exists(path)) return path;
  }
  return null;
}
