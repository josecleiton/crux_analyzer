import type { DomainCore } from '../domain/types';
import { deserializeProposal, serializeProposal } from './serialize';
import type { Proposal } from './types';

const STORAGE_PREFIX = 'crux-proposal:';

/**
 * Computes a simple deterministic hash string for a DomainCore base object.
 */
export function computeCoreHash(core: DomainCore): string {
  const payload = {
    id: core.id,
    name: core.name,
    machines: core.machines.map((m) => ({
      id: m.id,
      name: m.name,
      states: m.states.map((s) => s.id),
      transitions: m.transitions.map((t) => `${t.from}->${t.event}->${t.to}`),
    })),
  };
  const str = JSON.stringify(payload);
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash |= 0; // Convert to 32bit integer
  }
  return `h_${Math.abs(hash).toString(16)}`;
}

export function loadProposal(coreId: string): Proposal | null {
  try {
    const raw = localStorage.getItem(`${STORAGE_PREFIX}${coreId}`);
    if (!raw) return null;
    return deserializeProposal(raw);
  } catch {
    return null;
  }
}

export function saveProposal(proposal: Proposal): void {
  try {
    const serialized = serializeProposal(proposal);
    localStorage.setItem(`${STORAGE_PREFIX}${proposal.coreId}`, serialized);
  } catch {
    // Storage quota or error
  }
}

export function discardProposal(coreId: string): void {
  try {
    localStorage.removeItem(`${STORAGE_PREFIX}${coreId}`);
  } catch {
    // Ignore error
  }
}
