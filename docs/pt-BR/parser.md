# Parser

> 🌐 [English](../parser.md) · **Português (Brasil)**

`crates/parser` reconstrói estaticamente as máquinas de estado de uma aplicação
Crux a partir das suas fontes Rust. Ele nunca executa código e nunca depende do
Crux — tudo é derivado da AST do `syn`.

## Pipeline

1. **Carregar** — todo arquivo `.rs` sob `--src` é parseado e achatado (sem
   resolução da árvore de módulos). Módulos `#[cfg(test)]` são ignorados, então
   utilitários de teste nunca contribuem com estados ou transições.
2. **Indexar** — enums (todas as declarações por nome — nomes podem colidir entre
   módulos — mais aliases `use ... as ...`), structs (nome/tipo dos campos),
   funções (por `(tipo do self, nome)`, com nomes de parâmetros). Tipos de campos
   de variantes atravessam `Box`/`Rc`/`Arc`.
3. **Detectar máquinas** — veja abaixo.
4. **Encontrar Cores** — todo bloco `impl App for X`. O tipo associado `Event`
   semeia o fechamento de enums de evento (enums de evento aninhados como
   `Event::Recording(RecordingEvent)` são seguidos); o tipo associado `Effect`
   semeia o fechamento de efeitos do mesmo jeito.
5. **Extrair transições** — percorre `update` e todo auxiliar que ela chama
   (entre arquivos, à prova de ciclos), carregando contexto; emite uma transição
   em cada atribuição de estado.
6. **Emitir** — agrupa transições por `(enum, campo)` em uma máquina por região,
   deduplica e anexa efeitos.

## Detecção de máquinas

Uma máquina de estado é um par `(enum, campo)` com **evidência de atribuição**:

- direta: `*.campo = Enum::Variante` (qualquer forma de construção), ou
- via reset: `*.x = T::default()` onde a struct `T` tem um campo `campo: Enum`.

Nenhuma convenção de nomenclatura é exigida. A atribuição é o sinal
discriminante: enums espelho de ViewModel são apenas *construídos* dentro de
structs de visão, nunca atribuídos a um campo do modelo, então nunca se tornam
máquinas.

O mesmo enum pode dirigir várias máquinas através de campos diferentes (duas
sessões do mesmo tipo); transições são atribuídas por `(enum, campo)` e os nomes
das máquinas desambiguam: `State (left)`, `State (right)`.

### Estados compostos

Uma variante com exatamente um campo sem nome cujo tipo é outro enum do crate
(`State::Active(ActiveState)`) se torna um **estado composto** — folhas
`Active/Loading`, `Active/Ready`, ... — mas somente com **evidência de
subestado**: um padrão de variante aninhada
(`State::Active(ActiveState::Loading)`) em algum lugar do crate. Sem essa
evidência o campo é dado de payload (`State::Failed(ErrorCode)`) e a variante
permanece uma folha simples, então `model.state = State::Failed(reason)` resolve
para `Failed` como qualquer outro destino.

A resolução de padrões é profunda: `Active(Phase::Ready)` → a folha exata;
`Active(_)` → todas as folhas filhas.

## Eventos

Os rótulos de evento são os nomes das variantes **folha**. Variantes envelope que
apenas carregam um enum de evento aninhado (`Event::Recording(RecordingEvent)`)
delegam: o `match` interno resolve o rótulo. Um enum só se qualifica como enum de
evento aninhado quando o código de fato **despacha sobre ele** (suas variantes
aparecem em padrões) — enums de payload carregados por um evento
(`Event::Boom(ErrorCode)`) continuam sendo dados, e enums de estado são excluídos
explicitamente.

Braços com múltiplos eventos se desdobram: `event @ (A | B) => ...` produz uma
transição por evento.

## Estados de origem (`from`)

Resolvidos por máquina em cada atribuição, a partir de três tipos de evidência:

