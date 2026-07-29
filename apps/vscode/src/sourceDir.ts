/**
 * Which directory to analyze. Pure — the caller supplies the existence
 * check — so the fallback order is unit-testable.
 *
 * An explicit setting always wins, even if the directory is missing: the
 * analyzer's own error is more honest than silently analyzing somewhere
 * else. With no setting, the conventional Crux layout (`shared/src`) is
 * tried first, then a plain `src`.
 */

export function resolveSourceDir(
  configured: string,
  workspaceRoot: string,
  exists: (path: string) => boolean,
): string | null {
  const join = (base: string, relative: string) =>
    `${base.replace(/[/\\]+$/, '')}/${relative.replace(/^[/\\]+/, '')}`;

  const explicit = configured.trim();
  if (explicit !== '') return join(workspaceRoot, explicit);

  for (const candidate of ['shared/src', 'src']) {
    const path = join(workspaceRoot, candidate);
    if (exists(path)) return path;
  }
  return null;
}
