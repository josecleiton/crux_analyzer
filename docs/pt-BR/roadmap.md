# Roadmap

> 🌐 [English](../roadmap.md) · **Português (Brasil)**

A especificação original (`init.md`) está totalmente implementada. O que vem a
seguir não é "mais parsing" — o parser entende o que se propôs a entender. O
trabalho aberto é sobre **adoção** e sobre **impedir que a documentação
apodreça**.

Este documento é a fonte única do trabalho planejado; o `CLAUDE.md` aponta para
cá em vez de manter a própria lista.

## A tese

O crux_analyzer se vende como documentação viva, e hoje nada impede que essa
documentação minta. Um modelo é regerado quando alguém lembra de regerar; um
aviso do parser é impresso e sobe na tela; um estado sem descrição parece
exatamente igual a um cuja descrição alguém apagou.

Então a ordem abaixo é deliberada: **tornar a documentação defensável antes de
fazê-la alcançar mais longe.** Uma extensão do VS Code multiplica o público de
qualquer que seja a qualidade que a ferramenta hoje garante — e é por isso que
ela vem depois das garantias, não antes.

---

## 1. A catraca — dar dentes à documentação ✅ **feito**

O trabalho de maior alavancagem, e o mais barato. Entregue como três incrementos
independentes; veja [cli.md](cli.md) e
[development.md](development.md#o-que-o-ci-garante).

- **CI rodando `just check`** — `.github/workflows/ci.yml`. O `just check` já
  fazia a coisa certa; ele só nunca rodava a menos que uma pessoa digitasse. O
  teste do corpus se auto-restringe por `QUIPU_SRC` e essa fonte não é pública,
  então o CI comprova o caminho do fixture e o corpus continua sendo uma guarda
  local.
- **`--deny-warnings`** — uma flag global que sai com código diferente de zero
  quando o parser reportou algo. Transforma "o corpus extrai limpo" de uma
  observação no `parser.md` em algo que um pipeline garante. A saída ainda é
  escrita: o código de saída é o sinal.
- **`crux-analyzer coverage`** — a fração de estados que carregam uma
  *descrição*, por máquina e no total, falhando abaixo de `--min`. A documentação
  de estados tornou isso mensurável pela primeira vez. Documentação que se pode
  adicionar é boa; documentação que se pode *medir* é a que de fato é escrita.

Duas guardas saíram disso e vale mantê-las honestas: `just fixture-guard` (o
fixture precisa extrair com zero avisos e não perder documentação) e
`just docs-current` (um exemplo gerado versionado precisa corresponder ao
gerador). As duas foram quebradas de propósito uma vez e observadas ficando
vermelhas — uma guarda que não pode falhar é decoração.

**Encerrado:** o corpus agora tem catraca própria — `just quipu-coverage`
(parte do `just check`) falha quando o total do Quipu cai abaixo do piso no
`justfile`, e pula a si mesma onde a fonte está ausente, como o teste do
corpus. O piso começa nos 53% de hoje; o `RecordingState` continua em 13% e sem
descrição no próprio enum, que é exatamente o número que a catraca agora guarda
enquanto espera para ser elevada.

---

## 2. Fechar o loop das etiquetas ✅ **feito**

`@tag` existia no modelo e renderizava como chips, mas era **inerte**: dava
para declarar uma etiqueta e olhar para ela, não para *usá-la*. Com oito
estados isso não incomoda; com trinta é a diferença entre um diagrama e uma
ferramenta. As duas metades saíram em um incremento só; veja
[web-ui.md](web-ui.md#filtrando-o-canvas).

- **Filtro e busca por etiqueta na UI web** — digite `retryable` (ou um
  fragmento; o campo sugere as etiquetas do próprio núcleo), mantenha os
  estados que a carregam, esmaeça o resto. O esmaecimento *é* o da simulação,
  alcançado pela mesma prop de destaque via um nível `kept` silencioso, então
  o Graph continuou um renderizador puro e a lógica de casamento é um módulo
  de domínio testado (`src/domain/focus.ts`).
- **Destacar estados não documentados** — um botão opt-in **Sem documentação**
  mantém apenas os estados sem descrição autoral. Opt-in como planejado: a
  visão padrão continua sendo sobre a máquina, e o *número* continua com o
  `crux-analyzer coverage`.

---

## 3. Alcance — a extensão do VS Code ✅ **feito**

O maior público: a máquina de estados ao lado do código, sem sair do editor.
Ela chegou exatamente como a arquitetura previa — outro cliente do mesmo
contrato JSON, e um cliente pequeno, porque toda camada de que ela precisava já
existia. Veja [vscode.md](vscode.md).

`apps/vscode` embute o bundle web compilado em um webview e executa a CLI; o
modelo é injetado como `window.__CRUX_MODEL__` (o contrato de embutimento que o
`loadProject` honra), um watcher regenera a cada salvamento de `.rs` — o loop
de *escrita*, complementando o de leitura do `just site` — e os avisos do
parser aparecem em um canal de saída em vez de serem descartados. A adaptação
ao webview (re-enraizamento de assets, CSP com nonce, injeção do modelo) é um
módulo puro testado em unidade; a parte do host de extensão é encanamento
fino.

---

## 4. Lacunas menores que valem correção ✅ **feito**

Observadas durante a construção; as seis fechadas, na ordem em que foram
listadas:

- **Estados compostos aninham no grafo web** — um pai composto é um contêiner
  segurando suas folhas, o aninhamento que o Mermaid sempre teve. O motor de
  layout generalizou para agrupamento de profundidade arbitrária (uma execução
  hierárquica do ELK por máquina, arestas declaradas no menor ancestral
  comum).
- **A seleção é uma URL** — `#state=Core/Máquina/Nome`; links colados aplicam
  sem recarregar e os velhos caem de volta limpos.
- **Comentários de documentação em eventos e efeitos** — entrou *aditivamente*
  em vez da quebra de contrato prevista: catálogos `events` / `effects` por
  núcleo de `{ name, doc }`, apenas nomes documentados e usados, então uma
  aplicação sem documentação emite JSON byte-idêntico. Renderizados pelo
  gerador Markdown (tabelas por núcleo) e pelo Inspetor (doc do evento sob a
  transição, marcas + tooltips nas listas).
- **Efeitos agregam por estado** — os *Efeitos ao entrar* do Inspetor: a união
  sobre as transições de chegada, apresentada como união.
- **Transições com destino de runtime são explicadas no painel de simulação** —
  listadas inertes com uma nota, em vez de silêncio.
- **Markdown renderiza nos painéis web** (react-markdown — a dependência de
  que tratava o adiamento). HTML cru na prosa autoral fica texto inerte,
  verificado com um modelo hostil; tooltips nativos continuam texto puro.

---

## 5. Deliberadamente ainda não

- **Gerador PlantUML.** Listado no `init.md`, mas o Mermaid já renderiza
  nativamente no GitHub/GitLab e o `just site` cobre o resto. Um gerador inteiro
  novo para muito pouco alcance — por último, se algum dia.
- **Estilo de marcadores no Mermaid** (`classDef`). Um preenchimento fixo quebra
  em um leitor em modo escuro e o suporte varia entre renderizadores. Se algum
  dia entrar, é atrás de uma opção explícita do gerador, não na saída padrão.
- **`#[doc(hidden)]` como "esconda este estado".** Tentador e errado: o estado
  existe na máquina, e esconder faria o diagrama mentir por omissão.
- **Inferir marcadores a partir de nomes no parser.** A heurística de
  nomenclatura fica nos clientes. Veja a regra de honestidade em
  [architecture.md](architecture.md#regras-rígidas).
