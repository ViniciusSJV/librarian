# Baterias de tela aposentadas

Registro único das baterias de tela que saíram da lista que roda. Decisão do usuário em 25/11/2026 (arrumação das baterias antigas; [Parada 1](../../../../docs/reports/fase-2/baterias-parada-1.md), [Parada 2](../../../../docs/reports/fase-2/baterias-parada-2.md)). Regra: [DEC-47](../../../../DECISOES.md).

**Aposentar é sair da lista que roda, não apagar.** Os arquivos ficam onde estão, sem mudança, como registro e como ponto de partida se alguém recriar uma delas. Nenhuma conta como verificação.

**O único comando das baterias vivas é `node tools/precisao/telas.js`**, com a lista delas e o número de verificações de cada uma ([README](README.md)). Uma bateria nova ou recriada entra nessa lista; uma que sai entra aqui, com data, motivo e o que fica sem cobertura.

Os números de verificações são os da última execução registrada **[M, na época]**. O que fica sem cobertura é leitura do agente **[L]**, a partir do código de cada bateria e do README de 18/11/2026, salvo onde diz outra coisa.

## Resumo

| Bateria | Arquivo | Verificações | Aposentada em | Motivo |
|---|---|---|---|---|
| `rodar.js` (comando, não bateria) | `crates/librarian-studio/tests/tela/rodar.js` | — | 25/11/2026 | Só servia às baterias aposentadas abaixo. Substituído pelo `telas.js` |
| 2.4b | `docs/reports/fase-2/dados/2.4b/tela-24b.js` | 98 | 25/11/2026 | A DEC-44 (2.4c) tornou `is_noop` ambíguo; desde o 2.4c ela roda 38 de 98 e para. Continuada pela 2.4c |
| `tela-centro`, `tela-inspecao`, `tela-navegacao` | `crates/librarian-studio/tests/tela/` | 94, 98, 139 | 25/11/2026 | Pedem as raízes de 04/11, que não existem; os números mudariam com dados novos. **Recriar é pergunta da Parada 1 da 2B** |
| As 8 das medições de 04/11: `tela-cartao`, `tela-zero`, `tela-origem`, `tela-nomes`, `tela-passo2`, `tela-4a`, `tela-4b`, `etapa2c` | idem | 37, 29, 32, 12, 9, 30, 56, 45 (250) | 25/11/2026 | Dependem de rodadas medidas que não existem mais (`20261104-014710`, `20261104-020244`, `20261029-053623`, `.studio/gabarito`) |
| As outras 7 do Studio: `tela-builder`, `tela-rascunho`, `tela-perfil`, `tela-conversa`, `tela-projeto`, `tela-revisao`, `tela-evidencia` | idem | 102, 73, 150, 102, 219, 158, 25 (829) | 25/11/2026 | Raízes de 04/11 e projeto Python `backend` ausentes; controles e textos mudados pela simplificação de 17/11 |
| Parte B da `tela-estado` | `crates/librarian-studio/tests/tela/tela-estado.js` | 13 | 25/11/2026 | Pede o servidor do `rodar.js` em 8841 sobre a raiz de 04/11. A parte A continua viva |
| Parte do Healthchecks do g3 | `docs/reports/piloto-1b/dados/g3/tela-g3.js` (versão de `HEAD` até o commit da arrumação) | 7 de 12 | 25/11/2026 | Contagens de 18/11, antes da Fase 2; servidor na porta 8773 e cópia que não existem mais. A parte sintética e a do inglês foram consertadas e continuam vivas |

Execução de despedida, uma vez, na Parada 2 da arrumação: resultado no fim deste arquivo.

## `rodar.js`

Subia dois servidores (8841 e 8842) sobre as raízes de teste e rodava, nesta ordem: `tela-cartao`, `tela-zero`, `tela-origem`, `tela-nomes`, `tela-passo2`, `tela-4a`, `tela-4b`, `tela-passo6`, `etapa2c`, `tela-evidencia`, `tela-estado`, `tela-navegacao`, `tela-inspecao`, `tela-centro`, `tela-builder`, `tela-rascunho`, `tela-perfil`, `tela-conversa`, `tela-projeto`, `tela-revisao`. Das 20, ficam vivas pelo `telas.js` a `tela-passo6` e a parte A da `tela-estado`; as outras 18 estão aqui. A `tela-simplificacao`, viva, nunca esteve nele.

O arquivo não foi alterado: os `.js` de `crates/` entram no hash das fontes do portão (limite registrado no [README do portão](../../../../tools/precisao/README.md)), e mudá-lo mudaria a trava sem mudar nada compilado.

## 2.4b (`tela-24b.js`, 98)

A conferência das 98 contra a 2.4c, uma a uma, está na seção 3 da [Parada 1](../../../../docs/reports/fase-2/baterias-parada-1.md). Resultado:

