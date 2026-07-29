Acho que eu orientaria o MVP para ser um **analisador semântico**, e não um gerador de diagramas. O React vira apenas um cliente do modelo gerado. Isso deixa o caminho muito mais fácil para adicionar um CLI, uma extensão do VS Code e até uma interface TUI no futuro.

Segue um prompt que eu usaria com um agente de código (Codex, Claude Code ou Cursor):

---

# Crux Studio MVP

Quero construir um projeto chamado **Crux Studio**.

O objetivo é transformar aplicações escritas com **Rust + Crux** em uma documentação viva.

O projeto NÃO deve depender de Crux internamente. Ele apenas analisa código Rust utilizando a AST (`syn`) e constrói um modelo semântico da aplicação.

O MVP terá apenas uma interface Web em React, mas toda a arquitetura deve ser preparada para que, no futuro, exista:

* CLI (`crux-studio generate`)
* extensão VS Code
* geração de Markdown
* Mermaid
* PlantUML
* documentação HTML

Portanto, a UI nunca deve acessar diretamente a AST.

Ela deve consumir apenas um modelo intermediário.

---

# Arquitetura

Utilizar um monorepo.

```text
crux-studio/

apps/
    web/

crates/
    parser/
    model/

shared/
    schema/
```

Onde:

## parser

Biblioteca Rust.

Responsável por:

* ler arquivos Rust
* utilizar syn
* navegar pela AST
* identificar Core, State, Event, Effect e transições
* produzir um modelo intermediário

Nunca deve conhecer React.

---

## model

Biblioteca Rust.

Contém apenas as estruturas semânticas.

Exemplo:

```rust
Project

Core

State

Event

Effect

Transition

Capability
```

Nenhuma lógica de parsing.

Nenhuma lógica de UI.

---

## schema

Contém o contrato serializado.

Preferencialmente JSON Schema.

A UI depende apenas deste contrato.

---

## web

Aplicação React.

Responsável apenas por visualizar o modelo.

Nenhum parsing acontece na UI.

---

# MVP

Inicialmente NÃO analisar código Rust.

Criar apenas um JSON fake.

Exemplo:

```json
{
  "project":"Audio Recorder",

  "cores":[
    {
      "name":"Recorder",

      "states":[
        "Idle",
        "Recording",
        "Paused",
        "Uploading",
        "Completed"
      ],

      "transitions":[
        {
          "from":"Idle",
          "event":"RecordPressed",
          "to":"Recording"
        },

        {
          "from":"Recording",
          "event":"PausePressed",
          "to":"Paused"
        },

        {
          "from":"Paused",
          "event":"ResumePressed",
          "to":"Recording"
        },

        {
          "from":"Recording",
          "event":"StopPressed",
          "to":"Uploading"
        },

        {
          "from":"Uploading",
          "event":"UploadFinished",
          "to":"Completed"
        }
      ]
    }
  ]
}
```

Toda a UI deve funcionar apenas lendo esse JSON.

---

# Interface

Utilizar:

* React
* TypeScript
* React Flow
* ELKJS

Layout semelhante ao LangGraph Studio ou Trigger.dev.

A tela possui três áreas.

## Sidebar

Lista de Cores.

```
Recorder

Authentication

Sync
```

---

## Área principal

Diagrama React Flow.

Layout automático utilizando ELK.

Cada estado é um nó.

Cada transição é uma aresta.

O label da aresta é o evento.

---

## Painel direito

Ao selecionar um estado:

```
Recording

Incoming

PausePressed

ResumePressed

Outgoing

StopPressed
```

Ao selecionar uma transição:

```
RecordPressed

Idle

↓

Recording
```

---

# Organização da UI

A UI nunca deve conhecer o formato original do parser.

Criar uma camada:

```
Parser JSON

↓

Domain Model

↓

React Flow Model

↓

Componentes
```

Isso permitirá trocar o parser futuramente sem alterar a interface.

---

# Componentização

Criar componentes independentes.

```
Graph

Sidebar

Inspector

Toolbar

LayoutEngine
```

A troca do ELK por outro algoritmo deve exigir alterações apenas no LayoutEngine.

---

# Futuro

A arquitetura deve facilitar adicionar:

## CLI

```
crux-studio generate

crux-studio graph

crux-studio docs
```

A CLI apenas reutilizará parser + model.

---

## VSCode

A extensão apenas chamará parser + model.

A Web poderá ser reaproveitada em um WebView.

---

## Documentação

Geradores independentes.

```
MarkdownGenerator

MermaidGenerator

PlantUMLGenerator

HtmlGenerator
```

Todos consomem apenas o modelo.

---

## Simulação

No futuro o React deverá conseguir reproduzir eventos.

```
Idle

↓

RecordPressed

↓

Recording

↓

PausePressed

↓

Paused
```

Portanto a arquitetura da UI deve permitir adicionar um "Simulation Engine" sem modificar o grafo.

---

# Objetivos

Priorizar:

* arquitetura limpa
* separação entre parser e visualização
* baixo acoplamento
* componentes pequenos
* código preparado para evolução

Não gastar tempo com identidade visual.

O objetivo do MVP é validar a arquitetura e a experiência de navegação, não produzir uma ferramenta visual final.
