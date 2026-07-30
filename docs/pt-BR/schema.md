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
              "effects": [
                {
                  "name": "AudioOperation::Start",
                  "capability": "Audio",
                  "resolvesWith": ["RecordingStarted", "RecordingFailed"]
                }
              ]
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
| `states[].default` | Opcional (`false`). A fonte declara este estado como a variante `#[default]` do seu enum. Evidência de onde a máquina começa, e **não** um papel — os clientes derivam `initial` a partir dela e da forma das transições. |
| `transitions[].from` | Estado de origem, ou `"*"` — a transição dispara a partir de **qualquer** estado (estaticamente sem guarda). |
| `transitions[].event` | Nome da variante de evento folha que dispara a transição. |
| `transitions[].to` | Estado de destino, ou `"*"` — o destino é decidido em **tempo de execução** (por exemplo, carregado pelo payload do evento). |
| `transitions[].effects[]` | Opcional. Efeitos solicitados quando a transição dispara — **uma string simples ou um objeto** (veja abaixo). Omitido quando vazio. |
| `effects[].name` | A operação como as transições a rotulam: `"AudioOperation::Start"`, ou `"Render"` para o builtin do crux. A forma de string simples é exatamente este campo. |
| `effects[].capability` | Opcional. A variante do enum `Effect` raiz do núcleo que envolve esta operação (`Effect::Audio(AudioOperation)` → `"Audio"`). Ausente quando a solicitação não passa por nenhuma, ou quando não pôde ser resolvida. |
| `effects[].resolvesWith[]` | Opcional. Eventos com que o shell pode responder a esta solicitação, conforme declarado no local da solicitação — vários quando o callback mapeia um evento por desfecho. Ausente para solicitações do tipo disparar e esquecer. Um evento aqui não precisa aparecer em nenhuma transição: uma confirmação que o núcleo apenas renderiza é comportamento real. |
| `effects[].conditional` | Opcional (`false`). A solicitação está em um ramo que a própria transição não implica: chegar ali *pode* solicitá-la. |
| `cores[].events[]` | Opcional. Pares `{ name, doc }`: documentação autoral nas variantes do enum de eventos, **apenas** para eventos que aparecem nas transições deste núcleo e **apenas** quando documentados — as tabelas de transição já enumeram o vocabulário. Omitido quando vazio, então uma aplicação sem documentação emite exatamente o JSON que emitia antes deste campo existir. |
| `cores[].effects[]` | Opcional. O mesmo para efeitos, indexados pelo rótulo que as transições carregam (`AudioOperation::Start`, `Render`). |

## Efeitos, e o laço que eles fecham

Uma entrada de `effects[]` de uma transição é escrita **ou** como o rótulo simples
da operação **ou** como um objeto que acrescenta o que a fonte analisada declara
em torno da solicitação. O mesmo alargamento de `states[]`, pelo mesmo motivo: um
app cujas solicitações não mostram capacidade nem callback emite exatamente o JSON
que emitia antes desses campos existirem.

```json
"effects": ["Render", { "name": "HttpOperation::Upload", "capability": "Http", "resolvesWith": ["UploadFinished"] }]
```

`resolvesWith` é a volta do laço `Evento → Efeito → shell → Evento` do Crux, e a
razão de estar no contrato: um grafo de estados mostra os eventos que entram, e
sem isso nada diz quais deles o *shell* devolve. É um conjunto porque uma
solicitação tem uma resposta por desfecho, e é só o que a fonte nomeia no local da
solicitação — nunca inferido do nome de uma operação. Veja
[parser.md](parser.md#efeitos) para o que conta como evidência.

`conditional` é a regra de honestidade aplicada à atribuição. Um efeito solicitado
em um ramo abaixo da atribuição não é descartado nem afirmado sem ressalva: ele
viaja com a transição e diz que chegar ali *pode* solicitá-lo.

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
deliberadamente **não** são marcadores: são *derivados*, então declará-los
permitiria que uma fonte contradissesse as transições que ela também declara. Veja
[parser.md](parser.md#documentação-e-anotações) para como uma fonte escreve isso.

## Onde uma máquina começa

`default` é a única chave de um objeto de estado que não é documentação: ela diz
que a fonte escreveu `#[default]` naquela variante. É também a única chave que faz
um estado sem mais nada assumir a forma de objeto, então uma aplicação cujos enums
de estado derivam `Default` emite um objeto de estado por máquina onde um modelo
anterior ao `default` escrevia um nome simples.

```json
"states": [{ "name": "Idle", "default": true }, "Recording"]
```

O modelo para aí, no que a fonte declara. Transformar isso no papel `initial` é
trabalho do cliente, e todo cliente deve ler da mesma forma:

1. o estado cujo `default` é verdadeiro;
2. senão, todo estado onde nenhuma transição chega;
3. senão — uma máquina totalmente **cíclica**, onde nenhum dos dois tipos de
   evidência existe — o primeiro estado de `states[]`.

A ordem de declaração vem por último de propósito: num ciclo ela não significa
nada, e é exatamente por isso que `default` está no contrato. As duas
implementações são `crates/docgen/src/roles.rs` e
`apps/web/src/domain/stateRole.ts`; `final` não precisa de evidência própria,
sendo um estado do qual nenhuma transição sai (uma origem `"*"` de máquina inteira
não conta — essa fuga pertence ao pseudo-nó curinga).

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
