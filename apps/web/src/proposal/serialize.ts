import type { ChangeSet, Proposal } from './types';

/**
 * Serializes a ChangeSet to a formatted JSON string.
 */
export function serializeChangeSet(changeSet: ChangeSet): string {
  return JSON.stringify(changeSet, null, 2);
}

/**
 * Serializes a Proposal to a formatted JSON string.
 */
export function serializeProposal(proposal: Proposal): string {
  return JSON.stringify(proposal, null, 2);
}

/**
 * Deserializes a Proposal from a JSON string.
 */
export function deserializeProposal(jsonString: string): Proposal | null {
  try {
    const parsed = JSON.parse(jsonString);
    if (parsed && typeof parsed.coreId === 'string' && Array.isArray(parsed.ops)) {
      return parsed as Proposal;
    }
  } catch {
    // Ignore parse error
  }
  return null;
}
