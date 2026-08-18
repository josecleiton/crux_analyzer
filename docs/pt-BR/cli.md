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
| `--deny-warnings` | Sai com código diferente de zero se o parser reportou algo. Global: funciona em todos os subcomandos. |
| `--max-file-size` | Ignora arquivos `.rs` maiores que este número de bytes (padrão 2 MiB). Global. |
| `--max-total-size` | Para de ler quando esta quantidade de bytes de fonte tiver sido carregada (padrão 256 MiB). Global. |
| `--max-steps` | Passos de caminhada por expressão permitidos por núcleo (padrão 2.000.000). Global. |

Avisos (veja a [referência de avisos](parser.md#referência-de-avisos)) vão para
stderr; o JSON vai para `--out`/stdout. O código de saída é diferente de zero
quando o parsing falha (por exemplo, nenhum `impl App` encontrado).

### Limites de recursos

As flags `--max-*` existem porque o analisador é rotineiramente apontado para
código que ninguém do seu time escreveu — uma dependência, um pull request de um
fork, um crate que alguém baixou. Os padrões estão muito acima de qualquer
aplicação Crux real (a fixture de teste usa uma contagem de passos de quatro
dígitos) e muito abaixo do necessário para travar uma máquina, então **você nunca
deve precisar mudá-los**; aumente um apenas para uma base de código grande em que
você confia.

Todo limite obedece à regra da honestidade: acioná-lo emite um aviso
([avisos de recursos](parser.md#avisos-de-recursos)), então o `--deny-warnings`
faz uma análise truncada falhar a execução em vez de publicar um diagrama
silenciosamente parcial. O `docs/security.md` explica o que cada limite delimita e
por que memoização não é a alternativa.

O `--deny-warnings` ainda escreve a saída — o código de saída é o sinal, para que
um pipeline falhe enquanto uma pessoa ainda recebe o artefato para olhar. Sob
`--watch` ele reporta sem encerrar a sessão.

O **modelo JSON emitido é independente de locale** — tudo nele é lido da fonte
analisada, tanto identificadores quanto a prosa dos comentários de documentação
do próprio autor, então `generate` produz saída byte a byte idêntica em todo
locale. Aqui `--locale` afeta somente as mensagens em stderr.

Alimentar a UI web:

```sh
crux-analyzer generate --src caminho/para/app/src --name MeuApp \
  --out apps/web/public/model.json
```

## `site` — emitir site estático de documentação web

```sh
crux-analyzer site --src <dir> [--name <projeto>] [--out <diretório>] [--watch] [--locale <locale>]
```

| Flag | Significado |
| --- | --- |
| `--src` | Diretório com as fontes Rust a analisar. |
| `--name` | Nome do projeto no modelo. Por padrão, o nome do diretório de `--src`. |
| `--out` | Diretório de saída. Por padrão, `dist`. |
| `--watch` | Continua observando `--src` e regenera `model.json` a cada mudança em `.rs`. |

Exporta os arquivos da UI web interativa e insere o `model.json` analisado no diretório `--out`.

```sh
crux-analyzer site --src caminho/para/app/src --name MeuApp --out ./docs-publicos
```

## `docs` — emitir documentação

```sh
crux-analyzer docs --src <dir> [--name <projeto>] [--format markdown|mermaid|html|site] [--out <arquivo/diretório>] [--watch] [--locale <locale>]
```

Aqui `--locale` também traduz a prosa do próprio documento gerado (rótulos de
seção, cabeçalhos de tabela, nomes de marcadores, o pseudo-estado `qualquer
estado`). Nomes de estados, eventos e efeitos ficam intocados, **e a documentação
lida da fonte analisada também** — a língua de um comentário de documentação é
escolha de quem o escreveu. O id do nó Mermaid `any_state` permanece estável
porque as linhas de transição se referem a ele.

### Markdown (padrão)

Um documento: por máquina, sua descrição, um bloco ` ```mermaid `, uma tabela de
estados (Estado / Papel / Descrição / Marcadores / Etiquetas) e uma tabela de
transições (De / Evento / Para / Efeitos). GitHub, GitLab
e a maioria dos visualizadores de Markdown renderizam os diagramas embutidos
nativamente — faça commit do arquivo e a documentação fica legível no
repositório:

```sh
crux-analyzer docs --src caminho/para/app/src --name MeuApp --out MAQUINAS_DE_ESTADO.md
```

A coluna `Papel` carrega os dois papéis *derivados* — `inicial` para o ponto de
entrada da máquina, `final` para um estado do qual nada sai — mantidos separados de
`Marcadores`, que é o que o autor declarou. O diagrama afirma os mesmos dois pelo
pseudo-estado `[*]` do Mermaid, então ele diz onde uma máquina começa e termina até
para uma máquina sem documentação alguma. Como o ponto de entrada é decidido está
em [schema.md](schema.md#onde-uma-máquina-começa).

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
    [*] --> Idle
    Completed --> [*]

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

## `coverage` — quanto está documentado

```sh
crux-analyzer coverage --src <dir> [--name <projeto>] [--min <porcentagem>] [--list] [--locale <locale>]
```

| Flag | Significado |
| --- | --- |
| `--min` | Sai com código diferente de zero quando a fração de estados descritos está abaixo desta porcentagem. |
| `--list` | Também nomeia os estados que não têm descrição. |

```
$ crux-analyzer coverage --src crates/parser/fixtures/mini_recorder --name "Mini Recorder" --locale pt-BR
MiniRecorder / RecorderState                 100%  6 de 6 estados descritos
MiniRecorder / UploadState                     0%  0 de 3 estados descritos
total                                         67%  6 de 9 estados descritos
```

**"Documentado" significa que o estado tem uma descrição.** Um estado que carrega
apenas um marcador ou uma etiqueta está classificado, não explicado, então não
conta — o objetivo da medida é prosa da qual um leitor consiga aprender algo. Uma
máquina cujo enum de estado não tem descrição própria recebe uma nota sob a sua
linha.

O `--min` compara **exatamente**, não contra a porcentagem exibida: 2 de 3 estados
aparece como 67% e *não* satisfaz `--min 67`. Uma máquina sem estados conta como
completa, então um projeto vazio nunca falha.

Esta é a catraca: coloque no CI com um `--min` no número de hoje, e a documentação
pode subir mas não descer. `just coverage <src> <nome> [min]` embrulha o comando.

### Os dois portões, ou nenhum é catraca

`docs --deny-warnings` e `coverage --min` falham por motivos diferentes, e um
projeto que liga só o primeiro não ganha catraca alguma: a primeira adoção desta
ferramenta rodava `docs --deny-warnings` no CI e estava em 79% de cobertura, com a
maior máquina em 0 de 7 estados descritos, sem nada falhar. Os avisos pegam o que o
parser não conseguiu ler; a cobertura pega o que ninguém escreveu. Ligue os dois:

```sh
crux-analyzer docs --src shared/src --name MyApp --deny-warnings --out docs/machines.md
crux-analyzer coverage --src shared/src --name MyApp --min 79 --list
```

Suba o `--min` no mesmo commit que documenta um estado — é este o mecanismo
inteiro, e ele só funciona se o número estiver no repositório em vez de na memória
de alguém.

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
