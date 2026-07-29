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
`parse_project`) seria o próximo passo natural e está listado na §6.

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

### 5.5 Já vencido, não trabalho futuro

Duas obrigações de licença estão descumpridas **hoje**, então são defeitos e não
planos:

- **O aviso EPL-2.0 do elkjs não está no bundle compilado.** O Vite o remove e
  `apps/web/dist/` não carrega nenhum NOTICE, mas a EPL-2.0 §3.1/§3.2 exige que
  quem recebe o código objeto receba o texto da licença. O `README.md` atribui o
  elkjs corretamente, e o README não viaja com o artefato — então o preview no
  Pages vem redistribuindo o elkjs a descoberto em todo push para `main`. Um
  `THIRD-PARTY-NOTICES.md` versionado e copiado para `dist/` pelo `web-build`
  cobre o Pages, todo VSIX (`media/web` já está na lista de permissão) e qualquer
  arquivo de release de uma vez.
- **Todo VSIX publica código MIT sem o texto da licença.** O `.vscodeignore` já
  permite `!LICENSE*`, mas `apps/vscode/LICENSE` não existe. Consertar do jeito
  que o bundle web já é tratado — copiado no momento do build pelo `ext-build`,
  para haver uma fonte de verdade só e nada para divergir.

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

## 6. Deliberadamente ainda não

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
  fuzz não é um gate de 60 segundos) e de um corpus para valer algo, então fica
  depois da distribuição em vez de espremido no `just check`.
