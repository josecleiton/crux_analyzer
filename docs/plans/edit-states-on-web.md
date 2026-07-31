# Visão de edição de estados (proposta de mudança) — v1

## Tese
Adicionar uma **camada de proposta** no app web (que cai de graça no VS Code, que embarca o bundle). O usuário edita uma *cópia* do modelo carregado e a ferramenta gera um **briefing de mudança** (Markdown, + Rust scaffolding + JSON) que devs/LLMs implementam no código-fonte. **Não há write-back** para código/Rust — preserva a regra de que o modelo é uma projeção read-only do analisado. Nenhum crate Rust é tocado; tudo é novo em `apps/web/`.

## Por que este formato (e não gerar código)
- O modelo é *lossy*: não reconstrói `update`, helpers, ramos. Gerar Rust quebraria a honestidade do parser e a tese do projeto.
- O briefing como *instrução* (não como código final) é exatamente "fácil de implementar por devs e LLMs".
- Capacidades permanecem um campo do efeito (livre + sugestões); primeira-classe é v2.

---

## Escopo v1 (preciso)
- **Adicionar/editar/remover efeitos** em transições existentes (o pedido central — "trazer os efeitos para os estados").
- **Criar/remover transições** (necessário para anexar um efeito a um estado sem chegada) e **editar campos** `from`/`event`/`to` (limitado a estados/eventos já existentes, p/ sanidade).
- **Editar doc/markers/tags** de estados existentes (barato, torna a proposta um doc de mudança real).
- **Wildcards:** transições wildcard (`* → evt → B`) são **somente leitura** na v1; o usuário pode ver seus efeitos mas não editar (a semântica de "todos os estados" complica o briefing). Edição de wildcards é v2.
- **Fora do escopo (v2):** criar/remover/renomear estados; capacidades como entidade primeira-classe; edição de transições wildcard.

## UX
- **Entrada:** toggle "Propor mudanças" no Toolbar (i18n). **Mutuamente exclusivo com Simulation** — ativar um desativa o outro; o botão do modo inativo fica disabled com tooltip explicativo.
- **Modo proposta ativo:** o Inspector vira superfície de edição; o canvas troca a fatia de dados pela *proposta projetada* (o `Graph` permanece renderizador puro por props — regra dura mantida).
- **Diferenciação visual no canvas:** nodes/edges anotados com `changeType: "added" | "modified" | "removed" | null`. O Graph usa CSS para indicar mudanças (borda verde = added, borda âmbar = modified, borda vermelha tracejada + opacidade = removed). Isso **não viola** "Graph puro por props" — é mais um campo de dados.
- **Estado selecionado:** mostra doc/markers/tags editáveis; "Effects on entry" vira a união *editável* — cada transição de chegada com seus efeitos editáveis; "adicionar efeito" pergunta "em qual chegada?" (honesto: efeitos são da transição). Transições wildcard aparecem read-only com badge "(wildcard — read-only)".
- **Transição selecionada:** editor direto de efeitos (adicionar/editar/remover).
- **Sidebar:** estados ocultos via `hiddenStateIds` permanecem ocultos no modo proposta. Se o usuário oculta um estado que tem transições editadas, as edições são preservadas (visíveis no ReviewPanel) mas não aparecem no canvas.
- **Undo/redo:** Ctrl+Z / Ctrl+Shift+Z (Cmd no macOS). Stack de operações com limite de 50 entradas. Cada operação atômica (add-effect, remove-transition, edit-doc) é um item na stack.
- **Painel "Revisar mudanças":** lista agrupada (added/removed/modified) + nota livre + botões **Copiar briefing Markdown / Copiar scaffolding Rust / Copiar JSON / Exportar**. É onde a "ideia clara do que mudou" aparece.

## Arquitetura (respeita o fluxo em camadas)

```
Parser JSON → fromParserJson → DomainProject
                                  └→ DomainCore (active)
                                       └→ applyProposal(core, proposal)
                                            └→ DomainCore (projetado)
                                                 └→ toFlowModel(core, labels, hidden)
                                                      └→ FlowModel → ElkLayout → Graph
```

Sem mudar `Graph`. A proposta opera no nível de `DomainCore` (não `DomainProject`), porque `toFlowModel` recebe `DomainCore`.

Lógica de proposta em `apps/web/src/proposal/` (pura, vitest). Nada de Rust/schema é alterado — a proposta é um formato de rascunho do cliente, não parte do contrato serializado.

## Tipos principais (shape)

