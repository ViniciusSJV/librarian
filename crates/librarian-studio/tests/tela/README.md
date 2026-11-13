# Baterias de tela do Studio

Regressão dos Passos 1, 2, 4 e 6 da Etapa 3 e da seleção da Etapa 2 (`origin.selection`), na página real, num
Chrome sem janela, contra o servidor real. Cobrem o que os testes em Rust não alcançam: o que a tela diz, o
cartão de desempenho, a dica da cidade, o fluxograma e o registro da seleção.

Elas ficam fora do `cargo test` e do CI: o cargo não executa arquivos `.js`, e as baterias precisam de um
ambiente e de dados que não estão no repositório.

| Bateria | Verificações na última execução | O que protege |
|---|---|---|
| `tela-cartao.js` | 37 | O cartão diz de onde veio a resposta (`execution`, ambíguo, não encontrado), com e sem medição |
| `tela-zero.js` | 29 | A semântica do zero e a resolução do alvo no `renderer` real |
| `tela-origem.js` | 32 | O texto de cada `count_kind` no ranking, em Separar, na cidade, no fluxograma e na pergunta |
| `tela-nomes.js` | 12 | Função com mais de uma linha contada, nas mesmas telas |
| `tela-passo2.js` | 9 | Ordem por contagem, sem alvo automático, seletor de Separar |
| `tela-4a.js` | 30 | Passo 4, entrega 4A: ranking identificado como Top 20 e as listas de sem passagens e sem cobertura, comparadas com `/api/hotspots`, em português e inglês |
| `tela-4b.js` | 56 | Passo 4, entrega 4B: o estado de cada item no painel da sugestão, comparado com `/api/envelope/suggest`, sem medição, com estado ausente ou desconhecido, em português e inglês |
| `tela-passo6.js` | 36 | Passo 6: contagens manuais, inclusive depois de uma execução que falha, projeto sem fonte Rust e a mesma pergunta antes e depois da medição, em projetos Python e misto criados na hora, em `LIBRARIAN_TESTE_SAIDA` |
| `etapa2c.js` | 45 | Os bloqueadores e as combinações de `origin.selection`, gravadas pelo servidor real |
| `tela-evidencia.js` | 25 | Etapa 4, Passo 14: a seção "Pacote de evidências e conferências" de cada pergunta, comparada com `GET /api/evidence` — pergunta nova com seleção (formato 2, com a comparação das duas medições), pergunta antiga sem pacote e pacote de formato 1 com o aviso de `measure_diverges`; H1 a H3 conferidos e H4 à parte; `declared_by` de cada ficha visível; nenhum "aprovado" na tela; consultar não muda a resposta da API; em português e inglês, tema escuro e tela estreita. Cria uma pergunta na raiz (como `etapa2c.js`); nada é enviado |
| `tela-estado.js` | 46 + 13 | F1 do novo frontend: o estado local (navegação por projeto, rascunho por projeto e run, edition pelo `catalog_sha256`, duas abas, armazenamento bloqueado e cheio), numa página servida pela própria bateria, sem dados; e, com o servidor de `rodar.js`, que a página grava só a navegação do projeto (F2) e nunca o perfil |
| `tela-navegacao.js` | 139 | F2 do novo frontend: as quatro áreas e quatro ferramentas, as vistas de Explorar, Voltar, o ponto de partida, a navegação guardada por projeto, o Voltar só dentro do projeto aberto (abrir outro projeto limpa a pilha), o grafo, a seleção, a inspeção e o fluxograma de um projeto que não passam para outro (com e sem runs) e a conversão única de `librarian.tab`, o cabeçalho, a tela estreita com o alternador, PT e EN, temas, teclado e o modo arquivo; nada gravado no servidor |
| `tela-inspecao.js` | 98 | F3 do novo frontend: o clique num prédio (clique de mouse de verdade) dentro de Explorar abre a inspeção sobre a lista da vista (Estrutura, Onde pesa, Inventário) e "← voltar à lista" a devolve como estava (consulta, resultados, filtros, dobras, rolagem); fora de Explorar a área não muda e aparece "Selecionado: X [Inspecionar]", que leva à vista guardada de Explorar; Voltar à área de origem; a pilha de relações (T3); tela estreita; PT e EN, teclado, foco, temas e o modo arquivo; nada gravado no servidor |
| `tela-centro.js` | 94 | F4 do novo frontend: a barra de vistas do centro (Cidade sempre; o Fluxograma enquanto aberto, com título, estado e fechar), ir à Cidade e voltar preservando câmera, seleção, inspeção, semente, modo, profundidade, opções, escolhas e o desenho, substituir e fechar com confirmação quando há escolhas de homônimos, respostas antigas ou de outra run ignoradas, a troca de run fechando o fluxograma (T14), as perguntas por ponto (`askPath`) que não entram na conversa de outra run, a vista central guardada na navegação do projeto e restaurada só quando dá para refazer, o isolamento entre projetos, tela estreita, PT e EN, temas, teclado e o modo arquivo; nada gravado no servidor (as duas escritas do caso de `askPath` são respondidas pela própria bateria) |
| `tela-builder.js` | 102 | F5 do novo frontend: o Evidence Builder como vista do centro (aba com contador, fechar só esconde, voltar pela barra, pelo compositor e pelo cabeçalho), a lista fora do compositor (uma linha só), Juntar pela inspeção e pelo fluxograma sem trocar de vista, as quatro listas do formato 2 iguais ao registro (Sugerir, Juntar as sugeridas, Remover, Incluir), a proveniência registrada separada do "chegou por" local, a confirmação ao substituir um rascunho não vazio (Otimizar, Investigar, Limpar; Cancelar não muda nada), a sequência dos pedidos (resposta antiga ou de outra run descartada, com aviso) e Juntar desabilitado durante uma entrada que recomeça, o achado em modo achado sem seleção, "Pedir esta mudança" pelo Builder com o corpo da escada, o modo necessário, "já preparado" e "alterado depois", a recusa do servidor como veio, troca de run e de projeto, tela estreita, PT e EN, temas e teclado; nada gravado no servidor (`PUT /api/project` e `POST /api/gathered-question` são respondidos pela bateria; `/api/ladder` é abortado) |
| `tela-rascunho.js` | 73 | F6 do novo frontend: o rascunho do Builder guardado neste navegador, por projeto e run (`librarian.draft.v1:<key>:<run>`): recarregar restaura o mesmo rascunho (as quatro listas, a proveniência, o "chegou por", o texto, a escada, o modo necessário e o "já preparado") e o corpo de `/api/gathered-question` sai igual byte a byte; restaurar não grava o perfil nem troca o modo; trocar de run guarda e restaura sem levar o rascunho a outra run; outro projeto e Recomeçar; edition trocada, fonte inexistente (recuperar só o texto) e formato desconhecido não se aplicam nem são sobrescritos; duas abas (só leitura, sem mesclar, carregar a outra ou manter a minha); armazenamento bloqueado (aviso ao sair) e cheio; o Builder de volta à frente ao recarregar (T17); tela estreita, PT e EN, temas e teclado; nada gravado no servidor |
| `tela-perfil.js` | 150 | F7 do novo frontend: o modo como a política gravada no perfil de cada projeto (abrir e recarregar com qualquer valor em `librarian.mode`, política personalizada, nenhuma abertura grava), Alterar a política gravando só `allow_hypothesis` e `allow_patch` (também em bytes, no `project.json` da cópia) e mudando o seletor só depois do sucesso, falha simulada e rede interrompida, perfil alterado fora, Salvar com a lista dos campos sobre o perfil relido, nenhum outro fluxo gravando (Gerar e Exportar com as três escolhas, Sugerir e Aplicar léxico, Perguntar, Preparar, escada, cadeia, Separar), T26 em cada ação que exige um modo (escada, Separar, cadeia, achado, caminho, o cartão "Pedir esta mudança"), Otimizar e o cartão sem trocar o modo, os critérios do rascunho (editar, bloquear com as três opções, Salvar como padrão, Desfazer, guardar e restaurar, o padrão que mudou, por run e por projeto, rascunho anterior ao F7), o texto mantido ao preparar, o caminho previsto (3.11), o indicador de não salvo, tela estreita, PT e EN, temas, contraste e teclado. O perfil é respondido pela bateria; só um caso deixa `PUT /api/project` chegar ao servidor e restaura o `project.json` depois |
| `tela-conversa.js` | 102 | F8 do novo frontend: a linha do tempo na ordem dos números e os estados de cada pergunta conferidos contra `/api/turns` e `/api/executions` (139 perguntas reais), perguntas sem pacote e com pacote de formato 1 e 2, as execuções e os fatos de cada pergunta (inclusive desatualizado e revogado), a política e os critérios usados pela pergunta separados dos atuais, cada envio em três camadas, LACUNA, INFERENCE e hipótese sem mudança sem "Aprovar", Copiar diff (o texto entregue igual em bytes a `check.diffs[claim]`, linhas e arquivos, nenhuma escrita em `.studio` nem no projeto, teclado, área de transferência recusada, proposta sem diff), a pergunta ativa guardada por run, respostas atrasadas e de outra run ou projeto (perguntar, enviar, cadeia, decidir, apagar, rodar com a mudança, duas perguntas em andamento), os estados carregando, vazio e ilegível, a resposta colada não conferida, tela estreita, PT e EN, temas, contraste e teclado. As escritas que ela precisa ver são respondidas pela própria bateria (algumas seguradas até a troca de run); nada chega ao servidor |
| `tela-projeto.js` | 219 | F9 do novo frontend: o resumo de estado da área Projeto conferido contra `/api/commands`, `/api/hotspots`, `/api/facts`, `/api/turns` e o perfil, com o estado de cada item (carregando, falha de leitura, sucesso), falhas que não afetam os outros itens e o destino de cada item com o ponto no Voltar; a run escolhida guardada por projeto (recarregar e reabrir voltam a ela, com a pergunta ativa e o rascunho dela; run guardada que não existe mais; armazenamento bloqueado; resposta atrasada da run restaurada; abrir e recarregar não gravam nada); a confirmação ao sair de um projeto com alterações não salvas (T25: Abrir sem salvar, Salvar e abrir, Cancelar, o Salvar cancelado e recusado, pelos caminhos Abrir, Enter, Procurar e Limpar); o aviso do navegador ao recarregar e ao fechar (T17), com e sem edições, com rascunho guardado, armazenamento bloqueado, conflito entre abas e Salvar recusado; a Régua (versão, aprovação nesta versão, linha de base em vigor, Rodar desabilitado numa run com alterações, "Abrir a pergunta #N", "→ Onde pesa"); o Acervo (filtro por estado, Ir à origem na mesma run e em outra com T13, Revogar… no controle de decisão, Juntar a declaração, os estados); Rust com medição e Python sem `instrument` (9.10); tela estreita, PT e EN, temas, contraste e teclado. Desde o fechamento do F9: o grafo ilegível (todos os itens terminam, a falha dita na cidade, na nota da run e na Run, nada do grafo anterior na tela), cada leitura atrasada ou com falha sem segurar as outras, Juntar a declaração de funções Python decoradas (dados reais e fatos sintéticos sobre declarações reais, conferidos no texto da fonte), sem correspondência, ambígua (caso sintético declarado) e de fato desatualizado, Juntar durante uma investigação e num conflito entre abas, os campos do perfil fechados durante a abertura de outro projeto, Cancelar o T25 devolvendo o caminho, reabrir o mesmo projeto sem perda, a linha de base com a versão do comando trocada, os destinos Run e Contagens manuais, os critérios do rascunho separados das alterações não salvas, Gerar guardando a run e os itens vazios. Os `PUT /api/project` que ela precisa ver são respondidos por ela; nada chega ao servidor |
| `tela-revisao.js` | 158 | F10 do novo frontend, em três partes separadas. **Revisão individual** (4.1 a 4.3): Preparar pelo Builder cria a pergunta e não envia; "Ver revisão" só nas rotas que têm revisão (a escada diz "Sem revisão nesta rota"); os sete blocos com os rótulos obrigatórios, a query igual ao `turn.query`, "Não conferido nesta revisão" sempre presente; só o clique em Enviar envia (um `POST /api/send`, com as chaves de sempre), com o `evidence_sha256` do vínculo ao lado do hash visto; H2 recusada desabilita Enviar com o texto do servidor; fato do pacote revogado: aviso e recomendação sem bloqueio, os fatos relidos antes do envio, o clique que encontra uma revogação nova só a mostra; falhas de leitura, carregando, Nenhum, falha do envio, respostas atrasadas (outra Revisão aberta, troca de run), Voltar ao rascunho, a Revisão guardada e refeita ao recarregar (e não aberta em outra run). **Lote** (4.5): Separar e Conferir fatos criam uma pergunta por linha, uma fonte cada, sem enviar; criação com falha e H3 recusada (desmarcada com o motivo); a seleção conferível; "Enviar as marcadas" envia uma por vez, cada uma na própria pergunta, com "k de N"; a falha de uma não para as outras nem aparece nelas; as desmarcadas não saem; troca de run durante o envio e a criação; nenhum `POST /api/review`. **"Aprovar todos"** (depois das respostas): a prévia com os FACTs aprováveis e os não aprováveis com o motivo (já decidido, LACUNA, INFERENCE, HYPOTHESIS, conferência mecânica, sem vínculo, sem envio); cancelar e nota vazia não decidem; só os aprováveis, um pedido cada; o resultado por item, com falha parcial; item decidido entre a prévia e a confirmação; nenhum `POST /api/send`. Também EN, tema escuro, tela estreita e teclado. Casos simulados (H recusada, fato revogado, perguntas e envios criados pela bateria) são respostas da própria bateria; nada chega ao servidor |

