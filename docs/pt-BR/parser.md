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

- direta: `*.campo = Enum::Variante` (qualquer forma de construção),
- via reset: `*.x = T::default()` onde a struct `T` tem um campo `campo: Enum`, ou
- via **value flow em um campo alcançável pelo model**: `*.campo = <valor de
  runtime>`, onde o tipo associado `Model` alcança um campo com aquele nome tipado
  como um enum que o crate despacha em padrões.

Nenhuma convenção de nomenclatura é exigida. A atribuição é o sinal
discriminante: enums espelho de ViewModel são apenas *construídos* dentro de
structs de visão, nunca atribuídos a um campo do modelo, então nunca se tornam
máquinas.

A terceira forma existe porque qual lado é dono de uma transição decide como o core
a escreve. Um status que a *shell* dirige só é armazenado pelo core — de um payload
de evento, ou clonado de outro registro — então ele nunca nomeia seu enum na
atribuição, e as duas primeiras formas o perdem por completo. A alcançabilidade pelo
model é o que fornece o tipo que falta: a travessia começa no tipo associado `Model`
e segue campos de struct através dos containers que os guardam (`Vec<Entry>` →
`Entry`, mapas pelo tipo de valor), então um status guardado uma vez por entidade
ainda conta. Um enum espelho não é alcançável, então exigir alcançabilidade mantém a
exclusão original intacta sem heurística de nome.

Dois limites que vale nomear. Só campos de struct são seguidos, então uma struct
atrás de uma variante de enum não é alcançada. E a alcançabilidade é um caminho
*adicional*, não um filtro sobre as duas primeiras: um enum atribuído literalmente
continua virando máquina, o model o guardando ou não.

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

**A evidência de origem é chaveada por nome de campo, não por objeto.** Um guard em
`other.status` portanto restringe uma máquina em `campo: status` mesmo quando
`other` é um registro *diferente* do mesmo tipo — o espelho de valor usado para
destinos é chaveado por caminho exato, este não. Onde um guard desses é conjugado
com outro sobre o registro que está sendo escrito (`this.status == Pending &&
matches!(other.status, Done | Deferred)`), os dois conjuntos se intersectam em nada.
Essa contradição não pode descrever um branch real, então em vez de derrubar a
transição em silêncio ela é reportada como `unresolvable-source`. Resolver o
aliasing está planejado — [roadmap.md §6](roadmap.md).

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

Um efeito é uma **solicitação com volta**, e é assim que ele é lido: a operação,
a capacidade por onde ela trafega e os eventos com que o shell pode responder.

O que conta como solicitação:

- construções de enums do fechamento de efeitos (`AudioOperation::Start`),
  rotuladas `Enum::Variante`;
- uma chamada ao `render()` puro do crux → `Render`.

### A capacidade

O enum `Effect` do núcleo é o que nomeia suas capacidades: a variante que envolve
o enum de uma operação *é* a capacidade por onde a operação passa.
`Effect::Audio(AudioOperation)` coloca toda solicitação de `AudioOperation` sob
`Audio`. Estrutura, não inferência — nada é lido do formato de um nome. `Render`
não passa por nenhuma.

### A resposta

O laço do Crux é `Evento → Efeito → shell → Evento`, e a volta está escrita no
local da solicitação, então ela é evidência como qualquer outra. Três formatos
são lidos, e os eventos que eles nomeiam se tornam o `resolvesWith` da
solicitação:

```rust
// o callback ao lado da operação
Self::request_audio(AudioOperation::Start, Event::CaptureStarted);

// a API Command do crux
Command::request_from_shell(operation).then_send(Event::Started);

// um callback que mapeia o resultado do shell: todo evento que ele constrói
Command::request_from_shell(operation).then_send(move |result| match result {
    AudioResult::Started { id } => RecordingEvent::RecordingStarted { id },
    AudioResult::Failed(message) => RecordingEvent::RecordingFailed { message },
})
```

Um **conjunto**, não um evento: uma solicitação tem rotineiramente uma resposta
por desfecho. Uma solicitação construída por um helper compartilhado
(`audio_command(op)`, cujo corpo faz o `then_send`) é seguida uma chamada adiante,
então a operação que o chamador escreveu mantém as respostas que seu helper
declara — o que também significa que toda solicitação por aquele helper carrega a
união do que o callback dele pode construir. Essa é a leitura honesta: pelo que a
fonte mostra, qualquer uma delas pode voltar.

Uma resposta que nomeia um evento que **nenhuma transição carrega** é mantida. É
comportamento real — uma confirmação que o núcleo apenas renderiza — e os
clientes dizem isso em vez de esconder.

