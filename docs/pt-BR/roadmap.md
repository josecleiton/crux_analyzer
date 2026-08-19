# Roadmap

> 🌐 [English](../roadmap.md) · **Português (Brasil)**

A especificação original (`init.md`) está totalmente implementada, então a maior
parte do que vem a seguir é sobre **adoção** e sobre **impedir que a documentação
apodreça**, não sobre mais parsing. Uma exceção, e foi preciso um app real para
encontrá-la: a §6 era uma máquina de estados que o parser lia o suficiente para
saber que existia e ainda assim não extraía — a primeira lacuna genuína de parsing
desde que a especificação foi cumprida, agora fechada.

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
  teste contra uma aplicação alvo privada se auto-restringe por `APP_SRC` e
  nunca entra neste repositório, então o CI comprova o caminho do fixture e uma
  aplicação real continua sendo uma guarda local.
- **`--deny-warnings`** — uma flag global que sai com código diferente de zero
  quando o parser reportou algo. Transforma "uma aplicação real extrai limpo" de uma
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

**Encerrado:** a aplicação alvo privada ganhou uma catraca de cobertura própria,
uma receita que falhava quando o total de documentação dela caía abaixo de um
piso embutido no `justfile`. Essa receita se foi: uma guarda que ninguém fora de
uma máquina consegue rodar não pertence a um task runner compartilhado, e o piso
nomeava uma aplicação que este repositório não deve nomear. Rode
`just coverage <caminho> <nome> <piso>` contra uma aplicação privada localmente;
o `fixture-guard` é a catraca pública que o CI mantém clicando.

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

## 4b. Endurecimento para uso público ✅ **feito**

Motivado pela pergunta "temos problemas de segurança?" antes de colocar isto
diante de um público. Uma auditoria dos dois lados encontrou quatro classes, todas
agora fechadas e todas fixadas por testes. O [security.md](security.md) é o
contrato permanente — modelo de ameaça, regras, e as propriedades que não devem
ser negociadas — e o `CLAUDE.md` carrega a forma curta como par da regra de
honestidade do parser.

- **Limites de recursos no parser.** O walker que segue chamadas quebrava *ciclos*
  de recursão mas nunca limitava o fan-out, então um grafo de chamadas em diamante
  com ~40 funções minúsculas descrevia 2⁴⁰ caminhadas — um travamento e um OOM a
  partir de um arquivo de 60 linhas. Agora há um orçamento de passos, limites de
  profundidade e de profundidade de chamadas, mais limites de tamanho por arquivo
  e total, e uma verificação prévia de aninhamento de delimitadores
  (`syn::parse_file` recursa sobre aninhamento e seu stack overflow *aborta o
  processo*, então essa precisa rodar antes do parsing). Todo limite reporta um
  `Warning`: a regra da honestidade aplicada a recursos, o que faz o
  `--deny-warnings` cobrir truncamento de graça.
- **Codificação de saída no docgen.** A prosa dos doc comments chegava ao Markdown
  publicado verbatim, então HTML cru se tornava um elemento real e uma linha em
  forma de cerca sequestrava a cerca do diagrama. Agora `<`/`&`/`>` são escapados
  na prosa enquanto o *Markdown* autoral é preservado, cercas são calculadas para
  superar seu conteúdo, células de tabela escapam a barra invertida antes do pipe,
  e ids do Mermaid são gerados, verificados contra colisão e contra palavras
  reservadas, com o nome real num rótulo entre aspas.
- **A postura de Markdown da UI web agora é explícita e testada.** Os padrões do
  react-markdown já eram seguros, mas isso era uma propriedade da dependência; a
  lista de protocolos permitidos, o `rel` dos links e o não-carregamento de
  imagens estão declarados no `StateDoc.tsx` e fixados pelo `StateDoc.test.tsx`.
  Mais um CSP no site estático (hashes calculados no build, não colados), um error
  boundary, e a correção de uma busca pela cadeia de protótipos que permitia a uma
  variante de evento chamada `constructor` apagar a aplicação.
- **A extensão e a cadeia de suprimentos.** `cruxAnalyzer.binary` tem escopo de
  máquina para que um repositório clonado não escolha o executável;
  `cruxAnalyzer.src` é contido à raiz do workspace. O CI declara `permissions:`,
  passa `github.event.*` por `env:` e fixa actions de terceiros em SHAs de commit.
  O `just security` (`cargo deny` + `pnpm audit`) é bloqueante dentro do
  `just check`, com o dependabot mantendo os pins frescos.

Deliberadamente *não* feito: fuzzing do parser (`cargo-fuzz` sobre
`parse_project`) seria o próximo passo natural e está listado na §7.

---

## 4c. Escape que vai além da conta

Encontrado lendo o documento gerado de uma aplicação real, não por um teste: a
passagem de codificação da §4b está certa sobre *sair* do Markdown e um pouco
errada sobre *permanecer* nele. As duas metades são o mesmo erro — escape
aplicado onde a marcação do autor deveria sobreviver — e o contrato que elas
violam já está escrito: o Markdown do autor é uma funcionalidade, só a
capacidade de sair dele é removida.

