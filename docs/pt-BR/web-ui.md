# UI Web

> 🌐 [English](../web-ui.md) · **Português (Brasil)**

`apps/web` — React + TypeScript + React Flow + ELKJS. Inicie com `just dev` (ou
`pnpm --filter web dev`).

## O que ela mostra

Três áreas, no estilo LangGraph Studio:

- **Barra lateral** — os Cores do projeto. Selecionar um renderiza suas máquinas.
- **Canvas** — as máquinas de estado. Um core com várias máquinas (regiões
  ortogonais) renderiza cada uma como uma **seção titulada**; um core de máquina
  única renderiza plano. Todo estado é um nó, toda transição é uma aresta
  rotulada com seu evento. Transições curinga (`from`/`to` = `"*"`) se conectam a
  um pseudo-nó tracejado **qualquer estado**. Folhas de compostos aparecem como
  `Pai / Filho`. Clicar em uma seção (no título ou na área vazia) seleciona o
  **estado de entrada** da máquina e enquadra essa máquina na view, então uma
  máquina pode ser inspecionada — e simulada — em um clique.
- **Inspetor** (painel direito) — selecionar um estado mostra seus selos de papel
  e seus eventos de entrada/saída; selecionar uma transição mostra
  `evento: de ↓ para` mais os **efeitos** que ela solicita. A máquina proprietária
  é etiquetada quando o core tem mais de uma.

## Papéis dos estados

Os papéis são pintados no canvas o tempo todo, com ou sem simulação
(`src/domain/stateRole.ts`):

- **inicial** (azul, ponto preenchido antes do rótulo) — o ponto de entrada da
  máquina: um estado para o qual nada transiciona. Em uma máquina totalmente
  cíclica, o primeiro estado carrega o papel, que é onde a simulação começa. O
  primeiro estado com esse papel é o estado de entrada da máquina (`entryState`).
- **final** (violeta, borda dupla) — um beco sem saída: nenhuma transição de saída
  própria. Um curinga que valha para toda a máquina (`from: "*"`) ainda pode
  deixá-lo; essa fuga permanece visível como uma aresta partindo do nó **qualquer
  estado**.
- **falha** (vermelho) — uma heurística de nomenclatura, o único palpite dos três:
  as palavras do estado incluem uma palavra de falha (`Failed`, `Error`, `Denied`,
  `Rejected`, `Invalid`, `TimedOut`, …). Isso nunca chega ao parser, que não deve
  inventar semântica; um estado que é ao mesmo tempo falha e final mantém a borda
  dupla em vermelho.

O Inspetor e o painel de simulação repetem os papéis como selos.

## Fonte de dados