Nada é enviado a um modelo: as baterias abortam `/api/send`, as execuções e `/api/generate`, e simulam ou
abortam `/api/ladder`. Só `tela-passo6.js` executa comandos, fora da página e pela API: `git grep` nos projetos que cria.
Duas gravam o perfil da cópia e o devolvem como estava: `tela-perfil.js` (um caso, restaurado em bytes) e
`etapa2c.js` (confirma a mudança da política pedida ao preparar depois de Otimizar e a devolve a Localizar pelo seletor).

## O que precisa existir

| Variável | O que é |
|---|---|
| `LIBRARIAN_TESTE_STUDIO` | Executável `librarian_studio`, compilado do código a testar |
| `LIBRARIAN_TESTE_RAIZ_RS` | Cópia de uma raiz do Librarian, com `.studio/projects/renderer-librarian-*` e as rodadas `20261104-014710` e `20261104-020244` |
| `LIBRARIAN_TESTE_RAIZ_PY` | Cópia de uma raiz, com o projeto Python `backend-*` e a rodada `20261029-053623`, sem medição |
| `LIBRARIAN_TESTE_COPIAS` | Pasta para as cópias de execução |
| `LIBRARIAN_TESTE_RENDERER` | Caminho do projeto `renderer`, a que as rodadas se referem |
| `LIBRARIAN_TESTE_PYTHON` | Caminho do projeto Python, a que a rodada se refere |
| `LIBRARIAN_TESTE_SAIDA` | Pasta para capturas de tela e o resultado do `etapa2c.js` |
| `LIBRARIAN_TESTE_PLAYWRIGHT` | Opcional: caminho do `playwright-core`, se ele não estiver no `NODE_PATH` |
| `LIBRARIAN_TESTE_CASOS` | Opcional: `casos.json` do gabarito; o padrão é `.studio/gabarito/casos.json` neste repositório, pasta que também fica fora do Git |
| `LIBRARIAN_TESTE_CHAVE_RENDERER`, `LIBRARIAN_TESTE_CHAVE_PYTHON` | Opcionais: chaves dos projetos, se os caminhos não forem os originais |

