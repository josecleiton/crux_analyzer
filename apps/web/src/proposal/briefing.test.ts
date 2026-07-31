import { describe, expect, it } from 'vitest';
import { escapeMarkdown, generateBriefing } from './briefing';
import type { ChangeSet } from './types';

describe('escapeMarkdown', () => {
  it('escapes html tags and markdown syntax characters', () => {
    const input = '<script>alert("xss")</script> & [link](http://test)';
    const escaped = escapeMarkdown(input);
    expect(escaped).not.toContain('<script>');
    expect(escaped).toContain('&lt;script&gt;');
    expect(escaped).toContain('&amp;');
    expect(escaped).toContain('\\[link\\]');
  });
});

describe('generateBriefing', () => {
  it('generates briefing with hostile strings safely', () => {
    const hostileChangeSet: ChangeSet = {
      coreId: 'c1',
      totalChanges: 1,
      machines: [
        {
          machineId: 'm1',
          machineName: '<HostileMachine>',
          transitions: {
            added: [
              {
                fromName: 'Idle',
                event: 'Event<XSS>',
                toName: 'Active',
                effects: [
                  {
                    name: 'Effect&Malicious',
                    capability: 'Http<Cap>',
                    answers: [],
                    conditional: false,
                  },
                ],
              },
            ],
            removed: [],
            modified: [],
          },
          states: { modified: [] },
        },
      ],
    };

    const briefing = generateBriefing(hostileChangeSet, 'en');
    expect(briefing).toContain('&lt;HostileMachine&gt;');
    expect(briefing).toContain('Event&lt;XSS&gt;');
    expect(briefing).toContain('Effect&amp;Malicious');
    expect(briefing).not.toContain('<HostileMachine>');
  });

  it('handles state doc modification when previous doc was undefined without throwing', () => {
    const changeSet: ChangeSet = {
      coreId: 'c1',
      totalChanges: 1,
      machines: [
        {
          machineId: 'm1',
          machineName: 'Machine1',
          transitions: { added: [], removed: [], modified: [] },
          states: {
            modified: [
              {
                stateId: 's1',
                stateName: 'Idle',
                field: 'doc',
                before: undefined,
                after: 'Newly added documentation prose',
              },
            ],
          },
        },
      ],
    };

    expect(() => generateBriefing(changeSet, 'en')).not.toThrow();
    const briefing = generateBriefing(changeSet, 'en');
    expect(briefing).toContain('Before: `(none)`');
    expect(briefing).toContain('Newly added documentation prose');
  });
});
