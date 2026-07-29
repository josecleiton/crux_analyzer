/**
 * Brazilian Portuguese.
 *
 * Typed as `Catalog`, so `tsc` rejects this file if a key from `en.ts` is
 * missing or misspelled — that type check is the parity guarantee.
 */

import type { Catalog } from './en';

export const ptBR: Catalog = {
  'app.loading': 'Carregando…',

  'toolbar.simulate': 'Simular',
  'toolbar.stopSimulation': 'Parar simulação',
  'toolbar.relayout': 'Reorganizar',

  'themeToggle.switchToLight': 'Mudar para o modo claro',
  'themeToggle.switchToDark': 'Mudar para o modo escuro',
  'localeToggle.switchTo': 'Mudar para {language}',

  'sidebar.cores': 'Núcleos',

  'inspector.title': 'Inspetor',
  'inspector.empty': 'Selecione um estado ou uma transição.',
  'inspector.incoming': 'Entradas',
  'inspector.outgoing': 'Saídas',
  'inspector.effects': 'Efeitos',
  'inspector.none': '—',

  'badge.initial': 'inicial',
  'badge.failure': 'falha',
  'badge.final': 'final',

  'state.anyState': 'qualquer estado',
  'state.anyStateRuntime': 'qualquer estado (em tempo de execução)',

  'simulation.title': 'Simulação',
  'simulation.unknownState': '?',
  'simulation.sendEvent': 'Enviar evento',
  'simulation.noEvents': 'Nenhum evento pode ser disparado a partir daqui.',
  'simulation.trail': 'Histórico',
  'simulation.nothingFired': 'Nada foi disparado ainda.',
  'simulation.restart': 'Reiniciar',

  'graph.a11y.controls': 'Painel de controle',
  'graph.a11y.zoomIn': 'Aproximar',
  'graph.a11y.zoomOut': 'Afastar',
  'graph.a11y.fitView': 'Ajustar à tela',
};
