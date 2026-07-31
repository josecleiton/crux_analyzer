import { describe, expect, it } from 'vitest';
import { generateScaffolding } from './scaffolding';
import type { ChangeSet } from './types';

describe('generateScaffolding', () => {
  it('generates Rust scaffolding template code', () => {
    const cs: ChangeSet = {
      coreId: 'c1',
      totalChanges: 1,
      machines: [
        {
          machineId: 'm1',
          machineName: 'AuthMachine',
          transitions: {
            added: [
              {
                fromName: 'LoggedOut',
                event: 'LoginRequest',
                toName: 'Authenticating',
                effects: [{ name: 'Auth::Login', capability: 'Auth', answers: [], conditional: false }],
              },
            ],
            removed: [],
            modified: [],
          },
          states: { modified: [] },
        },
      ],
    };

    const code = generateScaffolding(cs);
    expect(code).toContain('AuthMachine');
    expect(code).toContain('LoginRequest');
    expect(code).toContain('Auth::Login');
  });
});
