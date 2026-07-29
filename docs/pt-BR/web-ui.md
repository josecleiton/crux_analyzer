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
  um pseudo-nó tracejado **qualquer estado**. Um **estado composto** renderiza
  como um contêiner segurando suas folhas — o mesmo aninhamento da saída
  Mermaid; o pai nunca é um estado próprio, então o contêiner não seleciona
  nada (e uma máquina que por acaso declare um estado plano colidindo com o
  nome de um pai mantém essa família plana). Clicar em uma seção (no título ou
  na área vazia) seleciona o **estado de entrada** da máquina e enquadra essa
  máquina na view, então uma máquina pode ser inspecionada — e simulada — em
  um clique.
- **Inspetor** (painel direito) — selecionar um estado mostra seus selos de papel,
  a descrição e as etiquetas escritas nele na fonte analisada, seus eventos de
  entrada/saída (os documentados carregam uma marca e um tooltip) e os
  **Efeitos ao entrar**: a união dos efeitos que suas transições de chegada
  solicitam — "alguns destes disparam", nunca "todos". Selecionar uma transição
  mostra a descrição autoral do próprio evento, `de ↓ para`, mais os
  **efeitos** que ela solicita. Em ambos os casos a descrição da própria máquina
  encerra o painel. A máquina proprietária é etiquetada quando o core tem mais de
  uma.
- Cada efeito é uma solicitação com volta, e é renderizado como tal: a operação,
  um selo para a **capacidade** por onde ela trafega, `responde com` e os eventos
  que o shell pode devolver (os documentados mantêm seu tooltip), e um selo
  **pode** quando a solicitação está em um ramo que a transição não implica. Na
  união dos *Efeitos ao entrar*, as respostas das chegadas são reunidas e o
  **pode** só sobrevive quando toda chegada que faz a solicitação concorda que ela
  é condicional.

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
- **falha** (vermelho) — declarada, depois adivinhada. Um marcador `@failure` no
  comentário de documentação do estado na fonte analisada é autoritativo: ele
  viaja no modelo como dado, então é a declaração do *autor* e a regra de
  honestidade do parser continua valendo — nada foi inventado. Quando uma máquina
  não declara nenhuma falha, a heurística de nomenclatura entra no lugar
  (`Failed`, `Error`, `Denied`, `Rejected`, `Invalid`, `TimedOut`, …): o único
  palpite dos quatro, e é por isso que ele vive na UI (`isFailureName`) e nunca no
  parser. Um `@failure` em qualquer lugar de uma máquina silencia a heurística
  para aquela máquina inteira — dali em diante um estado sem marcação está sem
  marcação de propósito. Um estado que é ao mesmo tempo falha e final mantém a
  borda dupla em vermelho.
- **descontinuado** (âmbar, borda tracejada) — apenas declarado, a partir de
  `@deprecated`. Nenhuma heurística o sustenta e nenhuma deveria: um nome nunca
  diz que um estado está a caminho de sair. Os painéis também riscam o nome.
  Tracejado em vez de esmaecido, porque esmaecer já significa "fora do alcance da
  simulação".

O Inspetor e o painel de simulação repetem os papéis como selos.

## Documentação vinda da fonte

Comentários de documentação no enum de estado da aplicação analisada chegam ao
modelo e são renderizados **como estão** — são a prosa da própria aplicação,
então nunca são traduzidos (veja [i18n.md](i18n.md)). Apenas os títulos em volta
deles são.

No canvas, um estado documentado carrega uma pequena marca de três linhas depois
do rótulo e mostra sua descrição como tooltip nativo; uma caixa de seção faz o
mesmo pela descrição do próprio enum de estado. `title` em vez de um cartão de
hover de propósito: o React Flow escala o painel de nós, então um cartão dentro
de um nó fica borrado e um fora precisa de um portal posicionado contra a
transformação.

O Inspetor e o painel de simulação mostram o texto completo com as quebras de
parágrafo preservadas, mais quaisquer valores de `@tag` livres como chips
monoespaçados — monoespaçado porque uma etiqueta é dado da aplicação analisada,
diferente dos selos de papel em maiúsculas, que são o vocabulário desta UI. A
descrição de um estado fica logo abaixo do seu nome, sem título; a descrição da
própria máquina vem por último, sob *Sobre esta máquina*, junto com quaisquer
marcadores declarados na região.

Markdown dentro de um comentário de documentação **renderiza como Markdown**
nos painéis (react-markdown): trechos de código, listas, ênfase — a mesma
leitura que o documento gerado sempre deu. HTML cru na prosa autoral fica
deliberadamente inerte (mostrado como texto, nunca executado — react-markdown
constrói elementos React e não injeta HTML), e linhas `///` quebradas à mão se
rejuntam naturalmente, já que Markdown trata uma quebra simples como quebra
suave. Tooltips de nós e seções são atributos `title` nativos, então ali a
prosa continua texto puro.