| Evidência | Exemplo | Resultado |
| --- | --- | --- |
| guarda `matches!` | `if matches!(state, A \| B)` | `{A, B}` |
| negação | `if !matches!(state, Idle)` | complemento |
| comparação `==` / `!=` | `state == State::Idle` (também dentro de closures `find(\|d\| ...)`) | `{Idle}` / complemento |
| `match` sobre o estado | padrões dos braços; `_` resolve para o complemento dos braços anteriores | conjuntos por braço |
| método predicado | `state.has_capture_in_flight()` — o corpo do método (no impl do enum de estado) é analisado, incluindo negação e predicado chamando predicado (com limite de profundidade) | o conjunto do predicado |
| estreitamento com let-else | `let Some(d) = list.find(\|d\| d.state == X) else { return }` | vale para o resto do bloco |
| **nenhuma evidência** | atribuição sem guarda | curinga `"*"` — a transição legitimamente dispara a partir de qualquer estado |

Condições compõem por `&&` (interseção), `||` (união dos lados concretos) e `!`
(complemento). Uma restrição concreta vence um conjunto irresolvível — o conjunto
emitido pode então ser um superconjunto da verdade, que é o viés correto para
documentação.

## Destinos (`to`) e fluxo de valores

- Construções literais resolvem diretamente (filhos de compostos incluídos).
- `*.x = T::default()` implica que todo campo de estado de `T` cai na variante
  `#[default]` do seu enum.
- **Payload do evento**: `draft.status = status`, onde `status` é um binding do
  padrão do evento tipado como o enum de estado → destino `"*"` (o estado de
  chegada é escolha da shell).
- **Valores restritos**: `draft.st = known.st.clone()` guardado por
  `is_this_runs_answer(&known.st)` — o predicado (função livre ou método) é
  resolvido contra seu parâmetro e o destino se desdobra nas variantes que ele
  permite. Restrições de valor são indexadas pelo **caminho exato da expressão**
  (`known.st` nunca escapa para `draft.st`), e apenas chamadas que preservam
  identidade (`clone`, `to_owned`, `as_ref`, ...) são atravessadas — `.take()` ou
  acessores nunca criam alias.

## Efeitos

As operações solicitadas por cada braço de evento se anexam às transições que ele
produz:

- construções de enums do fechamento de efeitos (`AudioOperation::Start`),
  rotuladas `Enum::Variante`;
- uma chamada ao `render()` puro do crux → `Render`.

Um braço que produz várias transições compartilha seu conjunto de efeitos entre
elas (uma sobreaproximação para braços com ramificação interna).

## Referência de avisos

A regra da honestidade: o que não pode ser inferido estaticamente é exposto,
nunca descartado em silêncio, nunca adivinhado. Todos os avisos carregam
`arquivo:linha`.

Um aviso é **dado**, não string: `Warning { file, line, kind }`, onde `kind` é um
`WarningKind`. `kind.code()` é o identificador estável e independente de locale —
baseie ferramentas e documentação nele, já que o texto da mensagem é localizado
([i18n.md](i18n.md)). As mensagens em português aparecem abaixo.

| Código | Mensagem (`pt-BR`) | Significado |
| --- | --- | --- |
| `unknown-event` | `não foi possível inferir o evento que a dispara` | uma atribuição de estado foi alcançada sem rótulo de evento em escopo (por exemplo, sob um braço catch-all com contexto desconhecido) |
| `unresolvable-source` | `a condição do estado de origem não pôde ser resolvida estaticamente` | a guarda referencia o estado mas derrota a análise (por exemplo, um predicado irresolvível) |
| `dynamic-target` | `o estado de destino é dinâmico (atribuído a partir de um valor definido em tempo de execução)` | o valor atribuído não tem tipagem de payload nem restrições resolvíveis |
| `no-update-method` | `núcleo X: método update não encontrado` | um bloco `impl App` sem função `update` |

Uma execução limpa do corpus (o teste Quipu) extrai com **zero** avisos.

## Limites conhecidos

- Bindings não fluem por parâmetros de chamadas auxiliares (um binding de payload
  passado a um auxiliar com outro nome perde sua tipagem; restrições dentro do
  auxiliar continuam valendo).
- A avaliação de guardas casa campos de estado pelo último nome de campo dentro
  de um escopo; dois campos de mesmo nome do mesmo enum em um escopo podem se
  estreitar cruzado.
- Filhos de compostos envolvidos em genéricos diferentes de `Box`/`Rc`/`Arc` não
  são seguidos.
- Colisões de nome entre módulos são resolvidas por preferência ao mesmo arquivo e
  por dicas de alias (`use path::Event as RecordingEvent`), não por resolução
  completa da árvore de módulos.
