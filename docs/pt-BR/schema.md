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
          "states": ["Idle", "Recording", "Paused", "Uploading", "Completed"],
          "transitions": [
            {
              "from": "Idle",
              "event": "RecordPressed",
              "to": "Recording",
              "effects": ["AudioOperation::Start"]
            }
          ]
        }
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
| `states[]` | Nomes dos estados folha, em ordem de declaração. Filhos de **estados compostos** são caminhos `Pai/Filho` (`Active/Loading`). Um cliente que ignore a convenção ainda renderiza uma máquina plana válida. |
| `transitions[].from` | Estado de origem, ou `"*"` — a transição dispara a partir de **qualquer** estado (estaticamente sem guarda). |
| `transitions[].event` | Nome da variante de evento folha que dispara a transição. |
| `transitions[].to` | Estado de destino, ou `"*"` — o destino é decidido em **tempo de execução** (por exemplo, carregado pelo payload do evento). |
| `transitions[].effects[]` | Opcional. Efeitos solicitados quando a transição dispara: `"Render"`, `"AudioOperation::Start"`, ... Omitido quando vazio. |

## O contrato é independente de locale

Toda string do modelo é um identificador lido da aplicação analisada — nenhum
texto traduzido jamais entra nele. Portanto `crux-analyzer generate` produz JSON
byte a byte idêntico em todo locale, e os clientes localizam o próprio texto de
interface (a prosa que substitui `"*"`, cabeçalhos de tabela, títulos de painel).
Adicionar um locale nunca deve adicionar um campo aqui. Veja [i18n.md](i18n.md).

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
- Mudanças que quebram a forma (como a introdução de `machines[]`) alteram todas
  as camadas no mesmo commit: schema, `crates/model` (+ teste de ida e volta), o
  exemplo embutido, `crates/docgen`, `apps/web/src/schema` + domínio + testes.
- A aplicação web trata um modelo gerado inválido como ausente (cai no exemplo
  embutido com um aviso no console), então artefatos desatualizados nunca a
  quebram.