- 64 cobertas pela 2.4c como estava (7 com o texto esperado mudado pela DEC-44; 2 parciais).
- 18 cobertas pelo que a arrumação acrescentou à 2.4c: `super().__init__` (16) e "a caixa some e a linha fica cheia" (2).
- 16 tornadas falsas pela DEC-44: o link único de `is_noop`. O comportamento novo (ambíguo, caixa própria com 14) é verificado pela caixa `is_noop` da 2.4c. A regra que elas exercitavam (`@property` de link único) passou a `num_checks_available` (16, acrescentadas).

**Fica sem cobertura:**
- "o detalhe lista as 30" exigia `.notify` em cada item; a 2.4c confere o arquivo de cada item, a presença de `Webhook.notify` e a ausência de `Transport.notify` (parcial).
- "exatamente uma linha tracejada no desenho" deixou de ser verdade (DEC-44) e não tem substituto; a 2.4c confere a linha de cada chamada.

## `tela-centro`, `tela-inspecao`, `tela-navegacao` (331)

**Recriar é pergunta da Parada 1 da 2B** (com uma raiz pequena gerada pela própria bateria, o `renderer` e o sintético Python, e cada número esperado revisto; estimado em 2 a 3 sessões). São as que cobriam o fluxograma como vista da página.

**Fica sem cobertura:**
- `tela-centro` (F4): a barra de vistas do centro (Cidade, Fluxograma com título, estado e fechar); ir à Cidade e voltar preservando câmera, seleção, inspeção, semente, modo, profundidade, opções, escolhas e desenho; substituir e fechar com confirmação quando há escolhas de homônimos; respostas antigas ou de outra run ignoradas; a troca de run fechando o fluxograma (T14); as perguntas por ponto (`askPath`); a vista central guardada e restaurada; isolamento entre projetos; tela estreita, PT e EN, temas, teclado e o modo arquivo.
- `tela-inspecao` (F3): o clique num prédio abrindo a inspeção sobre a lista da vista e "← voltar à lista" devolvendo-a como estava; "Selecionado: X [Inspecionar]" fora de Explorar; Voltar à área de origem; a pilha de relações (T3); tela estreita, PT e EN, teclado, foco, temas e o modo arquivo.
- `tela-navegacao` (F2): as quatro áreas e ferramentas, as vistas de Explorar, Voltar só dentro do projeto aberto, a navegação guardada por projeto, o isolamento do grafo, da seleção, da inspeção e do fluxograma entre projetos, a conversão de `librarian.tab`, o cabeçalho, a tela estreita com o alternador, PT e EN, temas, teclado e o modo arquivo.

**O que continua coberto:** o desenho de "o que chama?" e "onde é usado?" (linhas, caixas, detalhe, escolha de homônimo, lista de não apresentadas) pelas baterias 2.1 a 2.4d e pelo g3; o estado local da navegação no navegador pela parte A da `tela-estado`.

## As 8 das medições de 04/11 (250)

Dependem de rodadas medidas que não existem mais, e os números esperados (`6.410.258`, 38 e 125 no zero, 12 candidatos de `default`) mudariam com dados novos.

**Fica sem cobertura de tela** (a API de cada uma não foi conferida aqui):
- `tela-cartao` (37): o cartão diz de onde veio a resposta (`execution`, ambíguo, não encontrado), com e sem medição.
- `tela-zero` (29): a semântica do zero e a resolução do alvo no `renderer` real.
- `tela-origem` (32): o texto de cada `count_kind` no ranking, em Separar, na cidade, no fluxograma e na pergunta.
- `tela-nomes` (12): função com mais de uma linha contada, nas mesmas telas.
- `tela-passo2` (9): ordem por contagem, sem alvo automático, seletor de Separar.
- `tela-4a` (30): o ranking como Top 20 e as listas de sem passagens e sem cobertura, comparadas com `/api/hotspots`, em PT e EN.
- `tela-4b` (56): o estado de cada item no painel da sugestão, comparado com `/api/envelope/suggest`, sem medição e com estado ausente ou desconhecido, em PT e EN.
- `etapa2c` (45): os bloqueadores e as combinações de `origin.selection` gravadas pelo servidor. Os bloqueadores no servidor têm testes Rust em `store.rs` (`notes`, `accept_selection`, `included_bytes`; README de 18/11).

**O que continua coberto:** as contagens manuais e o projeto sem fonte Rust (`tela-passo6`).

## As outras 7 do Studio (829)

A lista do que fica sem cobertura é proposta como critério do front refeito depois do 6.5 (proposta do usuário no PLANO, ainda a decidir). Recriar foi estimado em 5 a 8 sessões **[E]**.

