# Segurança

> 🌐 [English](../security.md) · **Português (Brasil)**

O crux_analyzer ocupa uma posição de exposição incomum para uma ferramenta de
desenvolvimento. Ele **lê código Rust que não escreveu**, **emite documentos que
são publicados** e **renderiza a prosa desse código em um navegador e num webview
do VS Code**. Nenhuma dessas coisas é uma "entrada" comum — cada uma é uma
fronteira de confiança, e este documento diz onde elas estão, quais são as regras
e o que é garantido deliberadamente.

As regras aqui são diretrizes de desenvolvimento, não aspirações: a maioria tem um
teste que falha quando é violada, e o `just check` executa o gate de cadeia de
suprimentos.

## Modelo de ameaça

**Entradas não confiáveis** — tudo abaixo pode ser hostil, malformado ou
simplesmente patológico:

| Entrada | Por que não é confiável | Chega em |
| --- | --- | --- |
| A árvore de fontes analisada | Uma dependência, um pull request de um fork, um crate que alguém baixou. A forma dela controla a AST. | `crates/parser` |
| Caminhos dentro dessa árvore | Nomes de arquivo, links simbólicos, tamanhos | `crates/parser/src/loader.rs` |
| Prosa de doc comments | Texto livre escrito por quem escreveu o código | saída do docgen, a UI web, o webview |
| Identificadores (nomes de estado/evento/efeito/máquina/núcleo) | Qualquer identificador Rust legal, inclusive raw e não-ASCII | saída do docgen, a UI web |
| `model.json` | Pode estar velho, editado à mão ou de outra versão | `apps/web/src/schema/` |
| O `.vscode/settings.json` de um workspace | Vem dentro de um repositório clonado | `apps/vscode` |

**Entradas confiáveis**: as próprias flags e o ambiente da CLI (`--src`, `--out`,
`--max-*`, `CRUX_ANALYZER_LOCALE`). Quem pode definir isso já pode executar
comandos arbitrários.

**Fora do modelo de ameaça**: o comportamento em *tempo de execução* da aplicação
analisada. O crux_analyzer nunca executa o código que lê — apenas o analisa — e
não depende do próprio Crux.

## As regras

### 1. Prosa do autor é texto não confiável, em todo lugar onde chega

Doc comments são texto livre a caminho de um navegador e de um documento
publicado. Eles nunca podem se tornar marcação.

- **Na UI web**: a prosa chega ao DOM apenas como filhos React, ou via
  `react-markdown` com HTML bruto desabilitado. Nunca
  `dangerouslySetInnerHTML`, nunca `rehype-raw`, nunca malabarismo com
  `skipHtml`, e nunca um `urlTransform` que amplie a lista de protocolos além de
  `http`, `https` e `mailto`. Links levam
  `rel="noopener noreferrer nofollow"`; imagens **não são buscadas** — um
  `![](https://host/x.png)` num doc comment é um rastreador que reportaria cada
  leitor de um documento publicado, então o texto alternativo fica no lugar
  dela. Fixado por
  [`StateDoc.test.tsx`](../../apps/web/src/components/Inspector/StateDoc.test.tsx).
- **No Markdown gerado**: `&`, `<` e `>` são escapados na prosa, então HTML bruto
  não pode se tornar um elemento. O *Markdown* do autor é preservado
  deliberadamente — `**negrito**`, listas e backticks são uma funcionalidade —
  então isto não é um escape da sintaxe Markdown, apenas da capacidade de sair
  dela. Linhas em forma de cerca são neutralizadas, e a cerca em volta de um
  diagrama é calculada para ser mais longa que qualquer sequência de backticks
  dentro dele.
- **No Mermaid gerado**: rótulos e notas passam por `mermaid_label`, que achata
  para uma linha (um comando termina na linha), remove caracteres de controle e
  troca `"`, `<`, `>` e `%%` por códigos de entidade.
- **Em células de tabela**: a barra invertida é escapada *antes* do pipe, ou
  prosa contendo `\|` reabre uma coluna. Backticks também são escapados.

Fixado por [`hostile_output.rs`](../../crates/docgen/tests/hostile_output.rs).

### 2. Identificadores são dados, e dados não se tornam estrutura

Um identificador da aplicação analisada nunca pode influenciar um caminho de
arquivo, e nunca pode ser emitido onde seus caracteres possam ser lidos como
sintaxe.

- A única escrita no workspace Rust é o `--out` do usuário. O docgen retorna
  strings e nunca toca o sistema de arquivos; não existem arquivos de saída por
  máquina. **Mantenha assim** — um nome de núcleo ou estado num caminho é um
  path traversal.
- Ids de nó do Mermaid são gerados, verificados contra colisão e contra palavras
  reservadas (`Ids::build`), com o nome real carregado num rótulo entre aspas. Um
  estado chamado `end`, um identificador raw (`r#type`) ou uma folha composta
  colidindo com uma irmã quebrariam o diagrama ou fundiriam nós silenciosamente.

### 3. Toda dimensão ilimitada de entrada recebe um limite, e todo limite acionado é reportado

Esta é a **regra de honestidade do parser aplicada a recursos**: uma análise
truncada diz que foi truncada, como um `Warning`, então `--deny-warnings` faz o
truncamento falhar um pipeline em vez de publicar um diagrama silenciosamente
parcial.

Os limites vivem em [`crates/parser/src/limits.rs`](../../crates/parser/src/limits.rs)
e são ajustáveis com `--max-file-size`, `--max-total-size` e `--max-steps`:

