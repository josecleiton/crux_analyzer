import type { ChangeSet } from './types';

/**
 * Generates template-based Rust code snippets as implementation suggestions.
 */
export function generateScaffolding(changeSet: ChangeSet): string {
  if (!changeSet || changeSet.totalChanges === 0) {
    return '// No scaffolding required (no changes proposed).';
  }

  const lines: string[] = [];
  lines.push('// =========================================================================');
  lines.push('// Rust Scaffolding Suggestions (Templates)');
  lines.push('// NOTE: These snippets are illustrative templates to assist manual implementation.');
  lines.push('// =========================================================================');
  lines.push('');

  for (const machine of changeSet.machines) {
    lines.push(`// --- Machine: ${machine.machineName} ---`);
    lines.push('');

    // Added event variants
    if (machine.transitions.added.length > 0) {
      lines.push('// Event Enum Variants to add:');
      lines.push('/*');
      for (const t of machine.transitions.added) {
        lines.push(`  /// Trigger transition from ${t.fromName} to ${t.toName}`);
        lines.push(`  ${t.event},`);
      }
      lines.push('*/');
      lines.push('');
    }

    // Effect handling arms
    const addedEffects: Array<{ event: string; effectName: string; cap?: string }> = [];
    for (const t of machine.transitions.added) {
      for (const eff of t.effects) {
        addedEffects.push({ event: t.event, effectName: eff.name, cap: eff.capability });
      }
    }
    for (const t of machine.transitions.modified) {
      for (const eff of t.effectsAdded) {
        addedEffects.push({ event: t.key.event, effectName: eff.name, cap: eff.capability });
      }
    }

    if (addedEffects.length > 0) {
      lines.push('// Match arms for update / effect requests:');
      lines.push('/*');
      for (const item of addedEffects) {
        const capComment = item.cap ? ` // Capability: ${item.cap}` : '';
        lines.push(`  Event::${item.event} => {`);
        lines.push(`      // Request effect: ${item.effectName}${capComment}`);
        lines.push(`      model.render(); // or render_with(...) / request(...)`);
        lines.push(`  }`);
      }
      lines.push('*/');
      lines.push('');
    }
  }

  return lines.join('\n');
}
