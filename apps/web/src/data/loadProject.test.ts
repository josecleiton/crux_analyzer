import { afterEach, describe, expect, it, vi } from 'vitest';
import bundledExample from '../../../../shared/schema/examples/audio-recorder.json';
import { loadProject } from './loadProject';

// Node has no `window`; the tests stand one in to play the embedding host.
function inject(model: unknown) {
  (globalThis as { window?: unknown }).window = { __CRUX_MODEL__: model };
}

afterEach(() => {
  delete (globalThis as { window?: unknown }).window;
  vi.restoreAllMocks();
});

describe('loadProject with an injected model', () => {
  it('prefers the injected model over any other source', async () => {
    inject({ ...bundledExample, project: 'Injected By Host' });
    const project = await loadProject();
    expect(project.name).toBe('Injected By Host');
  });

  it('falls through the normal ladder when the injection is invalid', async () => {
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    inject({ not: 'a model' });
    // no server in this environment, so the fetch step yields nothing and
    // the bundled example is the end of the ladder
    const project = await loadProject();
    expect(project.name).toBe(bundledExample.project);
    expect(console.warn).toHaveBeenCalledOnce();
  });

  it('ignores the global entirely when the host injects nothing', async () => {
    (globalThis as { window?: unknown }).window = {};
    const project = await loadProject();
    expect(project.name).toBe(bundledExample.project);
  });
});
