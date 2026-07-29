# Desenvolvimento

> 🌐 [English](../development.md) · **Português (Brasil)**

## Setup

Requisitos: Rust (stable), Node + pnpm e, opcionalmente,
[`just`](https://just.systems) para o task runner.

```sh
pnpm install
cargo check
just            # lista todas as receitas
```

## Comandos do dia a dia

| Tarefa | just | cru |
| --- | --- | --- |
| Servidor de dev web | `just dev` | `pnpm --filter web dev` |
| Testes web | `just web-test` | `pnpm --filter web test` |
| Build web (tsc + vite) | `just web-build` | `pnpm --filter web build` |
| Testes da extensão | `just ext-test` | `pnpm --filter crux-analyzer-vscode test` |
| Build da extensão (embute o dist web) | `just ext-build` | — |
| Pacote `.vsix` da extensão | `just ext-package` | — |
| Site estático de docs | `just site <src> <nome> [base]` | `CRUX_BASE=<base> pnpm --filter web build` |
| Testes Rust | `just rust-test` | `cargo test --workspace` |
| Testes de aplicação alvo (privados, locais) | — | `APP_SRC=<caminho> cargo test --workspace` |
| Cobertura da aplicação alvo (privada, local) | — | `just coverage <caminho> <nome> <piso>` |
| Clippy | `just clippy` | `cargo clippy --workspace` |
| Gate de cadeia de suprimentos | `just security` | `cargo deny check` + `pnpm audit --audit-level high` |
| Notas de terceiros | `just notices` | `cargo about generate about.hbs` (+ o build web) |
| Notas estão atualizadas | `just notices-current` | — |
| Tudo | `just check` | — |
| Modelo para a UI | `just model <src> <nome>` | `cargo run -p crux-analyzer-cli -- generate ...` |
| Documentação | `just docs <src> <nome> [formato] [locale]` | `cargo run -p crux-analyzer-cli -- docs ...` |
| Cobertura de documentação | `just coverage <src> <nome> [min]` | `cargo run -p crux-analyzer-cli -- coverage ...` |
| Atualizar docs de exemplo (todos os locales) | `just example-docs` | — |
| Exemplos versionados estão atuais | `just docs-current` | — |
| Fixture extrai limpo | `just fixture-guard` | — |

## Camadas de teste

1. **Testes unitários do parser** (`crates/parser/src/tests.rs`) — um teste com
   fonte inline por padrão de extração (guardas, predicados, compostos, fluxo de
   valores, curingas, ...). Comece aqui ao adicionar um padrão.
2. **Integração com fixture** (`crates/parser/fixtures/mini_recorder/` +
   `crates/parser/tests/mini_recorder.rs`) — uma aplicação mínima em forma de Crux
   exercitando delegação, eventos aninhados e extração de múltiplas regiões.
   Fontes simples, não um crate compilado.
3. **Teste de aplicação alvo** (`crates/parser/tests/*_hidden.rs`) — roda contra
   uma aplicação real de produção, condicionado à variável de ambiente `APP_SRC`
   (pula com uma mensagem quando ausente). Verifica os conjuntos completos de
   transições esperados e **zero avisos**. Esta é a verdade fundamental da
   qualidade de extração — e ela *não está neste repositório*: um teste assim
   nomeia uma aplicação privada, seus cores e seus estados, então
   `crates/parser/.gitignore` mantém `tests/*_hidden.rs` fora do versionamento.
   Quem tem a fonte escreve o seu e aponta `APP_SRC` para ela; um clone novo
   simplesmente não tem nenhum.
4. **Testes de docgen** (`crates/docgen`) — verificações da saída dos geradores,
   por locale: que a prosa é traduzida *e* que identificadores e ids de nó Mermaid
   não são.
5. **Testes web** (vitest) — as camadas de mapeamento (`schema → domínio → flow`),
   o motor de simulação e os catálogos de mensagens (paridade de chaves, nenhuma
   entrada vazia ou não traduzida). Componentes de UI deliberadamente não têm
   testes unitários; as camadas ao redor deles têm. A paridade dos catálogos
   também é garantida pelo `tsc`, então `just web-build` faz parte dessa garantia.
6. **Testes da extensão** (vitest, `apps/vscode`) — os módulos puros: a
   transformação do HTML do webview e a resolução do diretório de fontes. As
   partes do host de extensão são encanamento fino em volta deles e são
   exercitadas manualmente (veja [vscode.md](vscode.md)).

## Pipeline de validação de um incremento

Todo incremento só entra depois de:

1. `just rust-test` e `just clippy` — limpos (com uma aplicação alvo privada na
   máquina, use `APP_SRC=<caminho> cargo test --workspace`, para que esse teste
   rode também);
2. `just web-test` e `just web-build` — verdes;
3. uma verificação da UI ao vivo: `just model <src> <nome> && just dev`, dirigir o navegador e
   **olhar** o resultado (estados, transições, inspetor, simulação);
4. commits lógicos em inglês, enviados com push.

Mudanças que mexem em texto visível ao usuário adicionam dois passos: regenerar os
exemplos versionados (`just example-docs` não deve deixar diff para `en`) e
verificar a UI em **ambos** os locales — uma tradução mais longa muda as larguras
dos nós, então o grafo é re-layoutado, não apenas re-renderizado.

Para mudanças no parser que alteram a semântica de extração, adicione uma
verificação cruzada adversarial: derive independentemente as transições esperadas
a partir da fonte analisada e compare com a saída da CLI antes de confiar nos
testes.

### O que o CI garante

O `.github/workflows/ci.yml` roda `just check` (que agora inclui o
`fixture-guard`) mais `just docs-current`. Juntos eles cobrem as três formas pelas
quais este projeto pode apodrecer em silêncio:

| Guarda | Pega |
| --- | --- |
| `just check` | testes quebrados, clippy, uma chave faltando no catálogo de mensagens (`tsc`) |
| `security` | um aviso de segurança em dependência, uma licença fora do conjunto permitido, uma dependência git ou com wildcard |
| `notices-current` | uma dependência adicionada sem a nota dela chegar ao `THIRD-PARTY-NOTICES.md` |
| `fixture-guard` | o fixture começando a emitir avisos, ou sua documentação regredindo abaixo do piso |
| `docs-current` | um exemplo gerado versionado que não corresponde mais ao gerador |

O `just security` instala o `cargo-deny` no primeiro uso, e o `just notices`
instala o `cargo-about` do mesmo jeito — um gate que as pessoas pulam porque
precisa de setup não é um gate. As políticas vivem no `deny.toml` e no
`about.toml` (as listas de licenças aceitas precisam concordar), e o
[security.md](security.md) explica o que elas defendem.

O `THIRD-PARTY-NOTICES.md` é gerado a partir do **que cada artefato realmente
distribui**, não do que está instalado: a metade web a partir dos chunks que o
bundler emitiu (`apps/web/notices.ts`), a metade Rust a partir das crates ligadas
ao binário. Então adicionar uma dependência muda esse arquivo, e o
`notices-current` é o que torna esquecer isso um build vermelho em vez de uma
violação de licença silenciosa.

Note que `pnpm install --frozen-lockfile` ainda executa scripts de ciclo de vida
das dependências, então o CI executa os hooks de instalação de cada dependência
transitiva. Esse é o principal motivo pelo qual uma nova dependência merece uma
olhada em vez de um `pnpm add`.

Um teste de aplicação alvo se auto-restringe por `APP_SRC` e fica fora do
versionamento, porque a aplicação que ele nomeia é privada — então o CI comprova
o caminho do fixture e uma aplicação real continua sendo uma guarda local.
Mantenha assim ao adicionar guardas: o que o CI não consegue rodar não é uma
guarda.

A cada push na `main`, o CI também publica um **preview vivo**: o fixture
mini-recorder analisado pelo analisador recém-compilado e publicado no GitHub
Pages via `just site` — a mesma receita que os usuários rodam, apontada para o
fixture público. Se o preview estiver errado, o release também estaria.

Uma aplicação alvo privada não ganha catraca neste repositório: ela não está
aqui para ser catracada. Rode `just coverage <caminho> <nome> <piso>` contra ela
localmente quando quiser essa guarda — a contraparte pública, o `fixture-guard`,
é a que o CI mantém clicando.

Uma guarda que não pode falhar é decoração. Quando adicionar uma, quebre-a de
propósito uma vez e veja-a ficar vermelha antes de confiar nela.

## Convenções

- **Inglês é o idioma de origem** — commits, código, comentários, descrições do
  schema. Texto visível ao usuário (UI, saída da CLI, avisos, rótulos gerados) é
  localizado: vive nos catálogos de locale, nunca como literal inline. Veja
  [i18n.md](i18n.md).
- **Regra da honestidade** — o parser avisa sobre tudo que não consegue inferir;
  ele nunca adivinha e nunca descarta em silêncio. Novos recursos de inferência
  devem manter uma aplicação alvo real livre de avisos ou explicar cada aviso restante. Ler o
  que a fonte *declara* é permitido — anotações são dados que o parser pode
  reportar — mas inferir continua proibido, e palpites ficam nos clientes.
- **Evidência acima de forma** — heurísticas de detecção (máquinas, compostos,
  enums de evento aninhados) se baseiam em como o código *usa* um tipo, não em
  como ele se parece. Siga esse princípio ao estender a detecção.
- Mudanças de schema chegam com: schema + `crates/model` (+ teste de ida e volta)
  + exemplo embutido + docgen + camadas de schema/domínio da web + testes, em um
  único commit.

## Mapa da documentação do repositório

- `README.md` — porta de entrada; início rápido.
- `docs/` — o conjunto de documentação (inglês, a fonte); `docs/pt-BR/` é seu
  espelho em português.
- `CLAUDE.md` — acordos de trabalho para desenvolvimento assistido por IA
  (mantido em sincronia com as regras de arquitetura).
- `docs/roadmap.md` — a fonte única do trabalho planejado. Acrescente nele em vez
  de começar uma lista em outro lugar, e registre também o que você decidir *não*
  fazer.
- `init.md` — a especificação original do projeto (português, histórica).