Também é preciso Node 18 ou mais novo, Google Chrome e as portas 8841 e 8842 livres. Com isso:

```
node crates/librarian-studio/tests/tela/rodar.js
```

A execução completa leva mais de 10 minutos.

Para rodar uma só, suba os servidores nas duas portas e chame `node tela-zero.js <pasta de saída>` com as
mesmas variáveis. `etapa2c.js` pede também o caminho do projeto Python: `node etapa2c.js <saída> <python>`.

**Limites que valem para quem for rodar:**

- **O código não tem caminho fixo, mas os dados têm.** Os resultados esperados (`6.410.258`, 38 e 125 no
  zero, 12 candidatos de `default`) são dessas rodadas do `renderer`, que não estão no Git. O conjunto usado
  aqui ocupa cerca de 1,2 GB.
- **A chave do projeto é o nome mais o hash do caminho.** Em outro caminho, o Studio procura outra pasta em
  `.studio/projects`. As variáveis de chave existem para isso, mas rodar assim, ou em outra máquina, **nunca
  foi testado**.
- **As baterias gravam na raiz:** `etapa2c.js` cria turnos. `rodar.js` recusa o próprio repositório como raiz
  e exige que a página da raiz seja igual à do repositório.

## Origem

`tela-4a.js`, `tela-4b.js` e `tela-passo6.js` nasceram aqui. As outras são cópias das baterias usadas na auditoria dos Passos 1 e 2, com as verificações intactas. Só mudaram as linhas
de ambiente: o `require` do Playwright, os caminhos do `renderer`, do Python e do `casos.json` e as chaves.
Para conferir, compare com o original.

