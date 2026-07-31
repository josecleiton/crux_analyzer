import type { ChangeSet } from './types';

/**
 * Escapes characters that could alter Markdown formatting or introduce HTML/script tags.
 */
export function escapeMarkdown(str: string): string {
  if (typeof str !== 'string') {
    if (str === undefined || str === null) return '';
    str = String(str);
  }
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/`/g, '\\`')
    .replace(/\[/g, '\\[')
    .replace(/\]/g, '\\]');
}

/**
 * Generates an instruction Briefing in Markdown from a ChangeSet.
 */
export function generateBriefing(
  changeSet: ChangeSet,
  locale: 'en' | 'pt-BR' = 'en',
  userNote?: string
): string {
  if (!changeSet || changeSet.totalChanges === 0) {
    return locale === 'pt-BR'
      ? '# Briefing de Mudanças\n\nNenhuma mudança proposta.'
      : '# Change Briefing\n\nNo changes proposed.';
  }

  const lines: string[] = [];

  if (locale === 'pt-BR') {
    lines.push('# Briefing de Mudanças da State Machine');
    lines.push('');
    lines.push('Este documento instrui as alterações necessárias no código-fonte Rust.');
    lines.push('');
    if (userNote?.trim()) {
      lines.push('## Nota do Autor');
      lines.push(escapeMarkdown(userNote.trim()));
      lines.push('');
    }
    lines.push(`**Total de mudanças:** ${changeSet.totalChanges}`);
    lines.push('');

    for (const machine of changeSet.machines) {
      lines.push(`---`);
      lines.push(`## Máquina: \`${escapeMarkdown(machine.machineName)}\``);
      lines.push('');

      if (machine.transitions.added.length > 0) {
        lines.push('### Transições Adicionadas');
        for (const t of machine.transitions.added) {
          lines.push(
            `- De \`${escapeMarkdown(t.fromName)}\` em evento \`${escapeMarkdown(t.event)}\` para \`${escapeMarkdown(t.toName)}\``
          );
          if (t.effects.length > 0) {
            for (const eff of t.effects) {
              const capStr = eff.capability ? ` [Cap: \`${escapeMarkdown(eff.capability)}\`]` : '';
              const condStr = eff.conditional ? ' *(condicional)*' : '';
              lines.push(`  - Efeito: \`${escapeMarkdown(eff.name)}\`${capStr}${condStr}`);
            }
          }
        }
        lines.push('');
      }

      if (machine.transitions.removed.length > 0) {
        lines.push('### Transições Removidas');
        for (const t of machine.transitions.removed) {
          lines.push(
            `- Removida: \`${escapeMarkdown(t.fromName)}\` --[\`${escapeMarkdown(t.event)}\`]--> \`${escapeMarkdown(t.toName)}\``
          );
        }
        lines.push('');
      }

      if (machine.transitions.modified.length > 0) {
        lines.push('### Transições Modificadas');
        for (const t of machine.transitions.modified) {
          lines.push(
            `- Transição \`${escapeMarkdown(t.fromName)}\` --[\`${escapeMarkdown(t.key.event)}\`]--> \`${escapeMarkdown(t.toName)}\`:`
          );
          for (const eff of t.effectsAdded) {
            const capStr = eff.capability ? ` [Cap: \`${escapeMarkdown(eff.capability)}\`]` : '';
            lines.push(`  - **+ Adicionar efeito:** \`${escapeMarkdown(eff.name)}\`${capStr}`);
          }
          for (const eff of t.effectsRemoved) {
            lines.push(`  - **- Remover efeito:** \`${escapeMarkdown(eff.name)}\``);
          }
        }
        lines.push('');
      }

      if (machine.states.modified.length > 0) {
        lines.push('### Estados Modificados');
        for (const s of machine.states.modified) {
          lines.push(
            `- Estado \`${escapeMarkdown(s.stateName)}\` (campo \`${s.field}\` alterado)`
          );
          const beforeVal = s.before === undefined ? '(none)' : JSON.stringify(s.before);
          const afterVal = s.after === undefined ? '(none)' : JSON.stringify(s.after);
          lines.push(`  - Antes: \`${escapeMarkdown(beforeVal)}\``);
          lines.push(`  - Depois: \`${escapeMarkdown(afterVal)}\``);
        }
        lines.push('');
      }
    }
  } else {
    lines.push('# State Machine Change Briefing');
    lines.push('');
    lines.push('This document details requested changes to be implemented in Rust source code.');
    lines.push('');
    if (userNote?.trim()) {
      lines.push('## Author Note');
      lines.push(escapeMarkdown(userNote.trim()));
      lines.push('');
    }
    lines.push(`**Total changes:** ${changeSet.totalChanges}`);
    lines.push('');

    for (const machine of changeSet.machines) {
      lines.push(`---`);
      lines.push(`## Machine: \`${escapeMarkdown(machine.machineName)}\``);
      lines.push('');

      if (machine.transitions.added.length > 0) {
        lines.push('### Added Transitions');
        for (const t of machine.transitions.added) {
          lines.push(
            `- From \`${escapeMarkdown(t.fromName)}\` on event \`${escapeMarkdown(t.event)}\` to \`${escapeMarkdown(t.toName)}\``
          );
          if (t.effects.length > 0) {
            for (const eff of t.effects) {
              const capStr = eff.capability ? ` [Cap: \`${escapeMarkdown(eff.capability)}\`]` : '';
              const condStr = eff.conditional ? ' *(conditional)*' : '';
              lines.push(`  - Effect: \`${escapeMarkdown(eff.name)}\`${capStr}${condStr}`);
            }
          }
        }
        lines.push('');
      }

      if (machine.transitions.removed.length > 0) {
        lines.push('### Removed Transitions');
        for (const t of machine.transitions.removed) {
          lines.push(
            `- Removed: \`${escapeMarkdown(t.fromName)}\` --[\`${escapeMarkdown(t.event)}\`]--> \`${escapeMarkdown(t.toName)}\``
          );
        }
        lines.push('');
      }

      if (machine.transitions.modified.length > 0) {
        lines.push('### Modified Transitions');
        for (const t of machine.transitions.modified) {
          lines.push(
            `- Transition \`${escapeMarkdown(t.fromName)}\` --[\`${escapeMarkdown(t.key.event)}\`]--> \`${escapeMarkdown(t.toName)}\`:`
          );
          for (const eff of t.effectsAdded) {
            const capStr = eff.capability ? ` [Cap: \`${escapeMarkdown(eff.capability)}\`]` : '';
            lines.push(`  - **+ Add effect:** \`${escapeMarkdown(eff.name)}\`${capStr}`);
          }
          for (const eff of t.effectsRemoved) {
            lines.push(`  - **- Remove effect:** \`${escapeMarkdown(eff.name)}\``);
          }
        }
        lines.push('');
      }

      if (machine.states.modified.length > 0) {
        lines.push('### Modified States');
        for (const s of machine.states.modified) {
          lines.push(
            `- State \`${escapeMarkdown(s.stateName)}\` (field \`${s.field}\` changed)`
          );
          const beforeVal = s.before === undefined ? '(none)' : JSON.stringify(s.before);
          const afterVal = s.after === undefined ? '(none)' : JSON.stringify(s.after);
          lines.push(`  - Before: \`${escapeMarkdown(beforeVal)}\``);
          lines.push(`  - After: \`${escapeMarkdown(afterVal)}\``);
        }
        lines.push('');
      }
    }
  }

  return lines.join('\n');
}