Um callback cujo evento não pode ser lido no local da chamada (`then_send(f)`) é
um aviso `unresolved-effect-callback`: a solicitação continua registrada, apenas
sua resposta é desconhecida. Uma solicitação que não declara callback nenhum
*não* é aviso — disparar e esquecer é um formato legítimo.

### A quais transições uma solicitação pertence

Efeitos têm escopo de **ramo**, não de braço: a cadeia de alternativas (ramos de
`if`/`else`, braços de `match`) percorrida até a solicitação é comparada com a
cadeia da atribuição que produziu a transição.

- Mesma cadeia → a solicitação pertence àquela transição.
- Cadeias que se bifurcam → não pertence. Uma solicitação em um ramo nunca cai
  nas transições do ramo irmão.
- Mais profunda que a transição → pertence, marcada como **condicional**: chegar
  ali *pode* solicitá-la. Sobreaproximar é o certo (a solicitação é real), e
  dizê-lo é melhor do que soar como certeza.

```rust
RecorderEvent::RetryPressed => {
    if session.attempts_left() {
        state = Uploading;                     // ← Upload, certo
        Self::request(HttpOperation::Upload).then_send(RecorderEvent::UploadFinished);
    } else {
        state = Idle;                          // ← apenas Render
        render();
    }
}
```

## Documentação e anotações

Comentários de documentação em um enum de estado chegam ao modelo: o `///` do
próprio enum se torna a descrição da máquina, e o de cada variante se torna a do
seu estado.

Comentários de documentação em **variantes de enums de evento e efeito**
também chegam ao modelo, como catálogos `events` / `effects` por núcleo
(`{ name, doc }`). Duas restrições mantêm os catálogos honestos: só entram
nomes que aparecem no modelo do núcleo (um wrapper delegante documentado
como `Event::Recorder(RecorderEvent)` não é um evento que uma transição possa
carregar) — um evento nomeado como resposta de um efeito conta, já que está no
modelo e um cliente que o exibe deve poder exibir a prosa dele —, e só entram
nomes documentados, porque as tabelas de transição já enumeram o vocabulário. Anotações (linhas `@…`) não são lidas em eventos ou
efeitos; ainda não há nada que um marcador signifique ali.

```rust
/// Where a recording session lives.
pub enum RecorderState {
    /// Nothing is being recorded yet.
    Idle,

    /// The upload failed. The session is kept so the user can retry.
    ///
    /// @failure
    /// @tag retryable
    Failed { reason: String },
}
```

`///`, `/** … */` e um `#[doc = "…"]` escrito à mão todos funcionam; a
indentação comum é removida como o rustdoc a remove, e a quebra de linha do
autor nunca é refeita. `#[doc(hidden)]` é ignorado — ele não esconde um estado.

**Anotações** são linhas `@` escritas dentro do comentário de documentação. Esse
é o único mecanismo que não exige dependência no crate analisado: o
crux_analyzer nunca deve ser dependência das aplicações que ele lê, então um
atributo real está fora de questão e um atributo desconhecido não compilaria.

| Anotação | Significado |
| --- | --- |
| `@failure` | O estado representa uma falha que a aplicação reconhece como tal. |
| `@deprecated` | O estado está a caminho de sair. |
| `@tag <nome>` | Um rótulo livre (`retryable`, `offline`). Vários nomes podem dividir uma linha, separados por espaços ou vírgulas. |

Marcadores são um **vocabulário fechado**; `@tag` é a saída de emergência
aberta. Deliberadamente não existe `@initial` nem `@final`: esses são derivados
da forma do grafo e de `#[default]`, então declará-los permitiria que uma fonte
contradissesse as transições que ela também declara.

Linhas reconhecidas são removidas da descrição, e sequências de linhas em branco
são então colapsadas — assim uma anotação escrita entre dois parágrafos produz
exatamente a mesma prosa que uma escrita no fim.

### O que é anotação e o que é prosa

A regra é uma frase: **uma linha só é anotação quando está completa e bem
formada; qualquer outra coisa é prosa.** Palavras-chave casam sem diferenciar
maiúsculas, então um deslize de capitalização ainda funciona.