- **Backticks numa célula de tabela ✅ feito.** `table_cell` os escapava, então um
  `` `campo` `` documentado chegava ao leitor como um `` \`campo\` `` visível — 13
  células numa única aplicação alvo. A razão declarada ("um backtick solto
  derrama formatação de código pelo resto da linha") não se sustenta: uma linha
  de tabela é dividida nos seus pipes não escapados *antes* de suas células
  serem interpretadas como conteúdo inline, então um backtick não pode atravessar
  uma coluna, e um backtick ímpar já é literal. O escape saiu; o escape do pipe,
  que precisa sobreviver dentro de um code span, está fixado por um teste próprio.
- **Entidades dentro de um code span — aberto.** `<`, `>` e `&` são escapados na
  string inteira, code spans incluídos, e o CommonMark não decodifica
  referências de entidade dentro de um code span. Então um doc comment escrito
  como `` `Option<String>` `` é publicado como um `Option&lt;String&gt;` literal.
  Afeta `prose_block` e `table_cell` igualmente, e não é hipotético para uma base
  de código Rust — só precisa de uma aplicação que documente um tipo genérico, o
  que nenhuma fixture e nenhuma aplicação alvo faz ainda. A correção é escapar *em
  volta* dos code spans em vez de através deles, o que significa que `prose_block`
  tem que reconhecer um span como já reconhece uma linha em forma de cerca:
  procurar sequências de backticks e deixar em paz o que está entre um par
  casado. Barato, mas é parsing inline de verdade, então quer testes de entrada
  hostil próprios (sequências não casadas, sequências de tamanhos diferentes, um
  span contendo um `<script>` literal) antes de substituir um `replace` cego.

---

## 5. Distribuição — colocar na mão de outras pessoas

Ninguém fora deste checkout consegue instalar a ferramenta. `cargo run` e `just`
são interface de contribuidor, e a extensão do VS Code, quando não acha o
binário, manda o usuário rodar `cargo install --path crates/cli` — um comando que
não significa nada para quem nunca clonou o repositório. É a última frente não
endereçada, e pertence a este lugar pela tese acima: distribuição é o "alcançar
mais longe" definitivo, então vem depois das garantias.

### A ordem, e por que ela não é uma hesitação

1. **`cargo install crux-analyzer` do crates.io** — o canal principal. O público é
   de desenvolvedores Rust/Crux; todos já têm toolchain, e isso contorna as duas
   piores partes de distribuir binário (o Gatekeeper do macOS pondo em quarentena
   um download sem assinatura, e "qual arquivo eu quero"). Sem CI, sem segredos,
   sem assinatura.
2. **Binários prontos em GitHub Releases** — não primariamente para humanos, mas
   porque a §5.4 precisa deles: uma extensão de Marketplace que diz "vá instalar
   Rust" não tem público.
3. **Marketplace do VS Code + Open VSX** — em cima de (2).

Clonar e compilar continua documentado, rebaixado ao caminho do contribuidor.
Note o que já funciona hoje, sem nenhuma mudança no repositório:

```sh
cargo install --git https://github.com/josecleiton/crux_analyzer crux-analyzer-cli --locked
```

### 5.1 crates.io

São **os cinco crates ou nenhum**: um crate publicado não pode depender de uma
dependência por caminho não publicada, e colapsar as bibliotecas dentro do
binário violaria as [regras rígidas](architecture.md#regras-rígidas). A
mitigação já está no lugar — todo nome tem o prefixo `crux-analyzer`, então eles
são auto-namespaced. Os cinco nomes estão livres hoje.

O bloqueio duro é mecânico: as dependências entre crates são só por caminho, sem
a chave `version`, o que o `cargo publish` rejeita de saída em vez de avisar.
`[workspace.package]` também não tem `repository`, `keywords` nem `categories`.
Vale registrar porque elimina uma categoria inteira de ferramental: **`cargo
publish --workspace`** (cargo ≥ 1.90) ordena o DAG topologicamente *e* espera a
propagação do índice entre crates, então "publicar as folhas primeiro, dormir
esperando o índice" é uma flag, não um problema. Publicar do laptop, não do CI —
um segredo a menos, e cortar uma versão continua sendo um ato humano deliberado.

Renomear `crux-analyzer-cli` → `crux-analyzer` **antes** de publicar qualquer
coisa, para o comando documentado nunca mudar: o crate que *é* o produto deve ter
o nome curto, e `cargo install crux-analyzer-cli` instalando um binário chamado
`crux-analyzer` é uma unha encravada que se explicaria para sempre. Toca ~20
lugares (`Justfile`, `README.md`, `cli.md` e `development.md` com seus gêmeos) e
**zero** no CI, que só chama `just check`.

### 5.2 Binários prontos — um workflow escrito à mão, não `cargo-dist`

O `dist init` gera e depois *é dono* do `release.yml`, e não sabe nada da metade
pnpm do monorepo, então a matriz de VSIX terminaria em um segundo workflow de
todo jeito — e aí seriam dois para manter, um gerado e um à mão. Um `release.yml`
disparado por tag reusando as recipes do `just` combina com o jeito que todo o
resto aqui funciona: um humano consegue rodar cada passo localmente.

Alvos: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-musl`, `x86_64-pc-windows-msvc`. **musl em vez de gnu, de
propósito** — um binário `-gnu` compilado no `ubuntu-latest` liga contra a glibc
daquela imagem e morre com `GLIBC_2.xx not found` em uma distribuição mais velha,
o relato mais comum de "seu binário de release não funciona". O conjunto de
dependências é Rust puro, sem `build.rs` e sem ligação com C, então o musl
compila limpo e dá um artefato estático que roda em qualquer lugar.

### 5.3 Versões em passo travado

A extensão conversa com o CLI pelo contrato JSON, então o workspace Rust, o
`package.json` da raiz e o `apps/vscode` publicam um número só: "extensão 0.4.x
precisa do CLI 0.4.x" é uma frase que cabe na cabeça, e uma matriz de
compatibilidade não. `apps/web` fica em `0.0.0` de propósito — artefato de build,
nunca publicado. Garantido por uma recipe `version-check` dentro do `just check`,
não por um bot. Uma pegadinha que vale anotar: subir a versão do workspace
invalida o `Cargo.lock`, e os builds de release usam `--locked`, então o lock
regenerado faz parte do commit que sobe a versão.

### 5.4 O bloqueio real da extensão

