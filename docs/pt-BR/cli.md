# CLI — `crux-analyzer`

> 🌐 [English](../cli.md) · **Português (Brasil)**

Construída por `crates/cli`. Rode a partir do workspace com
`cargo run -p crux-analyzer-cli --` (ou instale o binário com
`cargo install --path crates/cli`).

## `generate` — emitir o modelo JSON

```sh
crux-analyzer generate --src <dir> [--name <projeto>] [--out <arquivo>] [--watch] [--locale <locale>]
```

| Flag | Significado |
| --- | --- |
| `--src` | Diretório com as fontes Rust a analisar (por exemplo, `caminho/para/app/shared/src`). |
| `--name` | Nome do projeto no modelo. Por padrão, o nome do diretório de `--src`. |
| `--out` | Arquivo de saída. Por padrão, stdout. |
| `--watch` | Continua observando `--src` e regenera a cada mudança em `.rs` (com debounce). |
| `--locale` | `en` ou `pt-BR`. Idioma da saída da própria CLI e da prosa gerada — veja abaixo. |

Avisos (veja a [referência de avisos](parser.md#referência-de-avisos)) vão para
stderr; o JSON vai para `--out`/stdout. O código de saída é diferente de zero
quando o parsing falha (por exemplo, nenhum `impl App` encontrado).

O **modelo JSON emitido é independente de locale** — tudo nele é lido da fonte
analisada, tanto identificadores quanto a prosa dos comentários de documentação
do próprio autor, então `generate` produz saída byte a byte idêntica em todo
locale. Aqui `--locale` afeta somente as mensagens em stderr.

Alimentar a UI web:

```sh
crux-analyzer generate --src caminho/para/app/src --name MeuApp \
  --out apps/web/public/model.json
```

## `docs` — emitir documentação

```sh
crux-analyzer docs --src <dir> [--name <projeto>] [--format markdown|mermaid] [--out <arquivo>] [--watch] [--locale <locale>]
```

Aqui `--locale` também traduz a prosa do próprio documento gerado (rótulos de
seção, cabeçalhos de tabela, nomes de marcadores, o pseudo-estado `qualquer
estado`). Nomes de estados, eventos e efeitos ficam intocados, **e a documentação
lida da fonte analisada também** — a língua de um comentário de documentação é
escolha de quem o escreveu. O id do nó Mermaid `any_state` permanece estável
porque as linhas de transição se referem a ele.

### Markdown (padrão)

Um documento: por máquina, sua descrição, um bloco ` ```mermaid `, uma tabela de
estados e uma tabela de transições (De / Evento / Para / Efeitos). GitHub, GitLab
e a maioria dos visualizadores de Markdown renderizam os diagramas embutidos
nativamente — faça commit do arquivo e a documentação fica legível no
repositório:

```sh
crux-analyzer docs --src caminho/para/app/src --name MeuApp --out MAQUINAS_DE_ESTADO.md
```

A descrição e a tabela de estados aparecem apenas quando a fonte analisada
documenta algo — veja [anotações](parser.md#documentação-e-anotações) para como
escrevê-las. Duas consequências de copiar a prosa do autor literalmente: uma
descrição é achatada em uma linha na célula da tabela (um estado cuja descrição
tem vários parágrafos a recebe de volta por inteiro, sob um título próprio abaixo
da tabela), e o Markdown que o autor escreveu continua sendo Markdown, então um
comentário de documentação que começa com `#` renderiza como título.

### Mermaid (bruto)

Fontes `stateDiagram-v2` cruas, um diagrama por máquina, separadas por
cabeçalhos de comentário `%% Core / Máquina`:

```sh
crux-analyzer docs --src caminho/para/app/src --format mermaid --out maquinas.mmd
```

```
%% Recorder / RecorderState
stateDiagram-v2
    Idle --> Recording: RecordPressed
    ...

%% Recorder / InputState
stateDiagram-v2
    state "qualquer estado" as any_state
    ...
```

Para visualizar: cole um único diagrama em [mermaid.live](https://mermaid.live),
ou divida o arquivo nos cabeçalhos `%%` para embutir. Estados compostos são
renderizados como blocos aninhados (`state Active { ... }`); origens/destinos
curinga são renderizados como um pseudo-estado `qualquer estado`.

### Documentação viva

Ambos os comandos aceitam `--watch`: combinado com um arquivo `--out` versionado
ou com o `model.json` da UI web, a documentação se regenera a cada save.

## Escolhendo o locale

Precedência, do maior para o menor:

1. `--locale en|pt-BR`;
2. a variável de ambiente `CRUX_ANALYZER_LOCALE`;
3. a cadeia POSIX `LC_ALL` → `LC_MESSAGES` → `LANG` (então `LANG=pt_BR.UTF-8` já
   basta);
4. `en`.

Valores de *ambiente* não reconhecidos são ignorados e a cadeia continua; um
`--locale` não reconhecido é um erro, porque ignorar em silêncio um pedido
explícito seria pior. Note que o próprio `--help` está apenas em inglês — veja a
[lacuna documentada em i18n.md](i18n.md#lacuna-conhecida---help-só-em-inglês).

## Exemplo (rodando contra o fixture de teste)

```sh
cargo run -p crux-analyzer-cli -- docs \
  --src crates/parser/fixtures/mini_recorder --name "Mini Recorder" --locale pt-BR
```

A saída versionada exatamente desse comando vive em
[docs/pt-BR/examples/mini-recorder.md](examples/mini-recorder.md), com sua gêmea
em inglês em [docs/examples/mini-recorder.md](../examples/mini-recorder.md).
Ambas são regeneradas por `just example-docs`.
