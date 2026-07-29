# Schema — o contrato do modelo

> 🌐 [English](../schema.md) · **Português (Brasil)**

O contrato vive em [`shared/schema/crux-model.schema.json`](../../shared/schema/crux-model.schema.json)
(JSON Schema draft 2020-12). Todo cliente — a UI web, os geradores de
documentação, qualquer coisa futura — depende apenas deste documento. Um exemplo
embutido é mantido em [`shared/schema/examples/audio-recorder.json`](../../shared/schema/examples/audio-recorder.json)
e um teste de ida e volta em `crates/model` mantém as structs Rust alinhadas com
ele.

## Forma

```json
{
  "project": "Audio Recorder",
  "cores": [
    {
      "name": "Recorder",
      "machines": [
        {
          "name": "RecorderState",
          "doc": "Where a recording session lives.",
          "states": [
            "Idle",
            "Recording",
            "Paused",
            "Uploading",
            {
              "name": "Failed",
              "doc": "The upload gave up. The session is kept so the user can retry.",
              "markers": ["failure"],
              "tags": ["retryable"]
            }
          ],
          "transitions": [
            {
              "from": "Idle",
              "event": "RecordPressed",
              "to": "Recording",
              "effects": ["AudioOperation::Start"]
            }
          ]
        }
      ],
      "events": [
        { "name": "RecordPressed", "doc": "The user hit the record button." }
      ],
      "effects": [
        { "name": "AudioOperation::Start", "doc": "Begins capturing audio." }
      ]
    }
  ]
}
```

## Semântica

| Campo | Significado |
| --- | --- |
| `project` | Nome do projeto analisado. |
| `cores[]` | Uma entrada por bloco `impl App` encontrado. |
| `machines[]` | **Regiões ortogonais** do statechart: uma por enum de estado dirigido pelo core. O nome é o nome do enum, desambiguado pelo campo quando o mesmo enum dirige duas máquinas (`State (left)`). |
| `machines[].doc` | Opcional. Documentação escrita no próprio enum de estado, sem as linhas de anotação. |
| `machines[].markers[]` | Opcional. Marcadores declarados no enum de estado — descrevem a região inteira. |
| `machines[].tags[]` | Opcional. Nomes de etiqueta livres declarados no enum de estado. |
| `states[]` | Estados folha, em ordem de declaração — **uma string simples ou um objeto** (veja abaixo). Filhos de **estados compostos** são caminhos `Pai/Filho` (`Active/Loading`). Um cliente que ignore a convenção ainda renderiza uma máquina plana válida. |
| `states[].name` | O nome do estado folha. A forma de string simples é exatamente este campo. |
| `states[].doc` | Opcional. Documentação escrita na variante do enum, sem as linhas de anotação. |
| `states[].markers[]` | Opcional. Marcadores declarados, na ordem em que aparecem: `"failure"`, `"deprecated"`. |
| `states[].tags[]` | Opcional. Nomes de etiqueta livres declarados com `@tag <nome>`, na ordem em que aparecem. |
| `transitions[].from` | Estado de origem, ou `"*"` — a transição dispara a partir de **qualquer** estado (estaticamente sem guarda). |
| `transitions[].event` | Nome da variante de evento folha que dispara a transição. |
| `transitions[].to` | Estado de destino, ou `"*"` — o destino é decidido em **tempo de execução** (por exemplo, carregado pelo payload do evento). |
| `transitions[].effects[]` | Opcional. Efeitos solicitados quando a transição dispara: `"Render"`, `"AudioOperation::Start"`, ... Omitido quando vazio. |
| `cores[].events[]` | Opcional. Pares `{ name, doc }`: documentação autoral nas variantes do enum de eventos, **apenas** para eventos que aparecem nas transições deste núcleo e **apenas** quando documentados — as tabelas de transição já enumeram o vocabulário. Omitido quando vazio, então uma aplicação sem documentação emite exatamente o JSON que emitia antes deste campo existir. |
| `cores[].effects[]` | Opcional. O mesmo para efeitos, indexados pelo rótulo que as transições carregam (`AudioOperation::Start`, `Render`). |