Essa prosa vem de qualquer repositório que tenha sido analisado, então três
limites são declarados explicitamente em vez de herdados dos padrões da
biblioteca — veja
[security.md](security.md#1-prosa-do-autor-é-texto-não-confiável-em-todo-lugar-onde-chega),
e o `StateDoc.test.tsx` se você for mudá-los:

- **destinos de link** só podem ser `http`, `https` ou `mailto`, e abrem com
  `rel="noopener noreferrer nofollow"`. Um link `javascript:` renderiza como
  texto inerte.
- **imagens nunca são buscadas.** Um `![](https://host/pixel.png)` num
  comentário de documentação reportaria cada leitor de um documento publicado a
  esse host, então o texto alternativo é mostrado no lugar dela.
- **HTML cru não tem caminho até o DOM.** Sem `dangerouslySetInnerHTML`, e
  `rehype-raw` não deve ser adicionado.

## Filtrando o canvas

Dois filtros de leitura. Ambos dizem "estes, não o resto" do mesmo jeito que a
simulação: os estados que casam ficam em força total enquanto todos os outros
estados e transições esmaecem.

- **Filtrar por etiqueta** — o campo ao lado do título (ele lê, enquanto os
  botões da barra agem): digite um fragmento de um nome de `@tag` declarado,
  sem diferenciar maiúsculas. Ele carrega sua própria lista de sugestões —
  etiquetas mais usadas primeiro, aberta no foco — em vez de um `<datalist>`
  nativo, cujo popup é inconsistente entre engines. Uma etiqueta declarada no
  enum de estado cobre a região inteira. O campo só é renderizado quando o
  núcleo declara alguma etiqueta — sem nada para filtrar, não há filtro.
- **Sem documentação** — um botão opt-in (triângulo de aviso âmbar; âmbar
  quando ativo — o verde pertence à simulação) que mantém apenas os estados
  sem descrição autoral: os estados em que um leitor ainda não deveria
  confiar. Opt-in de propósito, para que a visão padrão continue sendo sobre a
  máquina e não sobre cobertura de documentação (o número em si vem do
  `crux-analyzer coverage`, veja [cli.md](cli.md)).

Os dois critérios compõem como interseção. Uma transição continua legível
apenas quando tudo que ela conecta continua; em uma aresta curinga o
pseudo-nó **qualquer estado** conta como casando — "qualquer estado" inclui os
mantidos — então `* → Ready` sobrevive sempre que `Ready` sobrevive.

Os filtros são lógica pura de domínio (`src/domain/focus.ts`) e chegam ao
Graph pela mesma prop de destaque que a simulação usa — um nível `kept`
silencioso que apenas escapa do esmaecimento, então um resultado de filtro
nunca toma emprestadas as cores da simulação. Enquanto uma simulação roda os
filtros ficam desabilitados: a ênfase pertence ao replay. Trocar de núcleo os
limpa — cada núcleo declara suas próprias etiquetas.

## Links diretos

A seleção vive no hash da URL — `#state=Core/Máquina/Nome`,
`#transition=<id>`, `#core=<nome>` — então "este estado desta máquina" é um
link que pode ser colado em um review. Cliques se espelham na barra de
endereço via `replaceState` (sem acumular histórico), a visão padrão mantém a
URL limpa, um hash colado aplica sem recarregar, e um link velho ou estrangeiro
cai de volta para o núcleo (ou para nada) em vez de uma UI quebrada. Baseado em
hash de propósito: a publicação estática não tem roteador nem regra de
fallback de SPA, e um hash sobrevive intocado a qualquer host
(`src/state/urlSelection.ts`).

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
  disparar a partir dele (os de origem curinga estão sempre disponíveis) e o
  histórico do que já disparou. Transições com destino de tempo de execução
  `to: "*"` não podem ser reproduzidas — não há nada estático onde aterrissar —
  então são listadas inertes sob as disparáveis, com uma nota dizendo
  exatamente isso, em vez de escondidas em silêncio;
- o replay modela a **outra metade do laço do Crux**. Disparar um evento registra
  o que ele pediu ao shell — o histórico carrega os nomes dessas solicitações sob
  cada passo, que é o único lugar onde uma solicitação de *disparar e esquecer*
  (`render()`) fica visível, e uma condicional aparece ali como `pode ter`, porque
  o replay não avalia o ramo em que ela está. Uma solicitação que declara resposta
  fica em **Aguardando o shell** até que um evento a responda, e o evento que responde uma
  solicitação pendente recebe o selo `do shell` na lista de disparáveis — assim "o
  que o usuário pode fazer em seguida" e "o que o shell te deve" param de parecer
  a mesma coisa. Uma resposta que nenhuma transição daqui trata é listada inerte
  com o mesmo tipo de nota de um destino de tempo de execução: comportamento real
  que não muda estado;
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
- **o histórico é onde você está**, não só o que aconteceu: clicar em qualquer
  passo além do atual leva você até ele — para trás se já foi dado, para frente se
  ficou atrás. O próprio card é o controle (um botão de verdade, então alcançável
  pelo teclado), e voltar *não* joga fora o que você já tinha feito — os passos seguintes continuam listados e inertes
  (`ahead`), com uma nota dizendo isso. Disparar o mesmo evento de novo entra
  neles; um movimento diferente é o que os substitui. Qualquer posição é
  reconstruída *reexecutando* a corrida registrada, nunca a partir de snapshots
  guardados, então o estado atual, o caminho percorrido e as solicitações em voo
  voltam consistentes por construção;
- **Reiniciar** volta ao primeiro estado da máquina (e descarta a corrida
  registrada); **Parar simulação** retorna ao inspetor.

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
posições relativas dos filhos), e o agrupamento tem profundidade arbitrária:
contêineres de compostos aninham dentro das seções, com cada máquina calculada
como uma execução hierárquica única (`INCLUDE_CHILDREN`) para que arestas
possam cruzar a fronteira de um composto, e cada aresta declarada no menor
ancestral comum dos seus extremos. Os nós não são arrastáveis — as rotas
pertencem ao motor. **Reorganizar** recalcula *e* re-enquadra a viewport: o
layout é determinístico, então só recalcular não mudaria nada visível.

## Estendendo

- Nova visualização dos mesmos dados: consuma o modelo de domínio
  (`src/domain/types.ts`), não os tipos do React Flow.
- Novo algoritmo de layout: implemente `LayoutEngine` e troque em `App.tsx`.
- Nova funcionalidade dirigida por destaque (à la simulação): calcule ids e
  passe-os pela prop `highlight` do Graph — não modifique o Graph.