Ao carregar, a aplicação busca `model.json` relativo à sua base (veja
[Publicação estática](#publicação-estática) — `/model.json` em
desenvolvimento). Coloque um modelo gerado em `apps/web/public/model.json` —
`just model <src> <nome>`, ou `model-watch` para mantê-lo fresco. Sem um — ou
com um desatualizado/inválido — ela cai no exemplo embutido
(`shared/schema/examples/audio-recorder.json`) e registra um aviso no console.
O artefato está no gitignore.

## Publicação estática

A UI é um bundle estático, então publicá-la como documentação interna não exige
nenhuma lógica de servidor — apenas um host HTTP comum. Uma única recipe faz
tudo:

```sh
just site ../meu-app/shared/src MeuApp              # servido na raiz do domínio
just site ../meu-app/shared/src MeuApp /crux-docs/  # servido em um subcaminho
# depois publique apps/web/dist/
```

`site` analisa a crate para `apps/web/public/model.json` e só então faz o
build, de modo que o modelo vai *dentro* de `dist/` — a página publicada nunca
volta a chamar o analisador, e atualizar a documentação é rodar a recipe de
novo. Os dois passos ficam numa recipe só de propósito: fazer o build sem gerar
antes publica o exemplo embutido, o que parece um site funcionando em vez de um
erro.

O terceiro argumento é o `base` do Vite (`CRUX_BASE=<base>` para chamadas
diretas de `pnpm build`, normalizado em `vite.config.ts`). Ele é **obrigatório
sempre que o site não estiver na raiz do domínio** — GitHub/GitLab Pages por
projeto, por exemplo: as URLs dos assets e o fetch do `model.json` são
resolvidos a partir dele, e um build com caminhos absolutos servido em um
subcaminho falha silenciosamente no exemplo embutido. Uma origem completa
(`https://cdn.exemplo.com/docs/`) também funciona.

Duas coisas que não esperar: o bundle precisa ser servido por HTTP (`file://`
bloqueia tanto o módulo ES quanto o fetch do modelo), e nenhuma regra de
fallback de SPA é necessária — há uma única página e nenhum roteador.

## Simulação

Selecione um estado (opcional) e clique em **Simular**:

- o painel direito passa para a simulação: estado atual, os eventos que podem
  disparar a partir dele (os de origem curinga estão sempre disponíveis;
  transições com destino de tempo de execução `to: "*"` são excluídas do replay) e
  o histórico do que já disparou;
- o canvas se lê como um caminho, em três níveis de ênfase: tudo que já foi
  **percorrido** fica verde em negrito (estados e transições, incluindo o estado
  inicial), o que pode **disparar daqui** mantém um contorno verde, e todo o resto
  recua — incluindo as seções das outras máquinas;
- o estado atual e a última transição tomada são os mais fortes de todos, e o
  passo é animado: o traço da transição flui em tracejado com um pulso viajando
  pela rota, o estado que acabou de ser alcançado dá um salto e depois respira, e
  a nova entrada do histórico desliza para dentro;
- aterrissar em um estado de **falha** deixa todo esse destaque vermelho (aresta,
  rótulo, ponta da seta, anel), então caminhos de falha se destacam dos saudáveis;
- a view acompanha o replay: quando o estado recém-alcançado não está
  inteiramente visível, o canvas pana para centralizá-lo, sem mexer no zoom — um
  passo que cai na tela nunca move o canvas;
- **Reiniciar** volta ao primeiro estado da máquina; **Parar simulação** retorna
  ao inspetor.

Toda animação é suprimida sob `prefers-reduced-motion`, movimentos de view
incluídos (`src/components/Graph/ViewportFocus.tsx` lê a preferência em JS, já
que uma regra CSS não silencia uma animação por script).

O motor (`src/simulation/engine.ts`) é lógica de domínio pura; ele dirige o Graph
exclusivamente por props de destaque — `traveledPath` e `availableTransitions` são
os fatos, o Graph apenas os mapeia para níveis de ênfase.

## Temas

O alternador de tema na barra de ferramentas troca entre claro e escuro. O tema
ativo é o atributo `data-theme` no `<html>`; toda cor é uma propriedade CSS
customizada definida por tema em `src/index.css`, então adicionar um tema é um
único bloco `:root[data-theme='...']`. A escolha persiste em `localStorage` (um
script pré-pintura em `index.html` a aplica antes da primeira renderização — sem
flash), e sem escolha explícita a aplicação acompanha a preferência do sistema ao
vivo. Cores exclusivas de SVG (pontas de seta das arestas) são lidas de volta dos
mesmos tokens (`src/theme/theme.ts`), mantendo o CSS como única fonte da verdade.

## Localização

O alternador de idioma na barra de ferramentas troca entre inglês e português
(`en` / `pt-BR`); ele mostra o código curto do locale **ativo**, enquanto o
tooltip e o nome acessível dizem para qual idioma o clique leva. O
módulo (`src/i18n/`) espelha o tema de propósito: o locale ativo é o atributo
`data-locale` no `<html>` (com `lang` definido junto, para tecnologia assistiva),
a escolha persiste em `localStorage`, um script pré-pintura em `index.html` a
aplica antes da primeira renderização, e sem escolha explícita a aplicação segue
`navigator.languages` — qualquer português resolve para `pt-BR`.

Duas diferenças em relação aos temas valem ser conhecidas:

- as traduções chegam aos componentes por **contexto** (`I18nProvider` em
  `main.tsx`), não por props — todo painel precisa de `t`, enquanto apenas dois
  componentes precisam do tema;
- trocar de locale **refaz o layout**. As larguras dos nós são estimadas a partir
  do texto do rótulo, então o pseudo-nó traduzido `any state` / `qualquer estado`
  muda a geometria; `toFlowModel` recebe o rótulo como parâmetro `FlowLabels` em
  vez de importar o catálogo, mantendo as camadas de mapeamento livres de idioma.

Nomes de estados, eventos, efeitos, máquinas e cores nunca são traduzidos — são
identificadores da aplicação analisada. A separação monoespaçada/sem-serifa em
`index.css` reflete essa distinção. Veja [i18n.md](i18n.md).

## Layout

Toda a geometria vem da interface `LayoutEngine`
(`src/layout/LayoutEngine.ts`): posições dos nós, rotas ortogonais das arestas
com cantos arredondados, e a caixa que cada rótulo de aresta ocupa — o ELK as
calcula (`ElkLayoutEngine`, `elk.algorithm: layered` com rótulos de aresta
inline), então as arestas nunca cruzam nós e os rótulos nunca se sobrepõem. As
seções de máquina usam o layout hierárquico do ELK (nós de grupo do React Flow com
posições relativas dos filhos). Os nós não são arrastáveis — as rotas pertencem ao
motor; use **Reorganizar** para recalcular.

## Estendendo

- Nova visualização dos mesmos dados: consuma o modelo de domínio
  (`src/domain/types.ts`), não os tipos do React Flow.
- Novo algoritmo de layout: implemente `LayoutEngine` e troque em `App.tsx`.
- Nova funcionalidade dirigida por destaque (à la simulação): calcule ids e
  passe-os pela prop `highlight` do Graph — não modifique o Graph.
