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
  'toolbar.filterByTag': 'Filtrar por etiqueta',
  'toolbar.undocumented': 'Sem documentação',
  'toolbar.undocumentedHint':
    'Destaca estados sem documentação — os que não têm descrição /// no código-fonte analisado',

  'themeToggle.switchToLight': 'Mudar para o modo claro',
  'themeToggle.switchToDark': 'Mudar para o modo escuro',
  'localeToggle.switchTo': 'Mudar para {language}',

  'sidebar.cores': 'Núcleos',
  'sidebar.expand': 'Expandir',
  'sidebar.collapse': 'Recolher',
  'sidebar.hideFromCanvas': 'Ocultar {name} do diagrama',
  'sidebar.showOnCanvas': 'Mostrar {name} no diagrama',
  'sidebar.showAll': 'Mostrar todos os estados',
  'sidebar.sourceCode': 'Código-fonte no GitHub',

  'inspector.title': 'Inspetor',
  'inspector.empty': 'Selecione um estado ou uma transição.',
  'inspector.incoming': 'Entradas',
  'inspector.outgoing': 'Saídas',
  'inspector.effects': 'Efeitos',
  'inspector.entryEffects': 'Efeitos ao entrar',
  'inspector.answersWith': 'responde com',
  'inspector.conditional': 'pode',
  'inspector.none': '—',
  'inspector.tags': 'Etiquetas',
  'inspector.aboutMachine': 'Sobre esta máquina',

  'badge.initial': 'inicial',
  'badge.failure': 'falha',
  'badge.deprecated': 'descontinuado',
  'badge.final': 'final',

  'state.anyState': 'qualquer estado',
  'state.anyStateRuntime': 'qualquer estado (em tempo de execução)',

  'simulation.title': 'Simulação',
  'simulation.unknownState': '?',
  'simulation.sendEvent': 'Enviar evento',
  'simulation.noEvents': 'Nenhum evento pode ser disparado a partir daqui.',
  'simulation.runtimeTargetNote':
    'Estes levam a um estado decidido em tempo de execução, então o replay não consegue segui-los.',
  'simulation.inFlight': 'Aguardando o shell',
  'simulation.fromShell': 'do shell',
  'simulation.inertAnswerNote':
    'O shell pode enviar estes de volta, e nenhuma transição daqui os trata: comportamento real que não muda estado.',
  'simulation.trail': 'Histórico',
  'simulation.requested': 'solicitou',
  'simulation.mayHave': 'pode ter',
  'simulation.aheadNote':
    'Passos que o replay deixou para trás. Eles ficam guardados, não dados: disparar o mesmo evento de novo entra neles, e um movimento diferente os substitui.',
  'simulation.nothingFired': 'Nada foi disparado ainda.',
  'simulation.restart': 'Reiniciar',

  'graph.a11y.controls': 'Painel de controle',
  'graph.a11y.zoomIn': 'Aproximar',
  'graph.a11y.zoomOut': 'Afastar',
  'graph.a11y.fitView': 'Ajustar à tela',
};