## Estados documentados

Um estado é escrito **ou** como um nome simples **ou** como um objeto que carrega
o que a fonte analisada documenta sobre ele. Um produtor emite a forma simples
sempre que não há documentação, então o modelo de uma aplicação sem anotações é
idêntico a um de antes da documentação:

```json
"states": ["Idle", { "name": "Failed", "markers": ["failure"] }]
```

As duas formas podem aparecer no mesmo array. Clientes devem normalizar na
entrada (`typeof state === 'string' ? { name: state } : state`) para que nada
adiante ramifique conforme a forma escrita.

`markers` é um **vocabulário fechado** — o vocabulário do próprio crux_analyzer,
e é por isso que os clientes renderizam um rótulo localizado para cada valor
enquanto o valor em si permanece um identificador estável. `initial` e `final`
deliberadamente **não** são marcadores: são derivados da forma do grafo (e de
`#[default]`), então declará-los permitiria que uma fonte contradissesse as
transições que ela também declara. Veja
[parser.md](parser.md#documentação-e-anotações) para como uma fonte escreve isso.

O schema fixa esse vocabulário, então um erro de digitação em um modelo escrito à
mão é um erro de validação — mas **clientes devem ignorar um marcador que não
conhecem** em vez de rejeitar o modelo, para que um parser mais novo nunca apague
uma UI mais antiga. Essa assimetria entre schema estrito e cliente tolerante é
deliberada.

## O contrato é independente de locale

Toda string do modelo é lida da aplicação analisada — identificadores e, desde
que a documentação chegou ao modelo, a prosa do próprio autor (`doc`) e os nomes
de etiqueta. Nada disso é traduzido: o crux_analyzer copia a prosa do autor
literalmente, exatamente como copia o nome de um estado, então a *língua* desse
texto é escolha do autor e não nossa. Nenhum texto **do próprio crux_analyzer**
jamais entra no modelo, então `crux-analyzer generate` continua produzindo JSON
byte a byte idêntico em todo locale, e os clientes localizam o próprio texto de
interface (a prosa que substitui `"*"`, cabeçalhos de tabela, rótulos de
marcador, títulos de painel). Adicionar um locale nunca deve adicionar um campo
aqui. Veja [i18n.md](i18n.md).

## Curingas

`"*"` é um nome de estado reservado em ambas as pontas de uma transição:

- `from: "*"` — dispara a partir de qualquer estado. UIs renderizam um pseudo-nó
  ("qualquer estado"); a simulação oferece essas transições a partir de todo
  estado.
- `to: "*"` — chega onde o valor de tempo de execução determinar. A simulação
  exclui essas do replay (não há nada estático em que aterrissar).

## Diretrizes de evolução

- Campos aditivos (como `effects`) são opcionais com padrões vazios, então
  clientes antigos continuam funcionando.
- Um valor que era uma string simples pode ser **ampliado** para "string ou
  objeto", tornando a forma de objeto opcional e emitindo a forma simples sempre
  que os dados extras estiverem vazios (foi assim que `states[]` ganhou
  documentação). Artefatos existentes continuam válidos e a saída sem anotações
  continua byte a byte idêntica, então a mudança é aditiva na prática — mas os
  clientes ainda se movem no mesmo commit, porque o produtor passa a emitir
  objetos no momento em que uma fonte é anotada.
- Mudanças que quebram a forma (como a introdução de `machines[]`) alteram todas
  as camadas no mesmo commit: schema, `crates/model` (+ teste de ida e volta), o
  exemplo embutido, `crates/docgen`, `apps/web/src/schema` + domínio + testes.
- A aplicação web trata um modelo gerado inválido como ausente (cai no exemplo
  embutido com um aviso no console), então artefatos desatualizados nunca a
  quebram.