| Linha | Lida como |
| --- | --- |
| `@failure`, `@FAILURE` | o marcador |
| `@tag retryable, offline` | duas etiquetas |
| ``Apple constrains it — `@Generable` leaves no room`` | prosa — `@` não é o primeiro caractere |
| `Ask support@example.com` | prosa |
| `@deprecated` dentro de uma cerca ` ``` ` | prosa — blocos cercados são exemplos |
| `\@failure is how you mark one` | prosa, sem a barra invertida — a saída de emergência |
| `@failur`, `@see`, `@tag`, `@failure porque …` | **um aviso** (veja abaixo) |

Uma linha com forma de anotação que não é reconhecida é reportada em vez de
ficar na prosa, porque um `@failur` silenciosamente inerte é exatamente a
resposta errada e quieta que a regra de honestidade existe para evitar. Só enums
que de fato se tornaram máquinas são inspecionados, então um comentário de
documentação em um enum não relacionado nunca gera aviso.

### Estados compostos

Um pai composto não tem nó próprio no modelo — apenas suas folhas
`Pai/Filho`. Então cada folha **herda** a documentação da variante pai:
marcadores e etiquetas se unem (o pai primeiro), e a prosa do pai é colocada
acima da do filho em vez de ser substituída por ela. Nada do que o autor
escreveu é descartado.

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
| `unresolvable-source` | `a condição do estado de origem não pôde ser resolvida estaticamente` | a guarda referencia o estado mas derrota a análise: um predicado irresolvível, ou guardas que se intersectam em nada porque campos homônimos de dois objetos foram lidos como um (veja acima) |
| `dynamic-target` | `o estado de destino é dinâmico (atribuído a partir de um valor definido em tempo de execução)` | o valor atribuído não tem tipagem de payload nem restrições resolvíveis |
| `no-update-method` | `núcleo X: método update não encontrado` | um bloco `impl App` sem função `update` |
| `unknown-annotation` | `anotação X não reconhecida: não é @failure, @deprecated nem @tag <nome>` | uma linha de documentação parecia uma anotação mas não é: um erro de digitação, um marcador com argumento, ou um `@tag` sem nome utilizável |
| `unresolved-effect-callback` | `callback de efeito não resolvido: o evento que responde a esta solicitação não é nomeado no local da chamada` | um `then_send` cujo argumento não constrói evento algum (uma função, ou um valor calculado em outro lugar). A solicitação é registrada; a resposta dela, não |

O fixture `mini_recorder` extrai com **zero** avisos, o que o `just check` garante
com `--deny-warnings`. Uma aplicação alvo real hoje reporta o aliasing por nome de
campo descrito em [Estados de origem](#estados-de-origem-from); isso é uma lacuna
conhecida com correção planejada, não uma propriedade da aplicação.

### Avisos de recursos

A mesma regra aplicada a recursos: o analisador é apontado para código que não
controla, então tem limites — e um limite acionado é reportado em vez de truncar o
modelo em silêncio. O `--deny-warnings` portanto transforma uma análise truncada
em uma execução falha. Os limites e suas flags estão em
[security.md](security.md#3-toda-dimensão-ilimitada-de-entrada-recebe-um-limite-e-todo-limite-acionado-é-reportado)
e em [cli.md](cli.md#limites-de-recursos).

| Código | Mensagem (`pt-BR`) | Significado |
| --- | --- | --- |
| `analysis-truncated` | `núcleo X: a análise parou no limite de Y — o modelo pode estar incompleto` | o orçamento de passos, profundidade ou profundidade de chamadas acabou; transições podem estar faltando |
| `file-too-large` | `arquivo ignorado: N bytes excede o limite de M bytes` | acima de `--max-file-size`, não lido |
| `input-too-large` | `arquivos restantes ignorados: a execução alcançou o limite total de M bytes de código-fonte` | acima de `--max-total-size`; a caminhada parou |
| `nesting-too-deep` | `arquivo ignorado: os delimitadores aninham mais de N níveis` | o `syn` recursa sobre aninhamento e um stack overflow abortaria o processo, então isto é verificado no texto bruto antes do parsing |
| `not-a-regular-file` | `caminho ignorado: não é um arquivo regular (link simbólico, dispositivo ou FIFO)` | um `.rs` com link simbólico leria fora da árvore; um FIFO travaria |
| `source-unreadable` | `caminho ignorado: <motivo>` | um erro de caminhada ou de metadados — antes silencioso, o que fazia um problema de permissão parecer um modelo completo |

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
- Documentar um enum de estado não o faz aparecer: uma máquina ainda precisa de
  pelo menos uma transição extraída para chegar ao modelo.
- Quando duas declarações compartilham um nome, a documentação vem da que ganha a
  colisão (a com mais variantes), como todo o resto sobre ela.
- `#[cfg_attr(…, doc = "…")]` não é seguido, e links intra-doc do rustdoc
  (`` [`Self::Unavailable`] ``) viajam literalmente — só resolvem no rustdoc.
- Uma anotação escrita errado é reportada, não corrigida: não há casamento por
  aproximação, porque adivinhar quase-acertos trocaria uma resposta errada e
  quieta por outra.
