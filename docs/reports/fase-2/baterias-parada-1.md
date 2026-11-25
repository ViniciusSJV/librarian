# Arrumação das baterias de tela antigas: Parada 1

25/11/2026. Não é passo da Fase 2: não muda regra nem código de produção. Decisões do usuário no [ESTADO-ATUAL](../../../ESTADO-ATUAL.md), item "Baterias de tela antigas", bloco "Decidido". Nada foi commitado; nada foi alterado nesta sessão além deste arquivo.

**[M]** medido nesta sessão, rodando. **[L]** leitura do agente (código, saídas antigas). **[E]** estimado.

## 1. Inventário dos 12 arquivos sem commit

Vêm de outra sessão e ainda não foram validados. `git diff` e `git status` lidos inteiros **[L]**.

| Arquivo | O que muda | Serve a | Situação |
|---|---|---|---|
| `tools/precisao/lib.js` | Só acrescenta a constante `PLAYWRIGHT` (o caminho da pasta temporária de outra sessão) e a exporta. Nenhuma função muda | Item 7 (Playwright) | Parcial: aponta para a pasta temporária, que é o que a decisão quer tirar. **Não altera o que o portão mede**: `portao.js` não lê `PLAYWRIGHT`, e nenhuma linha de `sourceHash`, `prepareBinaries`, `answers`, `pool` ou `withStudio` mudou |
| `tools/precisao/telas.js` (novo) | O comando único: 12 baterias vivas, com o número de verificações de cada; falha se uma verificação falha, se a saída não é 0 ou se roda menos verificações que o registrado | Item 6 | Parcial (ajustes na seção 6) |
| `docs/reports/fase-2/dados/2.2/tela-22.js` | As 2 verificações de texto do `import` (PT e EN) passam a esperar o começo novo, "seguida pelo import do arquivo: o receptor é um módulo do projeto", com o motivo (DEC-42, 2.4a) num comentário; mais o `require` do Playwright pela variável | Consertar a 2.2 | Completo, se a execução confirmar |
| `docs/reports/fase-2/dados/2.3/tela-23.js` | Na verificação "o detalhe da linha diz o texto da resolução", uma instanciação sem `__init__` (`runs: null`) passa a esperar "decidida, sem seguir" no lugar de "seguida", com o motivo (DEC-40, 2.3b) num comentário. Atinge 4 verificações (2 casos × PT e EN); mais o `require` | Consertar a 2.3 | Completo, se a execução confirmar. A regra copia a da página (`notWalked`, `index.html:7927`) |
| `docs/reports/fase-2/dados/2.4c/tela-24c.js` | Recebe da 2.4b o que não tinha equivalente: 2 casos (`super().__init__` e uma `@property` de link único, `num_checks_available`, porque a DEC-44 tornou `is_noop` ambíguo) e 2 verificações por caixa ("nenhuma candidata vira caixa própria"; "a caixa some e a linha fica cheia"). 149 → 193 verificações | Aposentar a 2.4b | **Além do decidido**: a decisão era conferir as 98 e aposentar; estender a 2.4c não foi pedido (pergunta P2). Um defeito: "a linha fica cheia" usa `every` sobre a lista filtrada, que passa vazia se a linha não for achada |
| `docs/reports/fase-2/dados/2.4c/seeds.json` | Duas sementes novas, para os 2 casos acima | Idem | Idem |
| `docs/reports/piloto-1b/dados/g3/tela-g3.js` | Reescrita: sobe o próprio Studio (`lib.js`) sobre o projeto sintético versionado, em PT e EN, 7 verificações por idioma (14). Sai a parte do Healthchecks (notify, `process_one_flip`) | Consertar o g3 | Quase completo. Três ressalvas: a busca do grupo "sem candidato" em inglês é frágil (`.slice(0, 13)` e um `/no candidate/i` de reserva); a pasta temporária não é apagada; a expectativa do texto `unique` mudou, e o comentário cita a DEC-42, mas o texto novo é da DEC-38 (2.2) |
| `2.1/tela-21.js` | `require` pela variável; a pasta de saída pela variável `TELA_OUT` | Itens 6 e 7 | Sem `TELA_OUT`, ela ainda grava em `docs/reports/fase-2/dados/2.1/tela/`, dentro do repositório e fora do `.gitignore`; as outras usam a pasta temporária |
| `2.3b`, `2.4a`, `2.4d`, `2.4d0` (`tela-*.js`) | Só o `require` do Playwright pela variável | Item 7 | Completo para o item 7 |