| Bateria | Sessão de origem | SHA-256 do original |
|---|---|---|
| `tela-cartao.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `51862c718f93167c0f544341a1aac7b69e4b238915f7a0d174a1de2b53fd260f` |
| `tela-zero.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `cc476a459d5a01ca1fd7518c0237bd3ddabf8351df77b0e2cec83f6138847c20` |
| `tela-origem.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `a284f8feec33c18c7f3fc3f5b8479baeeba40c619e82b59f0d94270d5f721aa8` |
| `tela-nomes.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `5eab6aeaf75a370325c4af5a7efe2b4e9b9f6a8440803388e83227a4d589f2b2` |
| `tela-passo2.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `2da96f5c141c7501c4b7ce3e26ae4957fed363fab887f800661ef8dac99cfda2` |
| `etapa2c.js` | `465f4d1b-0d17-4dff-903f-d3077bb3c8f4` | `39ee0a736e32e7cabce2cc34948c0df9d4b708614d332eac31c07d119c51ac07` |

`etapa2.js` e `etapa2b.js` não estão aqui: seguem no scratchpad da sessão `465f4d1b` e, na mesma versão, no
arquivo de preservação, descrito no [plano](../../../../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md). O `etapa2c.js`
desse arquivo é anterior ao daqui: a busca do par de funções foi ajustada depois.