```typescript
/** Uma operação atômica na proposta. Cada uma é um item na undo stack. */
type ProposalOp =
  | { kind: "add-effect"; transitionId: string; effect: EffectDraft }
  | { kind: "edit-effect"; transitionId: string; index: number; effect: EffectDraft }
  | { kind: "remove-effect"; transitionId: string; index: number }
  | { kind: "add-transition"; transition: TransitionDraft }
  | { kind: "remove-transition"; transitionId: string }
  | { kind: "edit-transition"; transitionId: string; fields: Partial<Pick<TransitionDraft, "from" | "event" | "to">> }
  | { kind: "edit-state-doc"; stateId: string; doc: string | undefined }
  | { kind: "edit-state-markers"; stateId: string; markers: StateMarker[] }
  | { kind: "edit-state-tags"; stateId: string; tags: string[] };

interface EffectDraft {
  name: string;
  capability?: string;
  answers: string[];       // callback events do shell
  conditional: boolean;    // "may request"
}

interface TransitionDraft {
  from: string;   // estado existente (validado)
  event: string;  // evento existente (validado)
  to: string;     // estado existente (validado)
  effects: EffectDraft[];
}

/** Estado completo da proposta. */
interface Proposal {
  /** ID do core alvo. */
  coreId: string;
  /** Operações aplicadas (em ordem). */
  ops: ProposalOp[];
  /** Cursor na undo stack (ops[0..undoCursor] estão ativas). */
  undoCursor: number;
  /** Hash do DomainCore base no momento da criação. */
  baseHash: string;
  /** Nota livre do usuário (aparece no briefing). */
  note: string;
}

/** Resultado do diff entre base e projetado. */
interface ChangeSet {
  machines: MachineChange[];
}

interface MachineChange {
  machineName: string;
  transitions: {
    added: DomainTransition[];
    removed: DomainTransition[];
    modified: { key: TransitionKey; effectsAdded: DomainEffect[]; effectsRemoved: DomainEffect[] }[];
  };
  states: {
    modified: { stateId: string; field: "doc" | "markers" | "tags"; before: unknown; after: unknown }[];
  };
}

type TransitionKey = { from: string; event: string; to: string };
```

A proposta é **operations-based**: composável, undo/redo natural (mover `undoCursor`), e `applyProposal` é um reducer que aplica `ops[0..undoCursor]` sobre o core base. O changeset é computado por **diff do modelo projetado** (não derivado das ops) para garantir que reflete o estado real, independente de operações redundantes.

## Identidade para o diff
Transições são identidade por `(from, event, to)` (é o que o parser deduplica). Mesma chave com efeitos diferentes = "transição modificada" (diff de efeitos added/removed).

## Re-layout
Operações que alteram a topologia do grafo (add/remove transition, add/remove effect que muda label width) disparam re-layout via ELK **debounced** (300ms após última operação). Operações que não alteram topologia (edit-doc, edit-tags, edit-markers) não disparam re-layout.

## Validação
Regras que `applyProposal` garante (ops inválidas são rejeitadas silenciosamente — a UI não deve permitir criá-las):
- `from`, `to` devem referenciar estados existentes no core base (não wildcards na v1).
- `event` deve referenciar um evento existente no catálogo do core (`core.eventDocs`).
- `name` de efeito não pode ser string vazia.
- Self-loops (`from === to`) são **permitidos** (Crux os permite).
- Efeitos duplicados (mesmo `name` + `capability`) na mesma transição são **rejeitados**.
- Após staleness detectada: edição é **bloqueada** até o usuário descartar a proposta ou rebasear (aplicar ops sobre o novo core, descartando ops que referenciam entidades removidas).

## Staleness
- **Assinatura:** hash SHA-256 do JSON serializado do `DomainCore` (determinístico — campos ordenados).
- **Trigger:** ao receber novo `DomainProject` via `postMessage` (VS Code) ou reload (web), comparar hash do core ativo com `proposal.baseHash`.
- **Ação:** se diferir, exibir banner de staleness no Toolbar. Edição bloqueada. Duas ações: "Descartar proposta" ou "Rebasear" (re-aplica ops válidas sobre o novo core, descartando as que referenciam entidades removidas, e atualiza `baseHash`).

---

## Arquivos novos
- `apps/web/src/proposal/types.ts` — `Proposal`, `ProposalOp`, `EffectDraft`, `TransitionDraft`, `ChangeSet`, `MachineChange`, `TransitionKey`.
- `apps/web/src/proposal/apply.ts` — `applyProposal(base: DomainCore, proposal: Proposal): DomainCore` (modelo projetado; imutável; **re-indexa `incoming`/`outgoing`** nos estados afetados).
- `apps/web/src/proposal/diff.ts` — `computeChangeSet(base: DomainCore, projected: DomainCore): ChangeSet` (added/removed/modified, diff de efeitos). Testado.
- `apps/web/src/proposal/briefing.ts` — `generateBriefing(changeSet, locale, note): string` Markdown de *instrução* agrupado por máquina; honra idioma no chrome; identificadores escapados e verbatim (nunca traduzidos).
- `apps/web/src/proposal/scaffolding.ts` — `generateScaffolding(changeSet, locale): string` snippets Rust (variante de enum, esqueleto de match arm) — **template-based**, claramente marcado como sugestão (não código compilável).
- `apps/web/src/proposal/serialize.ts` — change-set → JSON (companion p/ tooling/CI/futuro `diff`).
- `apps/web/src/proposal/storage.ts` — `localStorage` (load/save/discard); chave: `crux-proposal:${coreId}`; auto-save debounced (1s). Hash do base para staleness.
- `apps/web/src/proposal/useProposal.ts` — custom hook que encapsula toda a lógica: `const { proposal, dispatch, projectedCore, changeSet, isDirty, isStale, undo, redo, canUndo, canRedo, discard, rebase } = useProposal(baseCore)`. Internamente usa `useReducer`. Mantém `App.tsx` enxuto.
- `apps/web/src/proposal/annotate.ts` — `annotateFlowModel(baseModel: FlowModel, projectedModel: FlowModel): FlowModel` — compara e anota nodes/edges com `changeType` para diferenciação visual no canvas.
- `apps/web/src/components/Proposal/ReviewPanel.tsx` — revisão + ações de exportar/copiar.
- `apps/web/src/components/Inspector/EffectEditor.tsx` — editor inline de efeitos/transição no Inspector.
- `apps/web/src/proposal/*.test.ts` — diff, briefing, scaffolding, apply, annotate, e **hostile-input** (nome de efeito com `<`/`&` não quebra o briefing — security.md).

