# Extensão do VS Code

> 🌐 [English](../vscode.md) · **Português (Brasil)**

`apps/vscode` — as máquinas de estado ao lado do código. Um comando,
**Crux Analyzer: Mostrar máquinas de estado**, abre um painel que renderiza o
workspace analisado: máquinas, estados, transições, documentação autoral,
filtro por etiqueta e simulação — tudo o que a [UI web](web-ui.md) faz, porque
ele *é* a UI web. O painel regenera a cada salvamento de `.rs`, o que faz dele
o loop de autoria: escreva um comentário de documentação, salve, veja o
diagrama aprendê-lo.

## Como funciona

A extensão é um cliente do mesmo contrato JSON que todos os outros clientes.
Ela nunca faz parse de Rust: executa a CLI `crux-analyzer`, lê a stdout do
`generate` e entrega o modelo ao **bundle web compilado** embutido na extensão
(`media/web`, produzido por `just ext-build`).

Um webview difere do site estático para o qual o bundle foi construído em três
pontos, cada um tratado por uma reescrita em `src/webviewHtml.ts` (pura,
testada em unidade):

- URLs de assets absolutas na raiz são re-enraizadas em `asWebviewUri`;
- todo script — incluindo os blocos pre-paint do bundle — roda sob um CSP
  travado por nonce;
- não há origem HTTP de onde buscar `model.json`, então o modelo é injetado
  como `window.__CRUX_MODEL__` — o contrato de embutimento que o
  `loadProject` honra antes de sequer tentar buscar.

Avisos do parser não são descartados (regra da honestidade): eles aparecem no
canal de saída **Crux Analyzer** a cada regeneração.

## Configuração inicial

A extensão precisa da CLI:

```sh
cargo install --path crates/cli   # de um checkout do crux_analyzer
```

Compile e instale a própria extensão:

```sh
just ext-package                  # produz apps/vscode/crux-analyzer-vscode-<versão>.vsix
code --install-extension apps/vscode/crux-analyzer-vscode-*.vsix
```

## Configurações

| Configuração | Padrão | Significado |
| --- | --- | --- |
| `cruxAnalyzer.binary` | `crux-analyzer` | Caminho da CLI (padrão: resolvido no PATH). |
| `cruxAnalyzer.src` | *(vazio)* | Fontes a analisar, relativo à raiz do workspace. Vazio tenta `shared/src` (o layout Crux convencional), depois `src`. Um valor explícito vence mesmo se ausente — o erro do próprio analisador é melhor que analisar silenciosamente outro lugar. |
| `cruxAnalyzer.projectName` | *(vazio)* | Nome mostrado no painel; vazio usa o nome da pasta do workspace. |
| `cruxAnalyzer.watch` | `true` | Regenera quando um arquivo `.rs` sob o diretório analisado muda. |

## Localização

Os pontos de contribuição (título do comando, descrições de configuração)
vivem em `package.nls.json` / `package.nls.pt-br.json`; mensagens de runtime
passam por `vscode.l10n` com `l10n/bundle.l10n.pt-br.json`. O conteúdo do
painel segue o próprio seletor de idioma da UI web, independente do idioma do
editor — é o mesmo bundle com as mesmas regras ([i18n.md](i18n.md)).

## Desenvolvimento

| Tarefa | Receita |
| --- | --- |
| Testes de unidade (HTML do webview, resolução de fontes) | `just ext-test` |
| Compilar + embutir o bundle web | `just ext-build` |
| Empacotar um `.vsix` | `just ext-package` |

Teste e build fazem parte do `just check`. As partes do host de extensão
(painel, watcher, ativação) são deliberadamente encanamento fino em volta dos
módulos puros — as decisões de mapeamento e renderização vivem todas no bundle
web, que tem suas próprias camadas de teste.