`apps/vscode/src/panel.ts` chama um binário do `PATH` e, ao falhar, imprime a
mensagem do `cargo install --path crates/cli` descrita acima. O caminho adiante é
um módulo puro de resolução no estilo do `sourceDir.ts` já testado —
configuração explícita ganha, depois um binário embutido em `bin/`, depois o
`PATH` — mais `vsce package --target` por plataforma, com um VSIX sem alvo como
alternativa. *Baixar na ativação* está descartado: um Mach-O baixado ganha
`com.apple.quarantine` e o Gatekeeper se recusa a executá-lo, então todo aquele
código de rede e checksum compra um resultado pior do que passar `--target`.

Uma armadilha a lembrar quando essa mensagem for reescrita: no `vscode.l10n` **a
string em inglês é a chave do catálogo**, então reescrevê-la orfana a entrada
pt-BR em `apps/vscode/l10n/bundle.l10n.pt-br.json` e o usuário pt-BR passa a ver
inglês em silêncio, sem nenhum teste falhar. Um teste de paridade tornaria isso
um build vermelho; o lado web já tem o padrão.

### 5.5 Conformidade de licenças de terceiros ✅ **feito**

Duas obrigações que estavam descumpridas a cada push em `main`, fechadas juntas. O
`THIRD-PARTY-NOTICES.md` gerado agora vai em `apps/web/dist/` (portanto no Pages e
dentro de `media/web`), na raiz do VSIX, e na raiz do repositório como a união das
notas dos dois artefatos; o `just notices-current` dentro do `just check` o mantém
honesto. As regras estão em `docs/security.md` §10.

Investigar o assunto corrigiu duas coisas que esta seção afirmava:

