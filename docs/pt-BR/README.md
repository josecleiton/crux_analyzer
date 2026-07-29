# Documentação do crux_analyzer

> 🌐 [English](../README.md) · **Português (Brasil)**

O crux_analyzer é um **analisador semântico** para aplicações Rust +
[Crux](https://redbadger.github.io/crux/): ele extrai estaticamente as máquinas
de estado de cada Core — estados, transições, eventos que as disparam e efeitos
solicitados — para um modelo intermediário, e transforma esse modelo em
documentação viva (UI web interativa, documentos Mermaid/Markdown).

| Documento | O que cobre |
| --- | --- |
| [Arquitetura](architecture.md) | Layout do monorepo, o design orientado a modelo, regras rígidas, fluxo de dados |
| [Parser](parser.md) | Como a extração funciona: detecção, guardas, fluxo de valores, compostos, avisos |
| [Schema](schema.md) | O contrato JSON que todo cliente consome |
| [CLI](cli.md) | `crux-analyzer generate` / `docs`, `--watch`, formatos |
| [UI Web](web-ui.md) | Seções, inspetor, filtros, simulação, motor de layout |
| [Extensão do VS Code](vscode.md) | As máquinas de estado ao lado do código, regenerando ao salvar |
| [Internacionalização](i18n.md) | Locales, catálogos, o que nunca deve ser traduzido |
| [Segurança](security.md) | Modelo de ameaça, as regras de desenvolvimento, o que é garantido deliberadamente |
| [Desenvolvimento](development.md) | Setup, testes, corpus, convenções, pipeline de validação |
| [Roadmap](roadmap.md) | Trabalho planejado, em ordem, e o que deliberadamente não será feito |
| [Exemplo de saída](examples/mini-recorder.md) | Documentação gerada pela CLI a partir do fixture de teste |

O conjunto em inglês é a **fonte**; esta versão o espelha. Veja
[i18n.md](i18n.md) para saber o que se traduz e o que não se traduz.

## O resumo da ideia

Aplicações Crux são máquinas de estado por construção: um `Model`, um enum
`Event` e uma função `update` que move o modelo entre estados e solicita
`Effect`s. Essa estrutura *é* documentação — mas vive espalhada por braços de
`match`, guardas e funções auxiliares, e desatualiza no instante em que ninguém
está olhando.

O crux_analyzer lê o código-fonte (via a AST do `syn` — ele nunca depende do
Crux em si), reconstrói as máquinas e emite um pequeno modelo JSON. Todo o
resto — a UI web, os geradores de documentação, a extensão do VS Code,
clientes futuros (PlantUML/HTML) — consome apenas esse modelo. Rode com `--watch` e a
documentação se mantém viva conforme o código muda.
