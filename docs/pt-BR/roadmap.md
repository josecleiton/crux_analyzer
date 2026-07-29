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

## 1. A catraca — dar dentes à documentação

O trabalho de maior alavancagem, e o mais barato. Hoje não existe CI nenhum no
repositório.

### 1.1 CI rodando `just check`

O `just check` já faz a coisa certa (corpus + clippy + testes web + build web).
Ele só nunca roda a menos que uma pessoa digite. Um workflow fecha a maior
lacuna do projeto.

Precisa de: toolchain Rust + pnpm + `just`, e uma decisão sobre o corpus — o
teste do Corpus depende de `CORPUS_SRC` e essa fonte não é pública, então o CI roda
os testes de fixture e pula o corpus (o gate já trata disso por construção).

### 1.2 `--deny-warnings`

O `crux-analyzer` já conta os avisos e os imprime em stderr; nada age sobre eles.
Uma flag `--deny-warnings` que sai com código diferente de zero quando a contagem
é maior que zero transforma "o corpus extrai limpo" de uma observação no
`parser.md` em algo que um pipeline garante.

Pequeno, autocontido, e o companheiro natural do 1.1.

### 1.3 `crux-analyzer coverage`

O trabalho de documentação de estados tornou isso mensurável pela primeira vez:
`doc` está no modelo, então o modelo pode ser perguntado *quanto dele está
documentado*.

Um subcomando `coverage` que reporta, por core e por máquina, a fração de estados
que carregam descrição — e falha abaixo de um limite `--min`. É isso que
transforma a ferramenta de visualizador em catraca: um time adota, o número sobe,
e o CI impede que desça.

É também a contraparte honesta da feature que acabou de sair. Documentação que se
pode adicionar é boa; documentação que se pode *medir* é a que de fato é escrita.

---

## 2. Fechar o loop das etiquetas

`@tag` existe no modelo e renderiza como chips, mas é **inerte**: dá para
declarar uma etiqueta e olhar para ela, não para *usá-la*. Com oito estados isso
não incomoda; com trinta é a diferença entre um diagrama e uma ferramenta.

- **Filtro e busca por etiqueta na UI web.** Digite `retryable`, mantenha os
  estados que a carregam, esmaeça o resto. O grafo já esmaece nós durante a
  simulação, então o vocabulário visual existe.
- **Destacar estados não documentados.** A contraparte visual do §1.3 — os
  estados em que um leitor ainda não deveria confiar. Deliberadamente opt-in,
  para que a visão padrão continue sendo sobre a máquina e não sobre nossas
  métricas.

---

## 3. Alcance — a extensão do VS Code

O maior público e a maior construção: a máquina de estados ao lado do código, sem
sair do editor. Toda camada de que ela precisa já existe — é outro cliente do
mesmo contrato JSON, que é exatamente o formato que a arquitetura foi desenhada
para permitir.

O `just site` já cobre o caminho "compartilhar com o time" (um build estático com
o modelo embutido), então a extensão é sobre o loop de *escrita*, não sobre o de
leitura. É por isso que ela fica depois da catraca: amplia o alcance, não protege
a qualidade.

---

## 4. Lacunas menores que valem correção

Observadas durante a construção, mais ou menos em ordem de visibilidade:

- **Estados compostos renderizam planos no grafo web** (nós `Pai / Filho`)
  enquanto o Mermaid já os aninha. A inconsistência mais visível do produto hoje.
  Toca apenas `flow/` e `layout/` — o React Flow suporta nós-pai, que as seções
  de máquina já usam.
- **Nenhum estado de seleção na URL.** Não existe link para "este estado desta
  máquina". Para documentação feita para ser compartilhada e referenciada em uma
  revisão, isso custa mais do que parece.
- **Comentários de documentação em eventos e efeitos.** Estados e máquinas estão
  cobertos; um evento com um `///` explicando *quando* ele dispara é o pedido
  natural seguinte. Precisa de um tipo mais rico para `Transition.event`, então é
  mudança de contrato e não aditiva.
- **Efeitos só aparecem por transição**, nunca agregados por estado. "O que
  entrar em `Uploading` de fato faz" exige a união das transições de entrada.
- **A simulação não consegue reproduzir destinos curinga** (`to: "*"`), já que
  não há nada estático em que aterrissar. Está bem assim, mas merece uma
  explicação visível no painel em vez de silêncio.
- **Markdown dentro de descrições é literal na UI web.** O documento gerado
  renderiza direito; a UI mostra a sintaxe crua. Corrigir significa uma
  dependência de Markdown, e foi por isso que ficou adiado em vez de feito.

---

## 5. Deliberadamente ainda não

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
