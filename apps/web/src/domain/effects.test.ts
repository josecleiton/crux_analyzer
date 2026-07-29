import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from './../schema/parserJson';
import { fromParserJson } from './fromParserJson';
import { entryEffects } from './effects';

const project = fromParserJson(parseProjectJson(rawProject));
const recorder = project.cores[0].machines[0];

const names = (state: Parameters<typeof entryEffects>[0]) =>
  entryEffects(state).map((effect) => effect.name);

describe('entryEffects', () => {
  it('unions the effects of the incoming transitions, deduplicated', () => {
    // Uploading is entered by StopPressed (Stop + Upload); the duplicate
    // arrival path must not repeat the effects.
    const uploading = recorder.states.find((s) => s.name === 'Uploading')!;
    expect(names(uploading)).toEqual(['AudioOperation::Stop', 'HttpOperation::Upload']);
  });

  it('keeps first-seen order across different incoming transitions', () => {
    const recording = recorder.states.find((s) => s.name === 'Recording')!;
    // entered by RecordPressed (Start) and ResumePressed (Resume)
    expect(names(recording)).toEqual(['AudioOperation::Start', 'AudioOperation::Resume']);
  });

  it('carries what the source declares around each request', () => {
    const uploading = recorder.states.find((s) => s.name === 'Uploading')!;
    const upload = entryEffects(uploading).find((e) => e.name === 'HttpOperation::Upload')!;
    expect(upload.capability).toBe('Http');
    expect(upload.answers).toEqual(['UploadFinished']);
    expect(upload.conditional).toBe(false);
  });

  it('pools the answers and keeps "may" only when every arrival says so', () => {
    // Switching is entered by one conditional request; Ready by an
    // unconditional one, so the same operation reads differently on each.
    const inputs = project.cores[0].machines[1];
    const switching = inputs.states.find((s) => s.name === 'Switching')!;
    const [switchInput] = entryEffects(switching);
    expect(switchInput.name).toBe('AudioOperation::SwitchInput');
    expect(switchInput.conditional).toBe(true);
    expect(switchInput.answers).toEqual(['InputSwitched', 'InputSwitchFailed']);
  });

  it('is empty for a state whose arrivals request nothing', () => {
    const idle = recorder.states.find((s) => s.name === 'Idle')!;
    expect(entryEffects(idle)).toEqual([]);
  });
});