Não há nada nos 12 sobre: aposentar as 18 do Studio e a parte B da `tela-estado`, o registro único (o `APOSENTADAS.md` que os comentários citam não existe), os READMEs, a regra de processo e o ESTADO-ATUAL. O `rodar.js` ainda lista as 20 baterias antigas, 18 delas a aposentar.

## 2. Trava: hash das fontes

- Árvore atual (com os 12): `sourceHash()` = **`3aeb2fa77adc9ec44716bc113822faa14395b71cb7f7217deb46b4e2b87c4726`**, 250 arquivos **[M]**. É o `--fontes` da linha de base. O `telas.js` compilou e registrou a mesma **[M]**.
- **Nenhum dos 12 entra no hash** **[M]**: nenhum está em `crates/`, `Cargo.toml` ou `Cargo.lock`, e nenhum é embutido por `include_str!`/`include_bytes!` (os embutidos de fora de `crates/` são a resposta de `experiments/` e sete arquivos de `tools/precisao/sinteticos/`, intocados).
- **O que entraria:** todo arquivo não `.md` de `crates/`. As baterias do Studio (`crates/librarian-studio/tests/tela/*.js`) e o `rodar.js` **entram**, embora não sejam compilados. O `README.md` e um `APOSENTADAS.md` nessa pasta não entram.
- **Proposta (P1):** não tocar nos `.js` de `crates/`. A parte B da `tela-estado` já não roda sem o servidor da porta 8841 e sem `LIBRARIAN_TESTE_RAIZ_RS`; o `telas.js` garante as duas coisas e exige a linha "parte B pulada". O `rodar.js` fica como registro, aposentado no README e no registro. Assim o hash não muda, e a prova é o mesmo hash antes e depois, mais o portão sem `--fontes` com o mesmo veredito e os mesmos números.
- **Alternativa:** editar o `rodar.js` (tirar as 18) e a `tela-estado.js` (tirar a parte B). O hash muda só por arquivos de teste que não são compilados; a prova seria a lista `sha256 caminho` do `sourceHash` antes e depois, com a diferença só nesses dois arquivos, mais o portão sem `--fontes`. O próximo passo (2.4e) usaria o hash novo. Recomendo a primeira.

## 3. 2.4b contra a 2.4c

A 2.4b tem 98 verificações (última execução com 98 de 98: [tela-24b.out](dados/2.4b/tela-24b.out), no `6dc0e4a`). Desde o 2.4c, ela roda 38 e para **[M, no fim deste arquivo]**: a caixa de `is_noop` passou a existir (DEC-44), e o `box.click()` acha duas caixas `many:` e lança. As 60 que não chegam a rodar estão marcadas com **(não roda)**.

Coluna "2.4c": **HEAD** = verificação que já está na 2.4c commitada (149); **árvore** = acrescentada pelos arquivos sem commit (193). Os nomes são os que a bateria imprime, sem o prefixo do idioma.

### PT: os 4 casos de link único (32, rodam)

| 2.4b | Cobertura na 2.4c |
|---|---|
| `is_noop (@property)`: as 8 verificações (seguida; linha cheia; `/api/calls` dá type → `Transport.is_noop`; basis type, candidate_edge null; o texto da resolução; type_rule propriedade; relation calls; nenhuma chave crua) | **Superadas pela DEC-44**: `is_noop` agora é ambígua, coberta pela caixa `is_noop` (HEAD, 13 verificações por idioma). A regra "@property de link único" fica com `num_checks_available (@property)`, as mesmas 8 (árvore); "candidate_edge null" fica com `super().__init__` (árvore) |
| `cls.post (herdado)`: 8 | `cls.post (herdado)`: as mesmas 8 (HEAD) |
| `super().__init__`: 8 | `super().__init__`: as mesmas 8 (árvore). **Sem a árvore: sem cobertura** |
| `self.payload (herdado)`: 8 | `self.payload (herdado)`: as mesmas 8 (HEAD) |