| Dimensão | Por que é ilimitada sem um limite | Aviso |
| --- | --- | --- |
| Tamanho de arquivo e total | A AST de cada arquivo é mantida durante toda a execução, e uma AST é muito maior que seu fonte | `file-too-large`, `input-too-large` |
| Aninhamento de delimitadores | `syn::parse_file` recursa sobre aninhamento; um stack overflow **aborta o processo** e não pode ser capturado, então este é verificado no texto bruto *antes* do parsing | `nesting-too-deep` |
| Passos da caminhada | O walker que segue chamadas re-percorre um helper por caminho distinto, então um grafo de chamadas em diamante é exponencial — quarenta funções pequenas descrevem 2⁴⁰ caminhadas | `analysis-truncated` |
| Profundidade de expressão / padrão / chamada | Os walkers recursam sobre o aninhamento da própria entrada | `analysis-truncated` |

Memoizar o walker *não* é a alternativa: um helper é legitimamente re-percorrido
sob um contexto diferente e produz transições diferentes a cada vez. O que é
limitado é o trabalho total — e, diferente da memoização, um orçamento é
reportável.

Fixado por [`hostile_input.rs`](../../crates/parser/tests/hostile_input.rs), que
verifica terminação, não qualidade de extração.

### 4. Só arquivos regulares são lidos

O `walkdir` não desce por *diretórios* com link simbólico, mas um *arquivo* com
link simbólico seria seguido — lendo fonte de fora da árvore, travando para
sempre num FIFO ou esgotando a memória em `/dev/zero`. Uma única verificação
`file_type().is_file()` fecha os três casos. Caminhos ignorados são reportados
(`not-a-regular-file`), nunca descartados em silêncio.

### 5. Nunca um shell

Subprocessos recebem um array de argumentos. O workspace Rust não executa nada —
sem `Command::new`, sem shell. A extensão do VS Code usa `execFile` com array de
argumentos. Uma string passada a um shell é uma injeção de comando esperando pelo
primeiro caminho com espaço.

### 6. Configurações que escolhem um executável têm escopo de máquina

`cruxAnalyzer.binary` é `"scope": "machine"`, então o `.vscode/settings.json` de
um repositório clonado não decide qual executável roda — a classe de problema do
`nodePath` do ESLint. `cruxAnalyzer.src` tem escopo de workspace (é genuinamente
por projeto) e por isso é *contido*: um valor que sobe além da raiz do workspace
com `..` é recusado, porque o watcher também o segue. A extensão declara
`untrustedWorkspaces.supported: false`.

### 7. Diagnósticos são sanitizados antes de chegar a um terminal

Um doc comment ou um caminho interpolado num aviso é texto controlado pelo
atacante sendo escrito no terminal de alguém. `WarningKind::message` e
`ParseError::message` removem caracteres de controle da string renderizada
*inteira*, para que uma variante adicionada depois não possa esquecer.

### 8. O webview permanece trancado

`default-src 'none'`, scripts sob um nonce por renderização, sem `unsafe-inline`
nem `unsafe-eval` para scripts, `localResourceRoots` limitado ao diretório do
bundle. O modelo injetado escapa `<` e U+2028/U+2029 para que a prosa do autor
não possa fechar a tag de script nem quebrar o comando. **Não existe canal de
mensagens webview↔host** — o modelo flui em uma direção, por injeção. Não
adicione um sem validar cada mensagem.

### 9. Dependências são revisadas, e actions são fixadas

- Novas dependências precisam passar no `just security` (`cargo deny check`
  contra [`deny.toml`](../../deny.toml) + `pnpm audit --audit-level high`). Faz
  parte do `just check`, então é bloqueante.
- Os dois lockfiles são versionados e o `cargo` roda com `--locked`.
- GitHub Actions são fixadas em **SHAs de commit**, não em tags — `@stable` e
  `@v2` são refs mutáveis. O Dependabot evita que os pins apodreçam.
- Workflows declaram `permissions:` explicitamente, e valores `${{ }}` não
  confiáveis chegam a um bloco `run:` via `env:`, nunca por interpolação de
  string.

## O que é garantido deliberadamente

Estas propriedades são estruturais. São baratas de manter e caras de recuperar,
então trate remover uma como mudança de design, não como refatoração:

- **Nenhum `unsafe`** no workspace Rust.
- **Nenhum subprocesso ou shell** no workspace Rust.
- **Nenhum caminho de saída influenciável por um atacante**: um `fs::write`, a
  partir de `--out`.
- **Nenhum `unwrap`/`expect`/`panic!`/índice de slice alcançável** sobre entrada
  parseada. Os `expect` restantes são comprovadamente inalcançáveis e dizem isso.
- **Nenhum sink de injeção de HTML** para dados do modelo na UI web: sem
  `dangerouslySetInnerHTML`, sem `innerHTML`, sem `href`/`src` construídos a
  partir de dados do modelo.
- **Nenhum `eval`, `new Function` ou import dinâmico** de strings derivadas do
  modelo.
- **Nenhum canal de mensagens webview↔host.**
- **`localStorage` e a entrada do hash da URL são validados por lista de
  permissão** antes do uso, tanto nos scripts inline de pré-pintura quanto na
  aplicação.

## Como relatar uma vulnerabilidade

Abra um [security advisory](https://github.com/josecleiton/crux_analyzer/security/advisories/new)
em vez de uma issue pública, e por favor inclua a entrada que dispara o problema —
um arquivo `.rs` mínimo ou um `model.json` vale mais que uma descrição. Não há
recompensa; há uma entrada no changelog e agradecimentos.

## Veja também

- [Parser](parser.md) — a referência de avisos, incluindo os avisos de recursos
- [CLI](cli.md) — as flags `--max-*` e `--deny-warnings`
- [Desenvolvimento](development.md) — o `just check` e o pipeline de validação
- [Arquitetura](architecture.md) — por que existem as camadas que tornam estas
  regras aplicáveis
