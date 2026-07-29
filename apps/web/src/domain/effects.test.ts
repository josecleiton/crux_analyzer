import { describe, expect, it } from 'vitest';
import rawProject from '../../../../shared/schema/examples/audio-recorder.json';
import { parseProjectJson } from './../schema/parserJson';
import { fromParserJson } from './fromParserJson';
import { entryEffects } from './effects';

const project = fromParserJson(parseProjectJson(rawProject));
const recorder = project.cores[0].machines[0];

describe('entryEffects', () => {
  it('unions the effects of the incoming transitions, deduplicated', () => {
    // Uploading is entered by StopPressed (Stop + Upload); the duplicate
    // arrival path must not repeat the effects.
    const uploading = recorder.states.find((s) => s.name === 'Uploading')!;
    expect(entryEffects(uploading)).toEqual(['AudioOperation::Stop', 'HttpOperation::Upload']);
  });

  it('keeps first-seen order across different incoming transitions', () => {
    const recording = recorder.states.find((s) => s.name === 'Recording')!;
    // entered by RecordPressed (Start) and ResumePressed (Resume)
    expect(entryEffects(recording)).toEqual(['AudioOperation::Start', 'AudioOperation::Resume']);
  });

  it('is empty for a state whose arrivals request nothing', () => {
    const idle = recorder.states.find((s) => s.name === 'Idle')!;
    expect(entryEffects(idle)).toEqual([]);
  });
});