- **A cláusula vinculante da EPL-2.0 é a §3.1(a), não a §3.2.** A §3.2 ("uma cópia
  deste Agreement deve ser incluída com cada cópia") é escopada a *"When the
  Program is Distributed as **Source Code**"*. Nós distribuímos código-objeto, onde
  vale a §3.1(a): acompanhar de uma declaração de que o fonte está disponível sob o
  Agreement, e dizer como obtê-lo. O arquivo de notas faz as duas coisas, mais o
  texto integral.
- **Nada estava "removendo" a nota do elkjs** — o `elk.bundled.js` é distribuído
  sem nenhum cabeçalho de copyright, então a §3.3 nunca foi o problema real. O que
  *estava* sendo removido, pelo minificador, era o cabeçalho `@license` do **React**
  e a linha `Copyright (c) Meta Platforms`: o bundle não carregava nota de copyright
  nenhuma. `comments: { legal: true }` as restaurou pelos 1.687 bytes que custa, e o
  arquivo de notas cobre os pacotes que não distribuem cabeçalho inline.

O escopo também era maior que "todo VSIX embarca código MIT": 68 pacotes
contribuem código para o bundle, e MIT, ISC e BSD-3-Clause todas carregam termos de
retenção de nota. O gerador é dirigido pelos chunks que o bundler emitiu em vez da
árvore instalada — o que é ao mesmo tempo o escopo correto (sem `@types/*`, que não
distribuem nada) e o único que funciona, já que o `pnpm licenses list` reporta
caminhos de store que não resolvem neste layout de instalação.

O elkjs também passou a ser um chunk próprio. Dois ganhos: nenhum arquivo de saída
mistura código EPL-2.0 com o nosso, então o "qualquer novo arquivo que contenha
qualquer conteúdo do Program" da EPL-2.0 nunca precisa ser discutido — e, como o
elkjs era 82% do bundle, é também a resposta ao aviso de 500 kB.

### 5.6 Recusado, com gatilho de revisita

- **Tap do Homebrew.** Um segundo repositório e uma fórmula precisando de um novo
  SHA a cada release, para um público que tem cargo. *Revisitar se um usuário
  não-Rust pedir.*
- **Pacote npm para o schema.** Publicá-lo cria uma obrigação de versionamento
  sobre o contrato que o git satisfaz de graça; a URL bruta em uma tag é a
  resposta inteira. *Revisitar se aparecer um cliente de terceiro.*
- **Assinatura de código / notarização.** Uma conta de Apple Developer e um
  certificado Windows para evitar uma linha `xattr -d com.apple.quarantine` na
  documentação — mais um argumento a favor do canal (1). *Revisitar se o
  Gatekeeper virar um custo de suporte real.*
- **`cargo-dist`.** *Revisitar se a matriz de alvos crescer além de um YAML
  legível.*
- **`release-plz` / `cargo-release`.** O recurso principal deles agora é o
  `cargo publish --workspace`, e nenhum dos dois conhece o
  `apps/vscode/package.json` — então quebrariam o passo travado da §5.3 em vez de
  garanti-lo. *Revisitar com contribuidores externos ou uma cadência fixa.*

---

## 5b. Efeitos se tornam a outra metade do laço ✅ **feito**

Motivado pela pergunta "mapeamos bem os eventos de entrada e saída — e os
efeitos?" A resposta era que um efeito era uma *string*: um rótulo em uma
transição, coletado por braço de evento, sem capacidade, sem volta e sem
honestidade sobre braços que ramificam. Eventos tinham um vocabulário inteiro;
efeitos tinham um nome.

Quatro coisas entraram juntas, porque são uma única leitura da mesma fonte:

- **A capacidade.** `Effect::Audio(AudioOperation)` diz que toda solicitação de
  `AudioOperation` passa por `Audio`. Estrutura, não um chute pelo formato do
  nome — e responde a uma pergunta que as tabelas de transição respondiam mal: com
  o que este núcleo conversa? O gerador Markdown ganhou uma tabela
  **Capacidades** por núcleo por causa disso.
- **A resposta (`resolvesWith`).** O laço do Crux é
  `Evento → Efeito → shell → Evento`, e o evento de callback está escrito *no local
  da solicitação*, então lê-lo é evidência, não inferência. Os três formatos reais
  são lidos — um evento passado ao lado da operação, `then_send(Event::X)` e um
  callback que mapeia o resultado — mais uma chamada adiante dentro de um helper de
  solicitação, que é como o app-alvo real escreve. Um **conjunto**, porque uma
  solicitação tem uma resposta por desfecho; o helper de áudio compartilhado do
  app-alvo responde legitimamente com treze. Um `then_send` que nomeia algo
  ilegível é um aviso novo (`unresolved-effect-callback`); uma solicitação *sem*
  callback não é, porque disparar e esquecer é um formato legítimo.
- **Escopo de ramo, e `conditional`.** Efeitos eram compartilhados por todas as
  transições de um braço, uma sobreaproximação que o modelo nunca admitia. Agora a
  cadeia de alternativas percorrida até a solicitação é comparada com a da
  atribuição: a solicitação de um ramo irmão não cai mais nesta transição, e uma
  encontrada *mais profunda* viaja com ela marcada como condicional — "chegar aqui
  *pode* solicitar isso". A regra de honestidade aplicada à atribuição, e não à
  extração.
- **Efeitos no diagrama, e no replay.** Rótulos de transição no Mermaid são
  `evento / efeito` (a convenção de statechart; o diagrama vinha escondendo os
  efeitos por completo), e a simulação agora modela a volta: uma solicitação com
  resposta declarada aguarda em *Aguardando o shell*, o evento que a responde
  recebe o selo `do shell` na lista de disparáveis, e uma resposta que nenhuma
  transição trata é listada inerte em vez de escondida.

`Effect` passou de string simples para "string ou objeto" do mesmo jeito que
`states[]` (§4), então um app cujas solicitações não mostram capacidade nem
callback continua emitindo JSON byte a byte idêntico.

**Deliberadamente de fora:** anotações `@failure` / `@tag` em *variantes* de
efeito. O [parser.md](parser.md#efeitos) dizia que não havia nada que um marcador
significasse em um efeito; com capacidades e respostas agora haveria (uma
solicitação que pode falhar, uma capacidade que valha filtrar), então isso é um
incremento real e não uma recusa — só quer um caso de uso vindo da adoção
primeiro, como as etiquetas quiseram.

---

## 6. Máquinas atribuídas só por value flow ✅ **feito**

Encontrada rodando contra um app alvo, e reproduzível só pela forma. Dois enums de
status de forma deliberadamente idêntica, ambos guardados por entidade em uma
coleção que o model possui, ambos documentados como máquinas de estados pelo app.
Um é extraído; o outro é invisível.

A diferença inteira é uma linha. A detecção exige evidência de atribuição
*literal* — `*.campo = Enum::Variant`, ou um reset `T::default()`
([state_enum.rs](../../crates/parser/src/state_enum.rs)). O que é extraído tem
exatamente uma linha dessas, porque é o core que *inicia* aquele trabalho e
portanto é o core que escreve a variante de "em andamento"; a análise de value flow
então colhe de graça as atribuições `= status` restantes vindas de payload. O
invisível não tem nenhuma: ali quem inicia o trabalho é a shell, então o core só
*armazena* o que a shell reporta — por uma atribuição de payload e um `.clone()` de
campo para campo. Nenhuma das duas é um caminho literal de variante.

Essa assimetria no app é honesta: ela reflete qual lado é dono da transição, e
nenhuma reescrita ganha o diagrama sem inventar uma atribuição que mente sobre essa
posse. A lacuna é nossa.

**O que faz disso uma lacuna e não uma limitação:** o parser *lê* o enum que deixa
de extrair. Os guards que o comparam — um `==` contra uma variante, um `matches!`
sobre outras duas — já o colocam em `dispatched_enums`. Então o parser sabe que o
enum existe, conhece suas variantes, e não emite máquina **nem warning**: silêncio
onde a regra da honestidade exige um diagnóstico.

Um fixture genérico reproduzindo isso cabe em `crates/parser/tests` ao lado do
`mini_recorder`, para o caso ficar coberto por um teste versionado em vez de só por
um privado.

### A regra de evidência: campos alcançáveis pelo model

Alargar a detecção para aceitar atribuição por value flow não pode ser tão simples
quanto "qualquer campo cujo tipo declarado seja um enum de crate despachado" — isso
readmite os mirror enums de ViewModel que a regra de atribuição literal existe para
excluir (o [state_enum.rs](../../crates/parser/src/state_enum.rs) abre dizendo
exatamente isso). A decisão é exigir que o campo atribuído seja **alcançável a
partir do tipo associado `Model`**: mirror enums são construídos dentro de structs
de view, nunca guardados pelo model, então a alcançabilidade os separa sem
heurística de nome e sem enfraquecer a regra da honestidade.

Dois pré-requisitos, ambos descobertos ao dimensionar isso e ambos maiores que a
mudança de detecção em si. Os dois já estão feitos; ficam registrados aqui porque
cada um codifica uma distinção que um refactor posterior achataria com facilidade:

- **Tipos de campo de struct eram registrados sem atravessar coleções.** O índice
  guarda o tipo de um campo como o último segmento do caminho, então
  `items: Vec<Entry>` é indexado como `("items", "Vec")` e uma travessia de
  alcançabilidade quebra no `Vec` — que é exatamente onde vive um status por
  entidade (`Model` → … → algum subestado → `Vec<Entry>` → o campo de status). O
  `variant_fields` tem um desempacotador, mas ele atravessa só `Box`/`Rc`/`Arc`.
  Isso pede um desempacotador *separado* para campos de struct em vez de alargar o
  compartilhado, por duas razões: o compartilhado também alimenta a detecção de
  estados compostos, onde ensiná-lo sobre `Vec` muda o que pode ser lido como
  sub-estado; e o caminho de reset `T::default()` precisa do tipo *declarado*, já
  que `default()` em um campo `Option<E>` dá `None` e não uma variante de `E` —
  desempacotar ali inventaria uma atribuição. Então um campo carrega os dois tipos:
  declarado (rege os resets) e alcançável (rege a travessia).
- **O tipo associado `Model` nunca era resolvido.** O core finder lia os tipos
  associados `Event` e `Effect` e ignorava `Model`. Barato — o helper
  `associated_type` que já existia cobria — mas nada tinha precisado dele antes.

Também adicionado no caminho: um limite de profundidade nos dois walkers de tipo.
Argumentos genéricos aninham sem limite e o pré-check de brackets do loader conta
`(`, `[` e `{`, nunca `<`, então um `Box<Box<…>>` fundo o bastante estouraria a
pilha. O desempacotador `Box`/`Rc`/`Arc` que já existia tinha a mesma exposição.

### O que entrou ✅

Os dois pré-requisitos e o alargamento em si, mais o fixture versionado
`value_flow_status` que a seção acima pedia. A regra de evidência está no
[parser.md](parser.md#detecção-de-máquinas); nenhum `WarningKind` novo foi
necessário, então os catálogos de locale ficaram intactos.

Duas coisas saíram diferentes do planejado. O diagnóstico `untracked-state-enum`
nunca foi escrito: com a detecção alargada não existe mais uma ausência silenciosa
para reportar, e inventar um warning para um enum que o parser agora extrai seria
ruído. E um silêncio *diferente* apareceu no lugar dele, que é sobre o que trata o
resto desta seção.

### Constraints de origem agora carregam um sujeito ✅

Alargar a detecção expôs uma segunda lacuna, pré-existente e pior que a primeira
porque ela derrubava uma transição de uma máquina que o leitor *vê*. A evidência de
origem era chaveada por nome de campo, enquanto o espelho de valor que resolve alvos
é chaveado por caminho exato. Então um guard em `other.status` restringia uma
máquina em `field: status` mesmo quando `other` era outro registro, e numa escrita
de carry-over os dois conjuntos se intersectavam em nada:

```rust
if this.status == Pending && matches!(other.status, Done | Deferred) {
    this.status = other.status.clone();   // {Pending} ∩ {Done, Deferred} = ∅
}
```

Um conjunto de origem vazio fazia o laço de emissão iterar zero vezes — nenhuma
transição, nenhum warning.

Corrigido dando a toda avaliação de origem um **sujeito**: o objeto cujo campo de
estado a atribuição escreve. Um guard só conta como evidência quando seu receiver
pode ser aquele objeto, então o primeiro conjunto acima resolve a origem e o segundo
fica corretamente para o espelho de alvos. Contradição continua sendo reportada em
vez de derrubada, como rede de segurança para constraints que de fato não podem
valer.

O que impediu isso de ser uma linha, e vale lembrar antes de "simplificar":

- **A comparação de receiver precisa continuar permissiva.** Chavear por nome de
  campo era estrutural porque um objeto é alcançável por vários caminhos — um helper
  escreve `session.state` sob um guard que quem chamou escreveu em
  `model.recording.session.state`. Igualdade mais casamento por sufixo pontuado
  cobre isso; um receiver irresolvível é aceito, não rejeitado. Apertar para
  igualdade estrita estreita em silêncio todo guard escrito através de um alias
  local.
- **O sujeito não é derivável só da atribuição.** Escrever o campo direto o torna o
  receiver; resetar a struct que *guarda* o campo (`model.session = T::default()`)
  o torna o lado esquerdo inteiro. Colapsar os dois — a primeira coisa que parece
  duplicação aqui — transforma todo reset `default()` de volta em origem curinga.
  Essa regressão é pega pelo `default_reset_lands_on_default_variant`.

Verificado como aditivo e não meramente verde: sobre um app alvo real as três
máquinas que já extraíam mantiveram conjuntos de transição byte-idênticos (7, 6 e
30), incluindo o caso de alias, e a recém-detectada foi de ausente para três
transições. O `crates/parser/tests/value_flow_status.rs` cobre as duas metades em um
fixture versionado.

**A outra metade daquela investigação, e explicitamente não é trabalho de parser.**
O mesmo app documentava uma segunda máquina que o analyzer também não pegou — mas
ali o código não guarda enum nenhum, só vários campos correlacionados (um id
opcional, um booleano, um float de progresso) resetados juntos por um método. Não
existe nada para a análise de assignments encontrar, e o parser está certo em não
adivinhar: inferir máquina de um booleano e um `Option` é precisamente a inferência
baseada em nome que fica nos clientes
([architecture.md](architecture.md#regras-rígidas)). Uma máquina assim quer um enum
na aplicação primeiro — que é também o que tornaria irrepresentáveis suas
combinações impossíveis. Registrado para a distinção ficar documentada: **um
diagrama ausente é lacuna do parser só quando a fonte de fato declara os estados.**

---

## 6b. A entrada e o beco sem saída, em toda saída ✅ **feito**

Duas lacunas encontradas lendo um documento gerado ao lado do app: o analyzer
pintava `initial` e `final` no canvas e não mencionava nenhum dos dois em documento
gerado algum — `[*]` não aparecia em **lugar nenhum** do repositório — e a derivação
de `initial` não fazia o que o próprio comentário do modelo prometia.

**As saídas discordavam.** `Marker` é um vocabulário fechado (`failure`,
`deprecated`) e corretamente, mas isso deixava a tabela de estados do Markdown sem
coluna para os dois papéis derivados e o Mermaid sem pseudo-estado de início ou de
fim. "Não tem estado final na documentação" era verdade para *toda* máquina,
inclusive as que têm um.

**A derivação prometia demais.** O comentário do `Marker` e o schema diziam que
`initial` era derivado da forma do grafo *e* de `#[default]`, enquanto `StateDecl`
não carregava `#[default]` nenhum — o parser conhecia a variante default (é com ela
que resolve resets `T::default()`) e nunca a repassava. Então uma máquina cíclica
caía na ordem de declaração, e um `#[default]` numa variante posterior pintava a
entrada errada. Acertava por coincidência sempre que o default também era a primeira
variante, que é o caso comum e é por isso que passou despercebido.

Fechadas como um incremento só, porque têm uma causa: faltava a evidência no modelo.
`StateDecl.is_default` (`default` no wire) agora carrega o que a fonte declara; os
clientes derivam o papel a partir dela. As duas derivações —
`crates/docgen/src/roles.rs` para os geradores, `apps/web/src/domain/stateRole.ts`
para a UI — leem os mesmos três passos: default declarado, depois um estado para o
qual nada transiciona, depois o primeiro estado declarado. O Markdown ganhou uma
coluna `Papel` ao lado de `Marcadores` (declarado e derivado ficam em colunas
separadas de propósito) e o Mermaid ganhou `[*] --> Entrada` / `Beco sem saída --> [*]`.

Invariantes fáceis de achatar por acidente:

- **`default` é evidência, não documentação.** `is_documented()` a ignora
  deliberadamente, então derivar `Default` nunca melhora um número de cobertura nem
  faz a tabela de estados aparecer para uma máquina sem documentação. Voltar a
  dobrá-la em `is_bare()`-como-`!is_documented()` — eram a mesma função antes disto —
  infla a cobertura em silêncio.
- **Só uma folha é marcada.** `#[derive(Default)]` aceita `#[default]` apenas em
  variante unitária, então um composto nunca pode ser o default declarado. O parser
  ainda checa pertinência antes de marcar, então uma entrada hostil ou que não
  compila não marca nada em vez de nomear um `Active` que o modelo não declara.
- **As duas derivações têm de continuar idênticas.** São os mesmos três passos em
  duas linguagens; mudar uma sem a outra põe o canvas e o documento gerado em
  desacordo, que é exatamente a lacuna que isto fechou.
- **A ordem de declaração continua por último.** Ela é a evidência mais fraca, não o
  primeiro fallback: num ciclo não significa nada, que é toda a razão de `default`
  estar no contrato.

---

## 7. Deliberadamente ainda não

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
- **Fuzzing do parser.** `cargo-fuzz` sobre `parse_project` é o sucessor natural
  da §4b: os limites de recursos e a verificação prévia de aninhamento foram
  encontrados escrevendo entradas hostis *à mão*, e um fuzzer encontra as que
  ninguém pensou. Adiado, não recusado — precisa de orçamento de CI (um job de
  fuzz não é um gate de 60 segundos) e de um corpus de sementes para valer algo, então fica
  depois da distribuição em vez de espremido no `just check`.

---

## 8. O que a adoção encontrou — um core de produção com 13 máquinas

Rodar a ferramenta contra uma aplicação Crux real e privada (13 máquinas, 197
transições, 63 estados, ~711 menções de efeito) produziu
[plans/adoption-findings.md](../plans/adoption-findings.md) — em inglês, como todo
`docs/plans/`: treze achados, seis deles bugs — e desses seis, **dois** (P1, P2)
contradizem comportamento que o `docs/parser.md` já documenta, o que é o que os
torna os que valem desconfiar da documentação. Esse documento é **evidência**, não
um tracker: os números dele não são re-medidos conforme as correções entram, e o
status vive aqui. Ele foi corrigido uma vez depois de escrito — dois erros de
aritmética nas próprias contagens de efeito, e duas evidências acrescentadas (a
lacuna irmã do ramo `else`, no P1, e a medição por estado, no D2) — então não está
congelado em `cf4f914` no sentido literal, só no sentido de descrever o
comportamento daquele commit.

Duas regras valem para a frente inteira. Cada achado ganha uma **fixture
versionada escrita antes da correção**, com saída esperada commitada — as fixtures
usam nomes inventados, então nenhuma cai na regra dos `_hidden` não versionados, e
o P1 é exatamente a classe de bug que regride em silêncio. E o trabalho entra
**agrupado por causa, não por achado**: `{P3a+P3b}`, `{D1+P6}`, `{P1+P2}`,
`{P4}`, `{D2}`, `{D3+D4}`, `{M1–M3}`, `{proveniência do D5}` — cada um um
incremento com uma história que uma mensagem de commit consegue contar.

A ordem é a mais barata primeiro, como o plano propõe, com o **P5 promovido ao
primeiro lugar**, pelo motivo de ser o único achado sem reprodução: reduzir a
árvore real a uma fixture é o passo que decai, porque depende de um checkout que
outra pessoa controla. Não por destravar nada — a leitura anterior, de que era "o
aviso que mantém o CI de um adotante vermelho", não sobrevive à checagem. Dois
avisos reprovam aquele build, e o outro é o `dynamic-target` do sítio do P4, que a
correção do P4 mantém de propósito: aquele ramo realmente atribui um valor de
runtime. Nenhuma sequência de correções aqui deixa aquele build verde; um recipe
que gera mais um gate separado que checa é a jogada do adotante.

D2 e D3 são re-medidos depois do P1 em vez de ajustados contra os números de hoje,
já que boa parte do que os degenera é um guard clause que deveria ter
estreitado.

### 8.1 Parser

- **P5 — um callback que resolve isolado mas não na árvore** 🔍. Três fixtures não
  conseguiram reproduzir, então a árvore real é reduzida a uma fixture em nomes
  inventados e só a fixture é commitada. Comece pela observação de que a resolução
  nomeia a variante **envelope** em vez dos eventos envelopados.
- **P1 — um guard clause não estreita nada** 🐞. `if <condição negada> { return … }`
  antes da atribuição não publica restrição alguma, então a transição sai como
  wildcard `"*"`: 100 de 197 transições daquele core têm origem wildcard e 6 de 13
  máquinas são inteiramente assim — e adotantes já estavam contorcendo handlers
  para satisfazer uma ferramenta que depois descartava o guard de qualquer jeito. O
  `Ctx.conditions` ganha um sinal de polaridade (`{expr, negated}`) e o
  `eval_condition` inverte o `GuardEval` quando negado. A negação é publicada em
  **dois** lugares: por um bloco `then` que diverge, para o resto do bloco que o
  contém (o mesmo tempo de vida que a linha do let-else já promete), e pelo `else`
  de qualquer `if` — a segunda é uma lacuna irmã que os achados não nomeiam, já que
  o `Expr::If` empurra a condição só para o ramo `then`.
- **P2 — o parâmetro de um closure é comparado por nome** 🐞. Evidência de guard
  dentro de `find(|d| …)` só conta quando o parâmetro é escrito igual ao binding
  pelo qual o resultado é atribuído, o que torna a análise sensível a rename. O
  parâmetro de um closure passado à chamada cujo resultado é vinculado *é* o
  elemento que aquele binding recebe, então os dois são unificados **por posição,
  com qualquer nome**. Identidade estrutural, então a regra permissiva de receiver
  da §6 fica intacta no resto. Decidido junto com o P1: a forma 6 da fixture do
  plano falha pelas duas regras ao mesmo tempo.
- **P3a + P3b ✅ feito.** `is_effect_request_enum` e `declares_variant`
  (`crates/parser/src/core_finder.rs`) estreitam o que o `record_effect_path` pode
  registrar; o fecho mantém a associação inteira, já que o `emit` a lê para achar o
  doc comment escrito numa variante. Fixture primeiro, como a regra manda:
  `crates/parser/fixtures/effect_requests/` com `tests/effect_requests.rs`, cujos
  quatro casos são as duas formas que têm de sair (variante de payload em
  profundidade 2 e em 3, função associada num enum de operação) e as duas que têm
  de sobreviver (operação carregada pela raiz, `render()` pelado). Re-medido contra
  o core de onde os achados vieram: **711 → 441 menções, exatamente as 270
  previstas**, 23 nomes descartados, nenhum novo registrado, nenhuma máquina
  perdida. O documento caiu de 131 KB para 106 KB, o maior diagrama de 10,6 KB para
  5,9 KB, e o rótulo de aresta mais longo de 473 para 302 caracteres — que é para o
  que o limite do D3 continua servindo.
- **P3a — o fecho de efeitos não tem limite de profundidade** 🐞. Enums de payload
  alcançados transitivamente entram em `effect_enums`, então qualquer menção
  posterior a uma variante deles é registrada como pedido de efeito — **270 de
  711** menções naquele core (as 228 de enums de dados e funções associadas mais as
  42 do `TelemetrySignal`, que é payload de um pedido e não irmão dele). O
  predicado de registro passa a ser
  `name == effect_root || capability_of(name).is_some()`. A primeira cláusula
  sustenta o resto porque `capability_of` devolve `None` para duas coisas
  diferentes — um enum de payload que ninguém envolve, e a própria raiz — então sem
  ela um app cuja raiz carrega operações como variantes próprias
  (`Effect::StartAudio { .. }`) perde todo efeito que tem. Não, como estava escrito
  aqui antes, por causa do `Render`: um `render()` pelado nunca chega ao
  `record_effect_path`, sendo registrado pelo `record_effect` em
  `transitions.rs:603` com rótulo literal. A fixture que discrimina é uma raiz com
  variante-operação própria, e uma que afirme "o `Render` sobrevive" passaria de
  qualquer jeito. Enums mais profundos simplesmente deixam de ser registrados;
  documentá-los como tipos de payload é decisão separada, e remover um falso
  positivo não deve nenhum `Warning`.
- **P3b — funções associadas registradas como variantes** 🐞. O
  `record_effect_path` aceita o que o `enum_variant_path` devolver, então
  `FailureDomain::of` e `ApiFailure::from` são reportados como coisas que a shell é
  pedida a executar. Comparar o último segmento com `decl.variants` corrige isso
  sem nenhum falso negativo possível. Barato, e não de alto retorno — o que corrige
  o argumento de ordenação do plano: os cinco nomes que ele pega naquele core são
  todos enums de payload em profundidade ≥ 2, então o P3a já remove cada uma das
  122, e a contribuição marginal dele ali é zero. O que ele pega sozinho é uma
  função associada num enum de operação de *profundidade 1*
  (`AudioOperation::of(..)`), que é igualmente errado e simplesmente não ocorre
  nesta amostra.
- **P4 — um ramo dinâmico derruba a máquina inteira** 🐞. `model.filter = if cond
  { Filter::All } else { filter }` perde os dois ramos, e uma máquina sem transição
  nunca chega ao modelo: 14 máquinas na fonte, 13 na saída. O ramo literal é
  emitido como transição real, o irmão irresolúvel recebe a nota de alvo wildcard
  que já existe, e um **novo tipo de aviso** dispara quando uma máquina termina sem
  transições — o diagnóstico de hoje nomeia uma transição enquanto o que se perde é
  uma máquina.
- **P6 — avisos emitidos mais de uma vez** 🐞. Deduplicados em
  `(arquivo, linha, tipo)` antes do relato: três linhas idênticas se leem como três
  problemas.

### 8.2 Docgen

- **D1 — arestas duplicadas byte a byte** 🐞. Transições idênticas a não ser pelo
  `resolves_with`, que o rótulo não renderiza, desenham duas setas uma sobre a
  outra. Corrigido nos dois níveis: fundidas no modelo (união das respostas — dois
  caminhos de chamada até o mesmo helper não são duas transições) e ignoradas no
  `machine_diagram`, porque uma linha renderizada idêntica não carrega informação,
  diga o modelo o que disser. De passagem, verificar se um helper compartilhado
  alcançado por dois caminhos é *caminhado* duas vezes — a mesma suspeita por trás
  do P6, e se ela se confirmar as contagens de efeito também estão infladas.
- **D2 — o papel `final` é degenerado em máquinas movidas por wildcard** ⚖️
  **reaberto**. 34 estados marcados como finais, quatro máquinas marcando todos os
  seus, um estado chamado `Downloading` entre eles. A primeira decisão aqui —
  nenhum papel `final` para máquina cujas transições são todas de origem wildcard,
  as outras mantêm o comportamento de hoje — foi medida contra o core depois e
  mantém 11 das 34 marcas, **8 delas numa única máquina**: a que marca 8 dos seus 9
  estados como finais tem uma única transição wildcard em 7, então a regra não se
  aplica a ela, e essa transição é `* -- InsightsUpdated -> *`, wildcard na origem
  *e* no destino. Todo estado sai por ali. A regra é por máquina e a degeneração é
  por estado. A leitura estrita por estado (nenhum papel para estado de que um
  wildcard possa sair) mantém 0 das 34 aqui, já que toda máquina daquele core tem
  ao menos uma transição de origem wildcard — ou seja, esvazia o recurso em vez de
  corrigi-lo. O que aponta para o papel não ser binário: "nada sai deste estado
  **por nome**" é um fato real e diferente de "terminal", então pertence à tabela de
  estados sob uma palavra que diga isso, enquanto o `X --> [*]` só é desenhado onde
  nenhum wildcard pode sair. Um acoplamento a corrigir junto — o diagrama desenha
  papéis sem condição alguma enquanto a tabela de estados é gated em
  `has_documented_states`, então a máquina que detém 7 dessas marcas as afirma no
  único lugar em que um leitor não pode conferi-las. Segue re-medido depois do
  P1.

  **Decidido: vocabulário de dois níveis.** A tabela de estados mantém o fato sob
  uma palavra que o enuncia — nada sai deste estado *por nome* — e o `X --> [*]` só
  é desenhado onde nada sai, wildcard incluído. As duas leituras são verdadeiras e
  são leituras diferentes, e é por isso que uma palavra só não dava conta: o fato é
  evidência que a forma realmente fornece, e "terminal" é uma inferência que a
  forma contradiz no instante em que existe um wildcard. Mantém as 34 marcas, como
  afirmações verdadeiras, e não desenha nenhuma das setas falsas. A regra estrita
  foi recusada por esvaziar o recurso (0 de 34 aqui) e a por-máquina por perder
  exatamente a máquina que mais precisa dela. Custa um rótulo nos dois catálogos de
  locale e a mudança correspondente em `apps/web/src/domain/stateRole.ts` — o
  `roles.rs` diz mude um, mude os dois, e esta é a mudança que mostra por quê.
- **D3 — rótulos de aresta sem limite** ⚖️. Os efeitos em um rótulo de aresta são
  limitados a 3 com o sufixo `+n more`, espelhando o `ANSWERS_IN_A_CELL`; a tabela
  segue completa por contrato. Descartar o `Render` foi **recusado**: elidi-lo por
  nome é uma convenção de nomes em um projeto que recusa convenções de nome, e
  elidi-lo estruturalmente (a operação carregada pela raiz) apagaria todo efeito de
  um app cuja raiz carrega operações direto. O P3 já remove um terço do ruído.
- **D4 — `\n` como quebra de linha do rótulo depende do renderizador** ⚖️. Emitido
  como `<br/>`, que funciona nos dois caminhos de rótulo do mermaid; o rótulo é
  montado a partir de partes escapadas unidas pela tag crua, então a prosa do autor
  segue escapada e só o separador é markup.
- **D5 — um documento de 1027 linhas sem índice e sem proveniência** ⚖️. Dividido,
  já que metade não depende de nada: o sumário e as âncoras por máquina entram por
  conta própria, e os links de proveniência mais a de-duplicação da prosa (o
  primeiro parágrafo de um estado é renderizado três vezes) entram sobre o M2.

### 8.3 Modelo — um incremento, por último

Os três campos movem o contrato **uma vez**, depois do P1 e do P3, para serem
desenhados contra dados corrigidos em vez da saída de hoje com 51% de wildcard.
Todos os três são aditivos e opcionais, então nenhum cliente quebra.

- **M1 — `Transition` não tem guard** ⚖️. `guard: Option<String>` carregando a
  condição como escrita, renderizada como `Event [guard] / effects`. É prosa não
  confiável: escapada em toda fronteira e limitada em tamanho, com um `Warning`
  quando o limite dispara. É isto que torna o P1 *visível* onde um único estado de
  origem se abre em leque sobre um evento.
- **M2 — sem span de origem** ⚖️. `source: { file, line }`, com `file` relativo à
  raiz `src` analisada — nunca absoluto. Caminhos saem de uma árvore não confiável,
  e um caminho relativo também mantém o `model.json` reproduzível entre máquinas.
- **M3 — sem caminho até a máquina** ⚖️. Um singleton em `flags.identity` e uma
  instância por registro em `drafts[].submission.status` renderizam idênticos hoje.
  O parser emite o caminho do campo **e** a cardinalidade que derivou de esse
  caminho atravessar uma coleção: a lição da §6b foi que a mesma derivação escrita
  duas vezes em duas linguagens se desencontra.
- **A parte do web.** `apps/web/src/schema/` e o modelo de domínio aceitam os três
  no mesmo incremento; o Inspector renderiza **só o guard** — o que falta ao leitor
  quando três setas compartilham um evento. Span e cardinalidade esperam uma
  decisão de UI.

### 8.4 Uma coisa que é só documentação

Aquela execução reportou 79% de cobertura de documentação com a maior máquina em 0
de 7 estados descritos, e nada falhou por isso: o `coverage --min` nunca foi ligado
ao recipe `check` do adotante. O `docs/cli.md` e seu gêmeo pt-BR ganham a
orientação de ligação ao lado do `docs --deny-warnings`, porque a adoção
evidentemente não descobre isso sozinha.