### PT: a caixa do F23 (10; 6 rodam, 4 não)

| 2.4b | Cobertura na 2.4c (caixa `notify (abstrato)`) |
|---|---|
| uma caixa só para as 30 (e nenhuma `notify` desenhada à parte) | "uma caixa com a contagem" (HEAD) + "nenhuma candidata vira caixa própria" (árvore) |
| a caixa diz 30 | "uma caixa com a contagem" (HEAD) |
| uma linha tracejada (exatamente 1 no desenho) | "linha tracejada" (HEAD), só da linha dela. O "exatamente 1" ficou falso pela DEC-44 (a linha de `is_noop` também é tracejada) |
| o detalhe lista as 30, com arquivo e botão | "o detalhe lista todas, com arquivo" (HEAD). **Parcial:** a 2.4b exigia `.notify` em cada item; a 2.4c confere o arquivo, "traz Webhook.notify" e "não traz Transport.notify" |
| o texto da resolução (`overrides`, 30) | "o texto diz que Transport.notify é abstrato…" (HEAD). O texto mudou pela DEC-44 (`overridesAbstract`) |
| nenhuma chave crua | "nenhuma chave crua" (HEAD) |
| clicar na caixa abre a lista **(não roda)** | "clicar na caixa abre a lista" (HEAD) |
| a escolha vira um link chosen para Slackalike.notify **(não roda)** | "a escolha vira um link chosen para Slackalike.notify" (HEAD) |
| a caixa some e a linha fica cheia **(não roda)** | "a caixa some e a linha fica cheia" (árvore; com o defeito da seção 1). A 2.4b exigia nenhuma caixa e nenhuma linha tracejada no desenho inteiro; a da árvore olha só a caixa e a linha desta chamada. **Sem a árvore: sem cobertura** |
| limpar as escolhas volta à caixa **(não roda)** | "limpar as escolhas volta à caixa" (HEAD) |

### PT: página (2, não rodam)

| 2.4b | 2.4c |
|---|---|
| nenhum erro de página **(não roda)** | "nenhum erro de página" (HEAD) |
| nada além de abrir o projeto e /api/calls **(não roda)** | a mesma (HEAD) |

### EN (44, nenhuma roda)

As mesmas 44 do PT, com a mesma cobertura, no bloco EN da 2.4c (as duas línguas estão no mesmo laço nas duas baterias).

### Temas e tela estreita (9, nenhuma roda)

| 2.4b | 2.4c |
|---|---|
| tema escuro / tema claro / tela estreita: o detalhe aparece com o texto do abstrato (3) | "…, abstrato: o detalhe aparece com o texto certo" (HEAD, 3). O texto exigido mudou pela DEC-44: de "o método do tipo é abstrato" para "Transport.notify, do tipo escrito, é abstrato" |
| tema escuro / claro: tema aplicado (2) | "…, abstrato: tema aplicado" (HEAD, 2) |
| tela estreita: sem rolagem horizontal (1) | "tela estreita (390 px), abstrato: sem rolagem horizontal" (HEAD) |
| nenhum erro de página, nos 3 (3) | as mesmas 3 (HEAD) |

A 2.4c faz também as mesmas 9 para `is_noop` (HEAD).

### Fim (1, não roda)

| 2.4b | 2.4c |
|---|---|
| `.studio` original igual antes e depois | a mesma (HEAD) |

### Resumo

**[L]**, somando a tabela (98 = 64 + 18 + 16):

- **64 cobertas pela 2.4c commitada (HEAD):** por idioma, `cls.post` 8, `self.payload` 8, F23 9 (todas menos "a caixa some") e página 2 (27 × 2 = 54); mais temas 9 e `.studio` 1. Dessas, 7 com a expectativa mudada pela DEC-44 ("exatamente 1 linha tracejada" 2, o texto do abstrato 2 + 3 nos temas) e 2 parciais ("lista as 30" sem exigir `.notify` em cada item).
- **18 só com a árvore:** `super().__init__` 16 e "a caixa some e a linha fica cheia" 2.
- **16 superadas pela DEC-44:** o link único de `is_noop`. A regra que elas exercitavam (`@property` de link único) fica com `num_checks_available` (árvore, 16).
- **Sem a árvore, ficam sem cobertura 34** (18 + as 16 da `@property` de link único). Com a árvore, nenhuma, salvo as 2 parciais.