## Arquivos editados
- `apps/web/src/App.tsx` — integrar `useProposal`; `displayedCore` derivado (`isProposing ? projectedCore : activeCore`) alimenta `toFlowModel`/layout; anotar FlowModel via `annotateFlowModel`; abrir `ReviewPanel`; exclusão mútua com simulation; staleness banner.
- `apps/web/src/components/Inspector/Inspector.tsx` — renderizar controles de edição quando em modo proposta; marcar transições wildcard como read-only.
- `apps/web/src/components/Toolbar/Toolbar.tsx` — toggle "Propor mudanças" / "Revisar"; disable "Simulate" durante proposta (e vice-versa); badge de contagem de mudanças; undo/redo buttons.
- `apps/web/src/flow/toFlowModel.ts` — aceitar campo opcional `changeType` no data de nodes/edges (ou o `annotate.ts` pós-processa o FlowModel — decisão de implementação).
- `apps/web/src/i18n/messages/{en,pt-BR}.ts` — novas chaves (chrome; identificadores seguem verbatim).
- CSS existente (seguir padrão `apps/web`) + novos estilos para: modo proposta ativo (indicador no toolbar), campos editáveis no Inspector, ReviewPanel, badges de changeType no canvas (`.flow-node--added`, `.flow-edge--removed`, etc.).
- `docs/web-ui.md` + `docs/pt-BR/web-ui.md` + entrada em `docs/roadmap.md` (+ pt-BR twin) e no short-form do `CLAUDE.md`.

## Regras mantidas (explícito)
- **Graph puro por props** ✓ (só trocamos a fatia de modelo que o alimenta; `changeType` é mais um campo de dados).
- **Fluxo em camadas** ✓ (proposta vive no domínio; não cruza para o parser/schema).
- **i18n** ✓ (todo chrome via catálogos; identificadores nunca traduzidos).
- **Segurança** ✓ (escapar identificadores no Markdown gerado; nunca viram markup).
- **Honestidade do parser** ✓ (parser intacto; a proposta é explicitamente *proposta*, e o briefing sinaliza lacunas — ex.: efeito sem capability → "confirme a capability").
- **Sem write-back / sem crates Rust / sem mudar o schema.**
- **Acessibilidade** ✓ (controles de edição seguem padrão WAI-ARIA de formulários inline; foco move para o primeiro campo editável ao ativar modo proposta; undo/redo acessíveis por keyboard shortcuts).

## Ordem de implementação sugerida

```
 1. proposal/types.ts            — fundação de tipos
 2. proposal/apply.ts + tests    — reducer core (com re-index incoming/outgoing)
 3. proposal/diff.ts + tests     — changeset
 4. proposal/annotate.ts + tests — anotação visual
 5. proposal/briefing.ts + tests — geração de briefing
 6. proposal/scaffolding.ts + tests
 7. proposal/serialize.ts
 8. proposal/storage.ts
 9. proposal/useProposal.ts      — hook que une tudo
10. Inspector/EffectEditor.tsx    — UI de edição
11. Toolbar toggle + undo/redo   — entrada no modo
12. App.tsx integração            — wiring final
13. Proposal/ReviewPanel.tsx      — painel de revisão
14. annotate → toFlowModel CSS   — visual diff no canvas
15. i18n + docs + CLAUDE.md
```

## v2 (registrado, fora deste plano)
- Estados primeira-classe (criar/remover/renomear, mudar initial).
- Capacidades como entidade própria (pick de capabilities reais, listar seus efeitos).
- Edição de transições wildcard.
- Rebase automático (sem intervenção do usuário).

## Verificação
- `just web-test` (vitest: proposal/diff, briefing, scaffolding, apply, annotate, hostile-input) + `just check`.
- Confirmação manual: `just dev`, propor uma mudança no fixture, copiar o briefing, validar legibilidade p/ humano e LLM; testar no VS Code (`just ext-build`) que o modo proposta aparece na webview e o rascunho sobrevive a regenerar-on-save (com banner de staleness).