| Bateria | Já coberto por outra (README de 18/11) | **Fica sem cobertura** |
|---|---|---|
| `tela-builder` (102) | O corpo novo da escada: `tela-simplificacao` e testes Rust | A Seleção como vista do centro, as quatro listas iguais ao registro, a proveniência, a confirmação ao substituir um rascunho, a sequência dos pedidos (resposta antiga descartada), Juntar desabilitado durante uma entrada que recomeça, o achado sem seleção |
| `tela-rascunho` (73) | Nada | O rascunho guardado por projeto e run e restaurado ao recarregar, com o corpo de `/api/gathered-question` igual byte a byte; edition trocada, fonte inexistente e formato desconhecido; duas abas; armazenamento bloqueado e cheio na página real (o módulo `localState` isolado continua na parte A da `tela-estado`) |
| `tela-perfil` (150) | O corpo da escada: `tela-simplificacao` | Alterar a política gravando só `allow_hypothesis` e `allow_patch`; nenhum outro fluxo gravando o perfil; T26 em cada ação que exige um modo; os critérios do rascunho; o caminho previsto (3.11); o indicador de não salvo |
| `tela-conversa` (102) | O envio pela conversa (não existe mais) e a seção de pacote: `tela-simplificacao` | A linha do tempo e os estados de cada pergunta contra `/api/turns` e `/api/executions`; as três camadas de cada envio; Copiar diff; respostas atrasadas e de outra run; carregando, vazio e ilegível |
| `tela-projeto` (219) | Nada | O resumo da área Projeto contra cinco APIs; a run guardada por projeto; a confirmação ao sair com alterações não salvas (T25) e o aviso ao recarregar (T17); a Régua; o Acervo (filtro, Ir à origem, Revogar, Juntar a declaração) |
| `tela-revisao` (158) | Parte A, em parte: a vista Pacote, H1 a H4, Nenhum não envia (`tela-simplificacao`); o vínculo do envio com o hash: testes Rust | A revisão individual com os nomes novos; o lote (4.5); "Aprovar todos" |
| `tela-evidencia` (25) | H1 a H4 e a proveniência na vista Pacote (`tela-simplificacao`); `/api/evidence`: testes Rust | Pacote de formato 1 e 2 na tela, `declared_by` de cada ficha, o aviso de `measure_diverges`, tema escuro e tela estreita |

## Parte B da `tela-estado` (13)

Com o servidor do `rodar.js` em 8841: a página não grava nada do F1 no navegador nem no perfil ao abrir o projeto, trocar de run e recarregar; trocar de run guarda só a run (F9); recarregar volta à run escolhida; funciona com o armazenamento bloqueado; o `project.json` fica igual em bytes.

**Fica sem cobertura:** tudo isso, na página real com servidor. A parte A (46, viva) cobre o módulo `localState` isolado. O `telas.js` tira do ambiente as variáveis da parte B, recusa rodar com a porta 8841 ocupada e exige a linha "parte B pulada"; o arquivo não foi alterado (hash das fontes).

## Parte do Healthchecks do g3 (7 das 12)

Saíram: `sendalerts.notify` com 11 chamadas não apresentadas (1 acima do limite, `ch.notify(flip)` com 33 candidatos; 10 sem candidato) e o desenho com 5 caixas (4); `process_one_flip` sem nada desenhado, com a nota apontando para a seção e `executor.submit` na lista (2); o resumo em inglês sobre o Healthchecks (1, que aceitava também o texto em português).

**Fica sem cobertura:** a nota "Nenhuma chamada a desenhar" com a seção preenchida, e o grupo "acima do limite de homônimos" com a contagem de candidatos na lista de não apresentadas, sobre dados reais. A 2.2 cobre o receptor acima do limite e o motivo de `no_candidate_in_project` na lista **[L]**.

**Continua vivo (14):** a parte sintética (o `node_limit` em grupo próprio, com o alvo e a resolução original de cada item, `items[0]` sem candidato, o aviso de truncado) em PT e EN, com o próprio Studio, sem a porta 8773.

## Execução de despedida

25/11/2026, uma vez, na Parada 2 da arrumação, com o código de `6254641` (fontes `3aeb2fa7…`) **[M]**. Nenhuma conta como verificação; o registro é só para mostrar que saíram e por quê.

| Bateria | Saída | Verificações | O que a fez parar |
|---|---|---|---|
| As 18 do Studio (`tela-cartao` a `tela-evidencia`) e o `rodar.js` | 2 | 0 | "Falta a variável de ambiente `LIBRARIAN_TESTE_RENDERER`" (`ambiente.js`): não há raiz de 04/11 para apontar. Sem `LIBRARIAN_TESTE_PLAYWRIGHT`, param antes, com saída 1, sem achar o `playwright-core` |
| Parte B da `tela-estado` | — | 0 | Pulada ("parte B pulada: sem servidor em 8841"), dentro da execução viva da parte A pelo `telas.js` |
| 2.4b | 1 | 38 de 98 (32 passam, 6 falham) | O `box.click()` acha duas caixas `many:` (a de `is_noop`, desde a DEC-44) e lança |
| g3, versão antiga (de `HEAD`) | 1 | 0 | `ECONNREFUSED 127.0.0.1:8773` |