## 4. O que falta para cada decisão

| Decisão | Feito na árvore | Falta |
|---|---|---|
| Consertar a 2.2 | 2 verificações, motivo citando a DEC-42 | Rodar (seção 7). Pôr o comentário também na linha EN (hoje um só comentário cobre as duas) |
| Consertar a 2.3 | 4 verificações (uma linha de código), motivo citando a DEC-40 | Rodar |
| Aposentar a 2.4b | Conferência feita (seção 3); a 2.4c estendida (sem aprovação) | Aprovar ou não a extensão (P2); corrigir o `every` vazio; registrar no APOSENTADAS; rodar a 2.4b uma vez para registrar a saída |
| Aposentar `tela-centro`, `tela-inspecao`, `tela-navegacao` | Fora do `telas.js` | Registro, com a nota "recriar é pergunta da Parada 1 da 2B"; README |
| Aposentar as 8 de 04/11 (`tela-cartao`, `tela-zero`, `tela-origem`, `tela-nomes`, `tela-passo2`, `tela-4a`, `tela-4b`, `etapa2c`) | Fora do `telas.js` | Registro; README |
| Aposentar as outras 7 (`tela-builder`, `tela-rascunho`, `tela-perfil`, `tela-conversa`, `tela-projeto`, `tela-revisao`, `tela-evidencia`) | Fora do `telas.js` | Registro, com o que fica sem cobertura como critério do front refeito depois do 6.5 (proposta do usuário no PLANO, ainda a decidir); README |
| Aposentar a parte B da `tela-estado` | A parte A roda no `telas.js` | Registro; `telas.js` exigir "parte B pulada" (P1) |
| `rodar.js` | — | Aposentar como comando (P1); README aponta para o `telas.js` |
| g3: consertar o sintético e o inglês, trocar a porta 8773 pelo `lib.js`, aposentar o resto | Feito (14) | As três ressalvas da seção 1; registrar a parte do Healthchecks no APOSENTADAS |

## 5. Registro único das aposentadas

**Onde:** `crates/librarian-studio/tests/tela/APOSENTADAS.md` (um arquivo só, também para a 2.4b e o g3, que estão em `docs/reports/`). É `.md`: não entra no hash. Os comentários da árvore já apontam para ele.

**Forma:** uma tabela-resumo (bateria, arquivo, data, verificações na última execução, motivo, onde está a cobertura que sobra) e, por bateria, a lista do que ela cobria e fica sem cobertura. Os arquivos ficam onde estão, intocados: aposentar é sair da lista que roda, não apagar.

| Bateria | Data | Motivo | Nota |
|---|---|---|---|
| `tela-24b.js` (98) | 25/11/2026 | A DEC-44 tornou `is_noop` ambíguo; a bateria para no meio (60 não rodam). Continuada pela 2.4c | O mapa da seção 3 |
| `tela-centro.js` (94), `tela-inspecao.js` (98), `tela-navegacao.js` (139) | 25/11/2026 | Pedem as raízes de 04/11, que não existem; os números mudariam com dados novos | **Recriar é pergunta da Parada 1 da 2B.** Ficam sem cobertura o fluxograma como vista do centro, a inspeção e a navegação |
| As 8 de 04/11 (250) | 25/11/2026 | Dependem de rodadas medidas que não existem mais | Lista por bateria, do README |
| As outras 7 (829) | 25/11/2026 | Raízes de 04/11 e projeto `backend` ausentes; controles mudados em 17/11 | Lista por bateria (a tabela de 18/11 do README, movida); critério do front refeito |
| Parte B da `tela-estado.js` (13) | 25/11/2026 | Raiz de 04/11 | Parte A continua viva |
| g3, parte do Healthchecks (7) | 25/11/2026 | Contagens de 18/11, antes da Fase 2; servidor 8773 e cópia que não existem | `ch.notify` com 33 e `process_one_flip`; o `node_limit` e o inglês continuam no sintético |
| `rodar.js` (comando) | 25/11/2026 | Só servia às baterias aposentadas | Substituído pelo `telas.js` |

## 6. Um comando para todas as vivas

`node tools/precisao/telas.js [--so <nome>] [--saida <pasta>]`, o da árvore, com ajustes.

**Vivas (12):** 2.1 (51), 2.2 (56), 2.3 (84), 2.3b (102), 2.4a (88), 2.4c (149 ou 193, P2), 2.4d-0 (240), 2.4d (71), g3 (14), `tela-estado` parte A (46), `tela-simplificacao` (47), `tela-passo6` (36).

**Código de saída:** 0 só se todas passarem; 1 se qualquer bateria falhar; 2 se não conseguir começar (Playwright ausente, compilação falha). Uma bateria falha se: uma verificação falha; a saída não é 0 (inclui cair no meio, tempo esgotado ou sinal); ou o número de verificações rodadas não é o registrado.

**Ajustes propostos:**
- Argumentos por bateria (a 2.4d recebe a pasta do binário, não a de saída): o defeito que derrubou a 2.4d nesta execução.
- Número de verificações **igual** ao registrado, não "pelo menos": uma bateria que passa a rodar mais precisa ter o número atualizado (hoje só "menos" falha).
- Por bateria, imprimir rodadas, falhas e puladas (linhas "pulada").
- `tela-estado`: tirar do ambiente as variáveis da parte B e exigir a linha "parte B pulada".
- `tela-passo6`: recusar se a porta 8842 já responde antes de subir o servidor (hoje mediria outro servidor sem aviso) e falhar se ele não ficar pronto.
- `--so` imprime "parcial" no fim, para não ser lido como a rodada inteira.
- 2.1: pasta de saída padrão na pasta temporária, como as outras.

## 7. Playwright

Hoje: `playwright-core` 1.48.2, só em `…/c2d74a42-…/scratchpad/pw/node_modules` **[M]** (8,2 MB), citado por caminho em 9 baterias e no `lib.js`.

**Proposta:** `tools/precisao/package.json` (`private`, `playwright-core` fixo em `1.48.2`, a versão em uso) e `tools/precisao/package-lock.json`, instalados com `npm ci --prefix tools/precisao`; `/tools/precisao/node_modules/` no `.gitignore`. `tools/precisao/` fica fora do hash das fontes. Precisa de rede uma vez (registro do npm; pergunta P6).

**Como é achado:** `lib.js` exporta `playwright()`: a variável `LIBRARIAN_TESTE_PLAYWRIGHT`, se houver, ou `require.resolve('playwright-core')` a partir de `tools/precisao`; sem nenhum, um erro que diz o comando `npm ci`. As baterias de `docs/` passam a `require(L.playwright())`, sem caminho fixo. O `telas.js` põe o caminho na variável para as do Studio, que já a leem (`ambiente.js`, `tela-estado.js`, `tela-simplificacao.js`): nenhuma mudança em `crates/`. O `require` é preguiçoso: o portão não passa a depender do Playwright.

## 8. Regra de processo (proposta)

**DEC-47, em DECISOES.md:**

> | DEC-47 | Baterias de tela em toda Parada 2 | Toda Parada 2 roda todas as baterias de tela vivas, com um comando só (`node tools/precisao/telas.js`), e informa o resultado de cada uma. Uma falha conta como falha, e uma bateria que não roda até o fim ou roda outro número de verificações também; nenhuma fica de fora por estar "fora do passo". Uma verificação que um passo torna falsa é corrigida nele, com o motivo escrito na própria verificação citando a DEC que mudou o texto, ou a bateria é aposentada com aprovação do usuário, no registro único (`crates/librarian-studio/tests/tela/APOSENTADAS.md`), com data, motivo e o que fica sem cobertura. Uma bateria nova entra na lista do comando com o número de verificações. | Arrumação das baterias antigas (25/11/2026), decisão do usuário | Vigente (aprovada pelo usuário em …) |

**README das telas:** a seção "Como rodar" passa a ser o `telas.js`, com a regra numa frase e o link para a DEC-47. **README do portão:** uma linha em "Como rodar": "Na Parada 2, rode também `node tools/precisao/telas.js` (DEC-47)".

Achado no lugar: em DECISOES.md, a linha da DEC-46 está depois do parágrafo "As linhas do CTX…", fora da tabela, e por isso não aparece como linha da tabela **[L]**. Proponho subir a DEC-46 (sem mudar o texto) e pôr a DEC-47 logo abaixo (P5).

## 9. Perguntas

- **P1.** Hash: não tocar nos `.js` de `crates/` (o `rodar.js` e a parte B da `tela-estado` ficam como registro; hash igual) ou editá-los (hash novo só por arquivos de teste)? **Recomendo não tocar.**
- **P2.** A extensão da 2.4c (2 casos e 2 verificações por caixa, de 149 para 193) não estava na decisão. Aceitar, com o `every` corrigido (exigir que a linha exista)? Sem ela, ficam sem cobertura `super().__init__`, a `@property` de link único e "a caixa some e a linha fica cheia". **Recomendo aceitar.**
- **P3.** g3: aceitar a reescrita (14), com as três ressalvas corrigidas (busca do grupo em inglês pelo texto exato, apagar a pasta temporária, comentário citando a DEC-38)?
- **P4.** Contagem igual ao registrado no `telas.js` (não "pelo menos")?
- **P5.** DECISOES.md: subir a linha da DEC-46 para dentro da tabela, junto da DEC-47?
- **P6.** Autoriza o `npm ci` (rede, registro do npm) para criar o `package-lock.json` com `playwright-core` 1.48.2?
- **P7.** Um commit só (baterias, `telas.js`, `package.json`, registro e documentação) ou dois (código de teste; documentação)? **Recomendo um**, por ser pequeno.

## Execução desta Parada (estado da árvore)

Rodado nesta sessão, sobre a árvore com os 12 arquivos, fontes `3aeb2fa7…` **[M]**: `node tools/precisao/telas.js` → **saída 1**, 12 baterias, 957 verificações passam, 14,2 min.

| Bateria | Resultado |
|---|---|
| 2.1 | 51 de 51 |
| 2.2 | 56 de 56 (o conserto confirma) |
| 2.3 | 84 de 84 (o conserto confirma) |
| 2.3b | 102 de 102 |
| 2.4a | 88 de 88 |
| 2.4c | 193 de 193 (com a extensão da árvore) |
| 2.4d-0 | 240 de 240 |
| **2.4d** | **FALHA: 0 de 71, saída 2.** Defeito do `telas.js`: ele passa a pasta de saída como primeiro argumento a todas, e a 2.4d lê o primeiro argumento como a pasta do binário (`node tela-24d.js [bin]`) e não acha o studio. Não é defeito do produto; a regra "não rodou até o fim conta como falha" o pegou. Correção na Parada 2: cada bateria com os próprios argumentos |
| g3 | 14 de 14 |
| `tela-estado` (parte A) | 46 de 46; imprimiu "parte B pulada" |
| `tela-simplificacao` | 47 de 47 |
| `tela-passo6` | 36 de 36 |

2.4b, uma vez **[M]**: saída 1, 32 passam, 6 falham, e para no `box.click()` (duas caixas `many:`): 38 rodadas, 60 não, como na seção 3.

Ressalva da 2.4c: "a caixa some e a linha fica cheia" passou nas 6 vezes, mas a verificação não imprime se achou a linha; com o `every` sobre lista vazia, não dá para saber se passou de verdade (P2).

## Aprovação (25/11/2026)

Aprovada pelo usuário, com as respostas: P1 não tocar nos `.js` de `crates/`, com o `rodar.js` aposentado no README e no registro, e o limite da trava no README do portão; P2 aceita, com o conserto, e a pergunta sobre as 16 que a DEC-44 tornou falsas; P3 aceita, com as ressalvas corrigidas; P4 número igual ao registrado; P5 sim; P6 `npm install playwright-core@1.48.2 --save-exact` em `tools/precisao`, e de onde vem o navegador; P7 um commit. Execução na [Parada 2](baterias-parada-2.md).
