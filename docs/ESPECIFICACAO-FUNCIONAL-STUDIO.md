# Especificação funcional do novo Librarian Studio

**Situação:** especificação aprovada em 11/11/2026 como referência para a implementação futura do frontend. **Não descreve uma implementação concluída.** Nada do que está marcado como `[P]` existe hoje no Studio. Cada passo de implementação (F0 a F13, B1 a B7) depende de autorização própria.

**Base:** `main` no commit `9bc92b00cfc05f5cf46b491a0e1ee110e43d378a`. Esta especificação consolida a jornada do usuário (Etapa 1), a organização das áreas (Etapa 2) e a especificação funcional (Etapa 3) da fase de reformulação do frontend, com as decisões aprovadas sobre as questões I-1 a I-8 e Q1 a Q12 e a correção da regra de Salvar e Trocar modo (seção 3.13).

**Documentos relacionados:** o funcionamento atual está no [STUDIO.md](../STUDIO.md); os detalhes técnicos do servidor, no [README do Studio](../crates/librarian-studio/README.md); as etapas do Evidence Builder (incluindo as Etapas 5, 6 e 7 citadas aqui), no [plano](../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md#evidence-builder-no-studio-etapas-0-a-7-05112026).

## Marcas

| Marca | Significado |
|---|---|
| **[C]** | Contrato ou comportamento existente na `main` |
| **[A]** | Decisão aprovada |
| **[P]** | Comportamento novo especificado e aprovado como referência, **ainda não implementado** |
| **[D#]** | Depende de mudança de backend (seção 8.2) |
| **[L]** | Limitação aceita na primeira versão |
| **[Validar]** | Suposição a confirmar por teste no passo correspondente |

## Conferências de código que sustentam esta especificação

| Ponto | Onde | O que mostra |
|---|---|---|
| Rotas | [main.rs:109-276](../crates/librarian-studio/src/main.rs#L109-L276) | Lista completa usada na seção 8 |
| Escada | [main.rs:338](../crates/librarian-studio/src/main.rs#L338) | **Não aceita `turn`**: cria a própria pergunta, roda as linhas de base e envia na mesma chamada. O C1 não existe |
| Envio | [main.rs:306](../crates/librarian-studio/src/main.rs#L306), [store.rs:1259](../crates/librarian-studio/src/store.rs#L1259) | Refaz H1 a H3 e grava `evidence_sha256` no vínculo **depois**. Não recebe do cliente um hash revisado. Modelfile, skills e modelo vêm no corpo do envio |
| Skills no envio | [llm.rs:186](../crates/librarian-studio/src/llm.rs#L186) | Nome e hash ficam no `send.json`, gravados no envio. Não estão no pacote |
| H1 e H2 | [store.rs:1335](../crates/librarian-studio/src/store.rs#L1335) | Não conferem revogação de fatos |
| Seleção | [store.rs:511](../crates/librarian-studio/src/store.rs#L511) | Só a origem `selection` ou `ladder` traz seleção |
| Tipo de origem no pacote | [evidence.rs:181](../crates/librarian-ingest/src/evidence.rs#L181) | Enum fechado: `selection`, `ladder`, `hot`, `loop`, `operation` |
| Sugestão | [store.rs:604](../crates/librarian-studio/src/store.rs#L604) | Falha de leitura da medição vira "sem medição" na resposta; só o registro gravado distingue `Unreadable` |
| Língua do léxico | [main.rs:258](../crates/librarian-studio/src/main.rs#L258) | O campo `language` já vem do cliente; a dedução pelo `-en` é da página |
| Cartão de desempenho | [questions.rs:315](../crates/librarian-studio/src/questions.rs#L315) | `/api/performance-question` não grava nada |
| Grafo v2 | `librarian-ingest/src/graph/v2.rs` | Traz `catalog_sha256`, usado como identidade local da edition |
| Configurações | [store.rs:227](../crates/librarian-studio/src/store.rs#L227) | `PUT /api/project` grava o objeto `settings` **inteiro**, sem versão nem comparação |
| Política e critérios | [store.rs:364](../crates/librarian-studio/src/store.rs#L364), [store.rs:987-1015](../crates/librarian-studio/src/store.rs#L987-L1015) | Todas as rotas que criam pergunta leem as configurações **gravadas**. Nenhuma aceita política ou critérios no corpo. A política usada fica gravada com a pergunta ([store.rs:1582](../crates/librarian-studio/src/store.rs#L1582), [claims.rs:212](../crates/librarian-studio/src/claims.rs#L212)) |
| Diff de uma mudança | [execution.rs:429](../crates/librarian-studio/src/execution.rs#L429) | A execução aplica `check.json → diffs[claim]`, o mesmo texto que a página recebe em `check.diffs[claim]` |
| Modo na página | [index.html:2148-2168](../crates/librarian-studio/web/index.html#L2148-L2168), [index.html:1959](../crates/librarian-studio/web/index.html#L1959), [index.html:1996-2000](../crates/librarian-studio/web/index.html#L1996-L2000) | O modo é uma preferência **global** do navegador (`librarian.mode`) e sobrescreve na tela a política do projeto (seção 3.13) |
| Ordem de decisão do botão | [index.html:2255](../crates/librarian-studio/web/index.html#L2255) | `ask()`; `askModel` grava as configurações antes de perguntar |

## Decisões aprovadas aplicadas

1. Três colunas. Esquerda: Projeto, Explorar, Régua, Acervo e as ferramentas Léxico, Configurações, Modelfiles e Abrir arquivos. Centro: a cidade como base, com Fluxograma, Builder, Revisão e Comparação por cima. Direita: linha do tempo, pergunta ativa e compositor.
2. O Léxico fica nas ferramentas, mas fácil de acessar.
3. O rascunho fica no navegador, separado por projeto e run, e nunca é transferido para outra run.
4. A pergunta preparada e não enviada continua registrada e não é apagada automaticamente.
5. Lotes são conjuntos de perguntas independentes e nunca formam um pacote único.
6. O Builder é o destino único de montagem. O compositor cuida da pergunta, do modo, dos critérios e do modelo. A conversa não monta fontes.
7. **Duas operações explícitas e distintas gravam o perfil:** Salvar as configurações editáveis, e Alterar a política associada ao modo. A política gravada no perfil é a fonte de verdade do modo (seção 3.13).
8. Continuam valendo as decisões anteriores do plano: qwen para fatos e Haiku para mudanças, sem cascata e não configuráveis na tela; revisão do pacote antes do envio (Etapa 5); propostas lado a lado (Etapa 6), com `passed` como "Maior %, critério atual", sem dominância nem Pareto; H1 a H3 como portão e H4 informativo; "conferido" nunca é "aprovado"; a cidade continua central; fluxograma, modo arquivo, aba Léxico, `/api/ask`, `askPath`, colar resposta e cartão de desempenho permanecem; o Studio não aplica mudanças no projeto.

A Etapa 7 do plano é tratada como critério de aceitação da implementação (seção 9), não como etapa da jornada.

---

## 1. Estrutura geral

### 1.1 Três colunas [A]

| Coluna | Papel | Conteúdo |
|---|---|---|
| Esquerda | Navegação | Cabeçalho, quatro áreas, quatro ferramentas e o painel ativo |
| Centro | Espaço de trabalho | Cidade (base), barra de vistas, vista ativa |
| Direita | A pergunta | Linha do tempo com a pergunta ativa e, embaixo, o compositor |

**Tela estreita [P]:** uma coluna por vez, com um alternador de três posições: Navegação, Centro e Pergunta.

- Cada posição mostra um resumo de estado, como "Centro: Builder (3)".
- Trocar de posição não descarta nada.
- Quando uma ação leva a outra coluna, a posição muda e a troca é anunciada.

### 1.2 Cabeçalho (sempre visível)

| Elemento | Conteúdo | Ação | Fonte |
|---|---|---|---|
| Projeto | Nome, com o caminho completo no título | Vai à área Projeto | `POST /api/project` [C] |
| Run | Seletor: id, git curto, "com alterações" se `git.dirty` | Troca de run (T14) | `runs` [C] |
| Estado da run | "Run com alterações não commitadas: comandos da Régua não rodam nela" | Vai à Régua | `git.dirty` [C]; o aviso antecipado é [P] |
| Rascunho | "Rascunho: N fontes · guardado neste navegador" ou "· só nesta página" | Abre o Builder | [P] |
| Não salvo | "Alterações não salvas: Configurações, Léxico, critérios padrão" | Vai à ferramenta | [P] |
| Voltar | Volta ao ponto anterior da navegação (5.3) | — | [P] |
| Idioma e tema | PT/EN; claro, escuro ou sistema | — | [C] |

### 1.3 Navegação esquerda [A]

```
Áreas         Projeto · Explorar · Régua · Acervo
Ferramentas   Léxico · Configurações · Modelfiles · Abrir arquivos   (grupo sempre visível)
```

- A coluna mostra um painel por vez, e o item ativo tem `aria-current="page"`.
- **O Léxico fica fácil de achar:**
  - o item fica sempre visível e mostra o estado ("vazio" ou o número de pedidos pendentes);
  - há atalho no resumo de estado ("Léxico vazio");
  - há atalho numa pergunta livre cujos termos não acharam nada.
- **A preferência antiga `tab` é convertida uma vez [P]:**

| `tab` antigo | Destino |
|---|---|
| `project` | Projeto |
| `lexicon` | Léxico |
| `inventory` | Explorar › Onde pesa |
| `ruler` | Régua |
| `facts` | Acervo |
| `map` | Explorar › Estrutura |

### 1.4 Centro: base e vistas [A]

A barra de vistas está sempre presente. Mostra "Cidade" e as vistas abertas, cada uma com título, estado e botão de fechar (menos a Cidade), e se percorre pelo teclado.

| Vista | Instâncias | Abrir com outro conteúdo | Fechar |
|---|---|---|---|
| Cidade | Base | — | Não fecha |
| Fluxograma | 1 | Substitui. Pede confirmação se há escolhas de homônimos | Descarta semente e escolhas, com confirmação se houver escolhas |
| Builder | 1, o rascunho da run ativa | Só uma entrada que recomeça substitui, com confirmação (T6, T8) | **Só esconde.** "Limpar rascunho" é ação à parte |
| Revisão | 1 (uma pergunta ou um lote) | Substitui, sem perda: é registro | Só fecha |
| Comparação | 1 | Substitui | Só fecha |

**Regras do centro:**

- A câmera só muda por pedido explícito: "voar até", Enquadrar, ou clique numa pergunta. Restaurar não voa [C].
- Ir para "Cidade" esconde a vista de cima, que continua aberta na barra.
- A seleção de nó ou aresta é única: vale na cidade, no fluxograma e no Builder.

### 1.5 Direita [A]

A linha do tempo e a pergunta ativa ficam em cima. O compositor fica embaixo e mostra **uma linha** sobre o Builder, nunca a lista de fontes.

### 1.6 Ponto de partida [A, I-7]

Sem estado guardado, o Studio abre assim:

- área Projeto, com o resumo de estado;
- Cidade no centro.

Com estado guardado, restaura o que dá para restaurar (T17).

---

## 2. Especificação de cada área

### 2.1 Projeto

| Ação | Rota | Regras |
|---|---|---|
| Abrir | `POST /api/project {root}` → `{key, settings, runs}` [C] | Restaura a run guardada para o projeto, se ela ainda existir; senão, a mais recente [P]. **Sincroniza o modo com a política do projeto (3.13)** [P]. Sem runs: "Sem run: gere a primeira" |
| Procurar | `GET /api/browse` [C] | Diálogo atual |
| Recomeçar | — [C] | Limpa a página e esquece o último projeto no navegador. **Não apaga rascunhos guardados** [P] |
| Gerar | `POST /api/generate {key}` [C] | **Não aciona Salvar** [P] (hoje aciona). Com alterações não salvas, pergunta: "Gerar com as configurações gravadas", "Salvar e gerar" (Salvar explícito, com a lista dos campos) ou "Cancelar". A nova run vira a ativa (T14) |
| Entradas | Campo de `settings` | Editar marca "não salvo". As Entradas são gravadas por Salvar (2.10, 3.13) |

**Resumo de estado [P].** Cada item carrega à parte, e uma falha num item não afeta os outros.

| Item | Fonte [C] | Estados escritos |
|---|---|---|
| Medição | `GET /api/commands` (`measure_all`) e `GET /api/hotspots` (404 = sem medição) | Disponível (data) · Pode medir · Falta aprovar o comando (→ Régua) · Não disponível: {motivo} · Falha de leitura |
| Comandos | `GET /api/commands` | "N de M aprovados na versão atual" |
| Contagens manuais | `manual_counts` (`null` = ilegível) | Falha de leitura, que é diferente de "nenhuma" |
| Léxico | `settings.lexicon` | Vazio (→ Léxico) · N grupos, M stop words |
| Política | `settings.allow_hypothesis`, `allow_patch` | "Modo do projeto: Localizar" · "Melhorar" · "Política personalizada" (3.13) |
| Fatos | `GET /api/facts` | N vigentes · M desatualizados · K revogados · J aplicados |
| Perguntas | `GET /api/turns` | N preparadas e não enviadas · K com decisões pendentes (→ a primeira) |
| Run | Registro da run | Fontes, símbolos, diagnósticos, git, com alterações |

**Estados:** sem projeto; carregando; erro com a mensagem do servidor como veio; sucesso.

**Transições:** cada item do resumo leva ao destino correspondente e grava o ponto de partida no Voltar.

### 2.2 Explorar

Três vistas internas: **Estrutura**, **Onde pesa** e **Inventário**. A vista escolhida fica guardada no navegador [P].

#### Estrutura

| Elemento | Comportamento |
|---|---|
| Encontrar | Busca no grafo, no cliente; resultados; Enquadrar; Limpar [C] |
| Busca da pergunta ativa (`#overlayCard`) | Aparece quando a pergunta ativa tem busca: acertos e realce na cidade, que se pode desligar [C] |
| Inspeção | Nó: categoria, kind, linguagem, id, local, relações com evidência. Aresta: arquivo, linhas, bytes, hashes [C] |
| Ações da inspeção | "Onde é usado?" e "O que ele chama?" (→ Fluxograma); "Juntar ao rascunho" (→ Builder, soma); "Voar até"; as demais ações atuais ficam [C] |

**Inspeção como estado da área [A, I-1]:**

- Abrir uma inspeção a partir de uma lista guarda vista, rolagem, filtros e dobras, e mostra a inspeção sobre a lista.
- "← voltar à lista" restaura tudo isso.
- Inspecionar uma relação empilha a nova inspeção; "←" volta um nível.

**Regra da seleção [A, I-1]:**

| Onde a pessoa está | Clique num nó |
|---|---|
| Em Explorar | Inspeção sobre a lista atual; o ponto vai para o Voltar |
| Fora de Explorar | Seleção e realce atualizados, e a barra mostra "Selecionado: nome [Inspecionar]". **A área não muda.** Só "Inspecionar" leva a Explorar › Inspeção, gravando o ponto no Voltar |

#### Onde pesa

| Elemento | Comportamento | Rota [C] |
|---|---|---|
| Medir tudo | Instrumenta numa cópia e roda o comando aprovado | `POST /api/instrument/run` |
| Ranking | Ordem pela contagem; cobertura (4A); "como contou"; operações estruturais; top 8 da cidade por `cpu_ms`; "sem passagens observadas" não quer dizer "não rodou" [A] | `GET /api/hotspots`, `GET /api/measure-detail` |
| Por linha | **Inspecionar** (no lugar), **Otimizar** (T6), **Separar** (T7) | — |

**Estados de Onde pesa:**

| Estado | Mensagem |
|---|---|
| Não disponível | "Não disponível: {motivo}" |
| Sem medição | "Ainda não há medição nesta edition" |
| Sem comando aprovado | "Falta aprovar o comando (→ Régua)" |
| Em andamento | "Medindo…" (sem cancelamento [C]) |
| Erro | Mensagem do servidor |
| Sucesso | Data e comando da medição |

#### Inventário

| Elemento | Comportamento | Rota [C] |
|---|---|---|
| Montar, filtro, busca, termos | Como hoje | `GET /api/inventory` |
| Achados | Etiquetas `at=`; anel vermelho na cidade | — |
| Investigar achado | **Fluxo atual de investigação, mantido** [A, I-6] (2.4.2, T8) | `POST /api/inventory/investigate` |

**Estados:** não montado; montando; vazio ("Nenhum achado pelas regras atuais"); erro.

**Preservado em Explorar:** vista interna, rolagem por vista, filtros, busca, dobras e pilha de inspeção. Ficam na memória; só a vista interna vai para o navegador.

### 2.3 Fluxograma (vista central)

| Aspecto | Especificação |
|---|---|
| Entrada | Pela inspeção: `GET /api/used-by` ou `POST /api/calls` [C] |
| Controles | Sem testes; só o que rodou; traço sólido; profundidade; refazer escolhas; fechar [C] |
| Perguntas por ponto | `askPath` (`POST /api/path-questions`), **uma pergunta por ponto, sem revisão** [A]. Usa o modo Localizar: se a política do projeto for outra, pede confirmação para alterá-la (3.13, T26) |
| Juntar | "Juntar ao rascunho" num ponto **soma uma fonte** ao Builder, sem trocar de vista [P, Q5] |
| Vários pontos numa pergunta | **Fora da v1** [A, Q5] |
| Clique num ponto | Regra da seleção (2.2) e detalhe do ponto [C] |
| Voltar ao fluxograma | Pela barra de vistas: semente, modo, profundidade, opções e escolhas preservadas enquanto aberto |
| Fechar e recarregar | Descartam o fluxograma (fechar pede confirmação se houver escolhas) |
| Estados | Carregando; truncado ("mostrando 120 nós; há mais") [C]; vazio; erro |
| Guarda de run | Resposta que chega depois de trocar de run é ignorada (`sameRun`) [P] |

### 2.4 Evidence Builder (vista central)

Os detalhes de comportamento estão na seção 3. Aqui ficam o papel e a relação com as áreas.

- É o destino único de montagem das fontes de **uma** pergunta [A].
- Há um rascunho por par projeto e run; só aparece o da run ativa [A, I-2].
- Duas exceções, com regras próprias: o **achado do inventário** (2.4.2, até o D2) e os **lotes** (4.5), que não entram no Builder.

#### 2.4.1 Entradas no Builder

| Entrada | Efeito | Corpo enviado depois |
|---|---|---|
| Juntar (inspeção ou ponto do fluxograma) | Soma | Inalterado [C] |
| Sugerir | Lista a sugestão | `POST /api/envelope/suggest` [C] |
| Otimizar (Onde pesa) | Rascunho novo: alvo, sugestão, texto pré-escrito, escada marcada, **modo necessário Melhorar** (5.4) | Inalterado [C] |
| Pedir esta mudança (cartão) | Rascunho novo, como Otimizar [P] (hoje envia direto) | Sem edição da pessoa: igual ao de hoje |
| Fatos do Acervo | Soma a declaração do fato (manual) [P] | Inalterado |

Nenhuma entrada grava o perfil. Hoje Otimizar e o cartão trocam o modo da página; na especificação, só registram o modo necessário no rascunho, e a política só muda pela operação explícita de 3.13.

#### 2.4.2 Achado do inventário até o D2 [A, I-6]

- Mantém o fluxo atual: `investigationView`, `investigation.ticked`, origem `{inventory, kind, summary}` **sem** seleção [C].
- **A v1 não remove esse fluxo.** Ele sai do compositor e passa a aparecer na vista do Builder, em **modo achado**, com o rótulo "Achado do inventário: itens marcados, sem registro de seleção no servidor".
- **Nunca é convertido em seleção manual.**
- O corpo de `/api/gathered-question` continua igual ao de hoje.
- Usa o modo Localizar: modo necessário registrado no rascunho (5.4).
- A substituição definitiva depende do D2 e de validação própria.

### 2.5 Régua

| Elemento | Comportamento | Rota [C] |
|---|---|---|
| Comandos | Nome, versão (hash), "aprovado nesta versão" ou "aprovação de outra versão" | `GET /api/commands` |
| Aprovar | A versão exata | `POST /api/commands/approve {name, sha256}` |
| Rodar | Linha de base, sem `change` | `POST /api/commands/run` |
| Linha de base em vigor | A última com sucesso, sem `change` e sem instrumentação (o critério da escada) | `GET /api/executions` |
| Histórico | Com `change`: "Abrir a pergunta #N" (T9). Sem: "linha de base". Medição: "→ Onde pesa" [P] | idem |
| Run com alterações | Rodar desabilitado **antes** do clique, com o motivo [P]. O servidor recusa [C] | — |

**Estados:** sem comandos no perfil; não aprovado; rodando; falhou (com a saída); concluído.

**Papel [A, Q10]:** visão geral de comandos, linha de base e execuções da edition, sempre com volta à pergunta de origem.

### 2.6 Acervo

| Elemento | Comportamento | Rota [C] |
|---|---|---|
| Lista | Fatos reconferidos contra a run ativa: **vigente, desatualizado, revogado, aplicado**, com o motivo | `GET /api/facts` |
| Filtro por estado | [P] | — |
| Ir à origem | Mesma run: ativa a pergunta e rola até o envio e o claim. Outra run: T13 | — |
| Revogar | Abre a origem no controle de decisão: um lugar só para decidir [P] | `POST /api/review` (`rejected`) |
| Juntar a declaração ao rascunho | Soma ao Builder [P] | — |

**Estados:** vazio ("Nenhum fato aprovado neste projeto"); carregando; falha de leitura.

**Papel [A, Q10]:** visão dos fatos aprovados, com volta à pergunta e à resposta de origem.

### 2.7 Comparação (vista central)

As regras estão em 4.6.

### 2.8 Revisão (vista central)

As regras estão em 4.1 a 4.5.

### 2.9 Conversa (coluna direita)

As regras estão em 4.7.

### 2.10 Ferramentas

**Léxico** [C, mantido como capacidade]:

- Editar o texto do léxico, que fica "não salvo" até Salvar.
- Sugerir (`POST /api/lexicon/suggest`). **Não aciona Salvar** [P] (hoje aciona). A sugestão usa o léxico gravado; com edições não salvas, a tela avisa "A sugestão usa o léxico gravado, não as edições não salvas".
- Caminho manual: copiar o pedido, colar a resposta e Conferir (`POST /api/lexicon/check`).
- Proposta com cada grupo e stop word conferidos.
- **Aplicar** [P]: leva a proposta ao texto do léxico e abre Salvar (3.13) com a lista dos campos alterados. Confirmar grava; cancelar deixa o texto como "não salvo". Hoje Aplicar grava o perfil inteiro de imediato [C], inclusive outras edições não salvas.
- **Descartar** remove a proposta.
- **Língua explícita [P, D8]:** seletor PT/EN, com padrão deduzido do `-en` como hoje, enviado no campo `language`, que já existe.

**Configurações** [C]:

- O conteúdo atual de `#settingsBox`, **incluindo os critérios padrão**. `allow_hypothesis` e `allow_patch` não são editados aqui: pertencem à operação de alterar a política (3.13).
- **Salvar** grava as configurações editáveis (`PUT /api/project`, objeto inteiro). Antes de gravar, mostra a lista dos campos alterados [P]. As regras completas estão em 3.13.
- **Exportar perfil:** ensaio, confirmação e gravação. É a única escrita no projeto analisado [C]. Exporta o perfil **gravado**. **Não aciona Salvar** [P] (hoje aciona): com alterações não salvas, pergunta "Exportar o gravado", "Salvar e exportar" ou "Cancelar".

**Modelfiles** [C]: o diálogo atual, aberto pelas Ferramentas e pelo compositor. Gravar um Modelfile não toca o perfil.

**Abrir arquivos** [C]: modo arquivo. Sem servidor, é a única entrada, e as áreas que precisam do servidor mostram "Indisponível sem o servidor do Studio".

---

## 3. Comportamentos do Builder e do compositor

### 3.1 Listas do Builder [C], formato 2 da seleção

| Lista | O que é | Vai ao modelo |
|---|---|---|
| Incluídas (`included`) | O envio | Sim [A] |
| Removidas (`removed`) | Tiradas pela pessoa | Não |
| Deixadas de fora (`left_out`) | Fora pela sugestão, com `left_out_why` | Não |
| Não juntadas (`not_joined`) | Sugeridas e não trazidas | Não |

O contrato está em [Registro da seleção](../STUDIO.md#registro-da-seleção-originselection-formato-2).

### 3.2 Por fonte

| Campo | Origem |
|---|---|
| Nome, tipo, arquivo:linhas, bytes; papel; motivo | Sugestão e edition [C] |
| Cobertura: coberta, sem cobertura, sem medição | `annotate_suggestion` [C] |
| Falha de leitura da medição | [D9]. Na v1 aparece como "sem medição" [L] |
| Fatos vigentes ligados | Sugestão [C] |
| Proveniência gravada: manual, sugerida, de fora | `provenance` [C] |
| **Chegou por** | Só na tela e no rascunho local [P]. Gravar depende do D6 [L] |
| Alvo | Estado da tela [C] |
| Estado da informação | 7.1 [A] |

### 3.3 "Só você vê" [A]

Lacunas e candidatos; ambiguidades sem resolução automática; comportamento; categorias; observações da sugestão, com o rótulo "não vão ao modelo". `notes` nunca entra na origem.

### 3.4 Orçamento

Bytes das incluídas contra o orçamento, e itens contra o máximo [C]. A tela mostra a conta e não decide. [Validar] se o servidor recusa acima do orçamento.

### 3.5 Ações

| Ação | Efeito |
|---|---|
| Sugerir | Lista a sugestão; acrescenta o hash em `suggestions` |
| Juntar as sugeridas | Sugeridas → incluídas |
| Incluir | De fora ou não juntada → incluídas |
| Remover (×) | → removidas |
| Marcar ou desmarcar alvo | Fora de `decision` [A] |
| Levar fatos aprovados | `with_facts` |
| Limpar rascunho | Esvazia, com confirmação |
| Reordenar à mão | **Fora da v1** [A, I-8]. "Reorganizar" é só mover entre listas |

- `decision` e `decision_scope` são atualizados como hoje, sem histórico [A].
- Cada mudança grava o rascunho (5.4).

### 3.6 Pedidos concorrentes [P, só frontend]

- Cada pedido de sugestão ou investigação leva sequência e run.
- Resposta antiga ou de outra run é ignorada, com o aviso "Uma sugestão anterior foi descartada: outra foi pedida depois".
- Enquanto uma entrada que recomeça está carregando, "Juntar" fica desabilitado, com o motivo escrito.

### 3.7 Substituir um rascunho não vazio [A]

Diálogo: "Substituir o rascunho atual? N fontes, alvo X, texto '…'", com "Substituir" e "Cancelar". Rascunho vazio não pede confirmação.

### 3.8 Corpo enviado ao servidor [A]

Para as mesmas ações, o corpo de `/api/gathered-question` e de `/api/ladder` é **byte a byte o de hoje**: `nodes` = incluídas, `origin = {kind, summary, selection}`. Só muda onde a tela mostra.

### 3.9 Estados do Builder

| Estado | Mensagem |
|---|---|
| Vazio | "Junte uma fonte pela inspeção ou use Otimizar" |
| Pedindo sugestão | Em andamento |
| Recusa | "Seleção recusada ({motivo}); nenhuma pergunta foi criada" [C] |
| Restaurado | "Rascunho restaurado deste navegador ({data})" |
| Incompatível | 5.5 |
| Alterado em outra aba | 5.7 |
| Já preparado | "Já preparado como #N, sem mudanças desde então" |
| Alterado depois de preparar | "Alterado depois de preparar #N" |

### 3.10 Compositor: campos

| Campo | Comportamento |
|---|---|
| Texto | Fica no rascunho. **Não é apagado ao preparar** [P]; hoje é apagado |
| Localizar ou Melhorar | Mostra a **política gravada no projeto** (fonte de verdade). Trocar é a operação explícita "Alterar a política do modo" (3.13): grava só `allow_hypothesis` e `allow_patch`, e o seletor só muda depois do sucesso do servidor [P]. Sai do perfil com o D1 |
| Critérios | 3.12 |
| Escada | Visível em Melhorar com fontes. Haiku fixo, sem cascata [A] |
| Modelo | **Fixo e só mostrado:** qwen nos lotes, Haiku na escada [A, Q8]. **Escolha livre** só onde já existe hoje (pergunta livre, Builder sem escada, cadeia, reenvio, léxico), com as opções atuais, incluindo "Nenhum" [C]. A escolha livre não se estende a fluxos fixos. A escolha lembrada por tipo de ação (`llm.locate`, `llm.improve` e outras) continua [C] |
| SYSTEM e skills | Modelfile, resumo e skills escolhidos [C]. O texto vai no envio e o `send.json` grava nomes e hashes [C]. **Não fazem parte do pacote** [D4] |
| Acesso ao Builder | "Rascunho: N fontes · alvo X · [Abrir no Builder]" |

### 3.11 Caminho previsto [P]

Segue **a mesma ordem de decisão de `ask()`** [C, [index.html:2255](../crates/librarian-studio/web/index.html#L2255)]:

| # | Condição | Mostra | Botão | Revisão na v1 |
|---|---|---|---|---|
| 1 | Fontes e nenhum alvo | "Escolha um alvo no Builder" | Desabilitado | — |
| 2 | Melhorar, escada ligada, fontes | "Escada: Haiku; portão: {comandos}". Sem comando aprovado: "Sem comando aprovado no portão (→ Régua)", e o botão fica desabilitado | "Rodar a escada" | **Não**: "Esta rota envia sem revisão (depende do C1)" [L, D3] |
| 3 | Achado do inventário | "Achado do inventário: N itens marcados, sem registro de seleção" | "Preparar" | Parcial (4.1) |
| 4 | Fontes no Builder | "Builder: N fontes" | "Preparar" | Parcial (4.1) |
| 5 | Sem fontes | "Primeiro as regras de desempenho. Se não reconhecerem, pergunta livre com busca" | "Perguntar" | Não [A] |

- No caso 5, `/api/performance-question` é só leitura ([Validar] por teste). Se a pergunta é reconhecida, aparece o cartão "sem modelo" e nenhuma pergunta é criada [C].
- **Modo necessário diferente da política:** se o rascunho registra um modo necessário (5.4) diferente da política gravada, o caminho mostra "Esta pergunta usa o modo {X}; a política do projeto está em {Y}", e o botão de ação pede confirmação para alterar a política antes de continuar (T26).
- Nenhuma rota do compositor aciona Salvar [P] (hoje `askModel` e a escada acionam).

### 3.12 Critérios [A, Q9, com a incompatibilidade registrada]

| Conceito | Onde fica | Como muda |
|---|---|---|
| **Critérios padrão** | Perfil (`settings.criteria`) [C] | **Só por Salvar**, em Configurações ou no compositor |
| **Critérios do rascunho** | Rascunho local (5.4), quando diferem do padrão | Editados no compositor. Mostram "Diferem do padrão do projeto (não salvos)" |
| **Critérios usados pela pergunta** | Cópia na pergunta (`criteria-before.md` mais hash) [C] | Gravados ao preparar. Nunca mudam depois; o envio recusa se o arquivo mudar [C] |

**Incompatibilidade.** O servidor só copia para a pergunta os critérios **gravados no perfil**, e nenhuma rota recebe critérios no corpo. Com o contrato atual não dá para preparar com critérios editados sem gravar o perfil. A dependência fica registrada como **D10**.

**Comportamento da v1 [P, L]:**

- Critérios do rascunho iguais ao padrão: preparar copia o padrão, como hoje.
- Critérios do rascunho diferentes do padrão: **Preparar fica bloqueado**, com a mensagem "Os critérios editados ainda não podem valer só para esta pergunta (depende do D10). Escolha:" e três botões:
  - "Salvar como padrão do projeto", ação explícita de Salvar, que mostra o que muda;
  - "Desfazer a edição";
  - "Cancelar".
- **Nunca há gravação implícita.** `askModel` deixa de acionar Salvar.
- Alterar a política do modo (3.13) **não grava** os critérios editados.
- **Alternativa rejeitada:** gravar, preparar e restaurar o perfil. Seria gravação implícita, sujeita a corrida entre abas.

Com o D10, Preparar envia os critérios do rascunho, o servidor os copia para a pergunta e o perfil não muda.

**Estados do compositor:** sem run (desabilitado); preparando; alterando a política; recusa do servidor, sem perder o rascunho; "Pergunta #N preparada".

### 3.13 Gravação do perfil: Salvar e Alterar a política do modo

#### 3.13.1 Comportamento atual, conferido no código [C]

| Ponto | O que acontece hoje |
|---|---|
| Onde o modo é guardado | Em `localStorage` (`librarian.mode`), **uma chave só para todos os projetos**. `setMode` grava a chave a cada troca ([index.html:2162](../crates/librarian-studio/web/index.html#L2162)) |
| Ao iniciar | `initStudio` aplica o modo da chave **antes** de abrir o projeto ([index.html:1959](../crates/librarian-studio/web/index.html#L1959)) |
| Ao abrir um projeto | `renderProject` põe `allow_hypothesis` e `allow_patch` do perfil nas caixas escondidas e, em seguida, `applyMode()` **as sobrescreve com o modo global** ([index.html:1996-2000](../crates/librarian-studio/web/index.html#L1996-L2000), [index.html:2148](../crates/librarian-studio/web/index.html#L2148)). A tela nunca mostra a política gravada |
| Ao trocar de modo | `setMode` só muda a tela e a chave global. **Não grava o servidor** |
| Quando a política é gravada | Na próxima chamada a `saveSettings()`, que manda o objeto inteiro com **todas** as edições da tela. Chamam `saveSettings()`: Salvar, Salvar léxico, Gerar, `askModel`, escada, cadeia, Separar, Conferir fatos, perguntas por caminho, Sugerir léxico, Aplicar léxico, Exportar perfil |
| Fluxos que forçam o modo | Separar, Conferir fatos, perguntas por caminho e o achado do inventário põem Localizar; Otimizar e "Pedir esta mudança" põem Melhorar; a cadeia põe o modo escolhido no diálogo. Os que gravam em seguida mudam a política do projeto como efeito colateral |
| Ao trocar de projeto | O modo global continua. Na próxima gravação, ele é escrito no outro projeto. **O modo de um projeto é reaproveitado silenciosamente em outro** |
| Ao trocar de run | Nada muda: a política é do projeto |
| Política combinada | `librarian.json` importado pode trazer `allow_hypothesis` diferente de `allow_patch` ([store.rs:1893](../crates/librarian-studio/src/store.rs#L1893)). O modo só representa os dois iguais, e a próxima gravação iguala os dois sem aviso |
| Política efetivamente usada | O servidor lê a política das configurações **gravadas** no momento de preparar ([store.rs:1013](../crates/librarian-studio/src/store.rs#L1013)) e a grava com a pergunta (`prepare-request`, contexto de claims). O envio usa a gravada com a pergunta ([claims.rs:212](../crates/librarian-studio/src/claims.rs#L212)). A tela pode mostrar Melhorar enquanto o perfil diz Localizar, se nada foi gravado desde a troca |
| Gravação | `PUT /api/project` grava o objeto inteiro, sem versão nem comparação ([store.rs:227](../crates/librarian-studio/src/store.rs#L227)): vale a última gravação |

#### 3.13.2 Regra aprovada: duas operações explícitas e distintas [A]

| | **Salvar as configurações editáveis** | **Alterar a política do modo** |
|---|---|---|
| Quem aciona | Só a pessoa, pelo botão Salvar (Configurações, Léxico, critérios padrão no compositor) ou por uma opção explícita "Salvar e …" | Só a pessoa: o seletor Localizar/Melhorar, ou a confirmação de uma ação que exige outro modo (T26) |
| O que grava | As configurações editáveis do perfil, com as edições da tela | **Só** `allow_hypothesis` e `allow_patch` |
| Antes de gravar | Mostra a lista dos campos que serão alterados | Mostra o modo novo e o que ele permite ("Melhorar: hipóteses e mudanças permitidas") |
| Base | O perfil lido do servidor **imediatamente antes**, mais as edições da tela | O perfil lido do servidor **imediatamente antes**, com só os dois campos trocados. **Nunca leva edições não salvas** |
| Depois | As edições deixam de ser "não salvas" | O seletor muda **só depois do sucesso**; mensagem "Política do projeto alterada: {modo}". Outras edições continuam "não salvas" |
| Falha | Nada gravado; as edições continuam na tela; mensagem do servidor | O seletor **continua no modo anterior**; mensagem do servidor |
| Nunca é acionada por | Gerar, Preparar, Perguntar, escada, cadeia, lotes, caminho, Sugerir léxico, Exportar, trocar de modo | Abrir projeto, trocar de run, restaurar rascunho, entradas no Builder |

Nenhuma outra operação grava o perfil. Exportar perfil escreve no projeto analisado, não no perfil do Studio (2.10).

#### 3.13.3 Sincronização do modo [P]

- **A política gravada no perfil do projeto é a fonte de verdade.**
- **Abrir ou trocar de projeto:** o seletor é sincronizado com a política daquele projeto (`allow_hypothesis` e `allow_patch` de `/api/project`). O modo de outro projeto nunca é aplicado.
- **Trocar de run no mesmo projeto:** a política do projeto continua valendo; nada é relido nem gravado.
- **Política personalizada** (os dois campos diferentes): o seletor não marca nenhum dos dois modos e mostra "Política personalizada: hipótese {sim|não}, patch {sim|não}". Escolher um modo é a operação explícita, que iguala os dois.
- **Entradas e fluxos que precisam de um modo** (Otimizar e o cartão: Melhorar; lotes, caminho e achado: Localizar; cadeia: o escolhido no diálogo) **não trocam o modo sozinhos.** Se a política é outra, a ação pede confirmação para alterá-la (T26). Cancelar não faz nada.
- **Antes de cada gravação** (das duas operações), a página relê `GET /api/project`. Se o perfil mudou desde a última leitura, mostra "O perfil foi alterado fora desta página" com os campos diferentes, antes de continuar.

#### 3.13.4 Preferência antiga `librarian.mode` [P]

- Hoje a chave é lida só ao iniciar e escrita a cada troca de modo. Ela decide o modo da tela e, por ele, qual escolha de LLM lembrada (`llm.locate` ou `llm.improve`) aparece.
- Na especificação, **a chave deixa de decidir o modo**: o seletor vem da política do projeto. A escolha de LLM lembrada passa a seguir o modo sincronizado.
- **Sem migração:** a chave não é convertida nem gravada no perfil, porque o valor é global e não diz a qual projeto pertence. Gravá-la reproduziria o defeito de 3.13.1.
- A chave fica onde está, sem ser apagada. Removê-la é uma decisão à parte.

#### 3.13.5 Limitação de concorrência [L, D11]

`PUT /api/project` grava o objeto inteiro e o servidor não confere versão. Mesmo relendo o perfil antes de gravar, há uma janela entre a leitura e a gravação: outra aba ou outra pessoa pode gravar nesse intervalo, e a última gravação apaga a anterior. A releitura reduz o risco, mas não o elimina. A eliminação depende de uma gravação parcial ou com versão no servidor (D11).

---

## 4. Revisão, envio e respostas

### 4.1 Revisão parcial na v1 [A, I-3]

**Abre** depois de Preparar nas rotas 3 e 4 de 3.11, nos lotes (4.5), ou por "Ver revisão" numa pergunta preparada.

**Dados [C]:** `GET /api/turns` (`turn.query`, o arquivo que o envio entrega) e `GET /api/evidence` (marca, pacote, seleção, H1 a H3 refeitos na consulta, H4, medição, avisos).

| Bloco | Conteúdo | Rótulo obrigatório |
|---|---|---|
| **A pergunta preparada** | O texto completo da query que o envio entrega [C] | — |
| **Pacote gravado** | Marca, formato (1 ou 2), fichas, unidades, seleção, acréscimos do servidor com `declared_by` [C] | "Pacote como foi gravado ao preparar" |
| **Política e critérios usados** | A política e os critérios gravados com a pergunta [C] | "Gravados com a pergunta; podem diferir do perfil atual" |
| **Conferências nesta consulta** | H1, H2 e H3 refeitos agora; H4 informativo | "Conferido nesta consulta; o envio confere de novo. Conferido não quer dizer aprovado" |
| **Escolhido para o envio** | Provedor, modelo, Modelfile, skills; o texto do SYSTEM se `GET /api/modelfiles` trouxer o conteúdo ([Validar]) | "Escolhido para o envio. A identidade do SYSTEM e das skills não é garantida por esta revisão (D4)" |
| **Só você vê** | Lacunas e candidatos, comportamento, de fora, justificativas, categorias, medição na pergunta (divergência no formato 2; aviso "não calculado" no formato 1), ambiguidades [C] | — |
| **Não conferido nesta revisão** | Lista fixa: "a identidade entre esta revisão e o envio (D3)", "SYSTEM e skills (D4)", "recusa por fato revogado (D7)" | Sempre visível |

**Proibido na v1:**

- dizer "o pacote revisado é o enviado";
- apresentar um aviso do cliente como proteção do servidor.

### 4.2 Envio

| Ação | Rota [C] | Regras |
|---|---|---|
| Enviar | `POST /api/send {key, run, turn, provider, modelfile, claude_model, skills}` | Desabilitado se H1 a H3 falharam na consulta. O servidor refaz as conferências [C]. Não aciona Salvar |
| Depois do envio | — | Mostra o `evidence_sha256` do vínculo ao lado do hash visto na revisão, com o rótulo "comparação feita pela página, depois do envio" |
| Voltar ao rascunho | — | Abre o Builder; a pergunta fica "preparada, não enviada" [A] |
| Nenhum | `POST /api/paste` | Copiar a query e colar a resposta |
| Apagar | `DELETE /api/turn` | Só explícito, com confirmação |

### 4.3 Fato revogado depois de preparar [A, I-4]

Antes de enviar, a página relê `GET /api/facts`, quando possível.

- Se um fato de `reviewed_facts` do pacote foi revogado, aparece o aviso **"O fato F foi revogado depois da preparação. Recomendado: preparar de novo."**, com o botão "Voltar ao rascunho".
- **Não diz que o servidor bloqueou**, e Enviar continua possível.
- Se a releitura falhar, aparece "Não foi possível conferir revogações antes do envio".
- A recusa garantida no servidor depende do D7.

### 4.4 Rotas e revisão

| Rota | v1 | Alvo |
|---|---|---|
| Builder sem escada | Revisão parcial | Completa com D3, D4 e D7 (Etapa 5) |
| Achado do inventário | Revisão parcial, sem seleção registrada | Com D2 |
| Lotes | Revisão parcial de lote (4.5) | Etapa 5 |
| Escada (Melhorar com Builder, Otimizar, Pedir esta mudança) | **Sem revisão**, com o rótulo [L] | C1 (D3) |
| Cadeia (`/api/continue`) | **Como hoje, sem revisão.** A tela não afirma que passa por revisão [A, Q6] | Depende do contrato da Etapa 5 |
| `/api/ask` e perguntas por caminho | Sem revisão; "O que o modelo recebeu" [A] | Igual |

### 4.5 Lotes: perguntas independentes [A, Q4]

| Regra | Especificação |
|---|---|
| Criação | Separar e Conferir fatos criam as perguntas **pelo comportamento atual**: uma por item, cada uma com **uma** fonte e o modelo fixo qwen, no modo Localizar (com a confirmação de T26 se a política for outra) |
| Independência | **Cada pergunta tem pacote, conferência, envio, resultado e falha próprios. Nenhuma fonte de uma entra no pacote de outra.** A revisão só agrupa as perguntas na tela |
| Revisão do lote | Lista das N, cada uma com texto, fonte, a própria query, o próprio H1 a H3 e o próprio estado. Todas vêm marcadas |
| Desmarcar | Depois de criadas e antes de enviar. **As desmarcadas ficam "preparadas, não enviadas".** Desmarcar antes da criação fica fora da v1 |
| Envio | Uma por vez, com "k de N". **A falha de uma (conferência, provedor, erro) não interrompe as demais**, e cada falha aparece na própria pergunta |
| Pergunta com H1 a H3 falhando | Desmarcada automaticamente, com o motivo. As outras seguem |
| Criação com falha | Se a criação de uma falha, as outras continuam. [Validar] se o laço atual de Separar para no primeiro erro; se parar, F10 corrige no frontend |

### 4.6 Comparação temporária [A, I-5, Q11]

**Fonte:**

- Só a resposta de `/api/ladder` (`steps`) **enquanto estiver na memória da sessão**.
- **O resultado da rodada não é persistido hoje como registro próprio** [C].

**Conteúdo:**

- Uma coluna por proposta: texto; diff; execuções por comando (estado, saídas iguais, contagem ou medida); `worse`; `gain_pct`.
- O resultado sai **como o servidor devolveu**: `passed` vira "Maior %, critério atual"; `passed_smaller` vira "Passou, ganho menor"; `refused_by_gate` vira "Recusada pelo portão: {motivo}".
- Ordem da resposta. **Sem dominância e sem Pareto** [A].
- Ações por proposta: Copiar diff (4.8).

**Rótulos fixos:**

- "Rodada desta sessão — não sobrevive ao recarregamento".
- "Medida nesta máquina; não prova causa".

**Proibido:**

- calcular ganho, piora ou resultado no JavaScript;
- depois de recarregar, montar o lado a lado a partir das execuções.

**Depois de recarregar:** a vista diz "A rodada não foi gravada; a comparação persistente depende do D5 (Etapa 6)". As execuções continuam visíveis na pergunta (4.7), sem resultado de rodada.

**D5:** persistir o resultado completo como registro novo, com o cálculo no backend (`ladder.rs`). **Sem enfraquecer o H3 nem os contratos históricos.** Rodadas antigas aparecem como "anteriores ao registro".

### 4.7 Conversa e respostas

**Linha do tempo:** as perguntas da run ativa, em ordem [C], com os cartões de desempenho marcados "sem modelo" [C, memória].

**Estado derivado de cada pergunta [P].** Só do que está gravado:

| Estado | Regra |
|---|---|
| Preparação recusada | `prepare_error` |
| Preparada, não enviada | Sem envios |
| Aguardando resposta colada | Envio manual sem texto |
| Respondida | Com texto, e "N avisos da conferência" |
| Decisões pendentes | Claims sem revisão |
| Com fato aprovado / com recusa | Revisões |
| Mudança executada | Execuções com `change.turn` = esta pergunta |

**"Revisada" não é estado**, porque a revisão não é gravada [C, L].

**Pergunta ativa aberta:**

1. **Pergunta:** texto; **política gravada com a pergunta**; **critérios usados** (texto e hash), separados do padrão atual do projeto; origem; seleção; "Ver revisão".
2. **Cada envio em três camadas:**

| Camada | Conteúdo |
|---|---|
| **1. O que chegou** | Resposta; provedor, modelo, Modelfile, skills (hash); custo, tempo; "O que o modelo recebeu" |
| **2. O que o Librarian conferiu** | ISSUES explicados; claims com localização e citações; edições e diff; H1 a H4; comparação de conteúdo entre envios [C]; aviso fixo: "Referência conferida não significa que a relação semântica esteja comprovada." |
| **3. O que você decide** | Aprovar ou Recusar com nota (`POST /api/review`); recusar depois de aprovar revoga [C]. LACUNA e INFERENCE sem "Aprovar", com o motivo [A] |

3. **Proposta de mudança** (claim com diff derivado): diff, "Rodar com esta mudança" (`POST /api/commands/run` com `change`) [C], **Copiar diff** (4.8), aviso "Aplicar ao repositório é com você: aplique, faça commit e gere uma nova run" [A].
4. **Execuções** desta pergunta e **fatos** desta pergunta, com o estado de cada um.

**Ações sobre a pergunta:** reenviar; continuar em cadeia (o diálogo escolhe modo e LLM; se o modo escolhido difere da política, OK pede a confirmação de T26 e não aciona Salvar); colar resposta; comparar (se houver rodada na sessão); apagar (com confirmação). **Nenhuma usa o rascunho.**

**Ativar uma pergunta:** liga o realce da busca e voa só com o clique da pessoa. A pergunta fica guardada por run. O rascunho não é tocado.

### 4.8 Copiar diff [A, Q12]

| Aspecto | Especificação |
|---|---|
| Onde | Na proposta de mudança da camada 2 e em cada coluna da Comparação |
| Conteúdo | **Exatamente** o texto de `check.diffs[claim]` vindo do servidor: o mesmo que a execução aplica ([execution.rs:429](../crates/librarian-studio/src/execution.rs#L429)). Sem reformatar e sem montar no cliente |
| Sem diff derivado | Sem botão, e com o motivo: "Esta proposta não tem diff derivado" (as edições falharam [C]) |
| `k.patch` (patch proposto em texto) | Mostrado como hoje. **Não** é o "diff" copiado |
| Confirmação | "Diff copiado: N linhas, arquivos: a, b. Nada foi aplicado ao projeto." (EN: "Diff copied: N lines, files: a, b. Nothing was applied to the project.") |
| Falha da área de transferência | O diff é selecionado num campo de texto, com a mensagem "Não foi possível copiar automaticamente; o diff está selecionado para copiar" |
| Escrita | **Nenhuma**: nem no projeto nem em `.studio`. O caminho do `.diff` gravado no envio continua mostrado [C] |

---

## 5. Estado e persistência

### 5.1 Tipos de estado

| Tipo | Onde | Conteúdo |
|---|---|---|
| Permanente | `.studio` [C] | Projeto e perfil (com os **critérios padrão** e a **política do modo**), runs e editions, perguntas (com os **critérios usados** e a **política usada**), sugestões, envios, revisões, execuções, aprovações, fatos, léxico |
| Navegação | Navegador [P] | Projeto, run por projeto, pergunta ativa por run, área ou ferramenta, vista de Explorar, vista central |
| Rascunho | Navegador, por projeto e run [A] | 5.4 |
| Preferências | Navegador [C] | Idioma, tema, `llm.<tipo>`, skills, Modelfile. `librarian.mode` deixa de decidir o modo (3.13.4) |
| Derivado | Refeito | Ranking, estados dos fatos, inventário, H1 a H3 na consulta, H4, comparação de conteúdo, resumo, modo mostrado (da política gravada) |
| Memória da página | Some ao recarregar | Seleção, câmera, filtros, rolagem, pilha de inspeção, fluxograma, cartões, **rodada da escada**, pilha do Voltar, edições não salvas de Configurações e Léxico |

### 5.2 Identidade [A, I-2]

| Item | Identificador |
|---|---|
| Projeto | `key` |
| Run | `id` |
| Edition (local) | `catalog_sha256` do grafo |

A identidade local **não substitui** a conferência do servidor ao preparar, que usa `edition:<hash do manifest>` e o registro das sugestões.

### 5.3 Chaves no navegador [P]

| Chave | Conteúdo |
|---|---|
| `librarian.nav.v1:<key>` | `{run, area, explore_view, center:{view, turn?}, active_turn:{<run>: turn}}` |
| `librarian.draft.v1:<key>:<run>` | Rascunho |
| Chaves atuais | Mantidas. `tab` é convertida uma vez. `mode` fica sem uso para o modo (3.13.4) |

- Toda leitura e escrita é protegida, e uma falha leva a 5.8.
- **Pilha do Voltar:** pontos de navegação, só na memória. **Nunca desfaz o rascunho nem registros.**
- **Voltar e troca de projeto [A, esclarecimento de 11/11/2026]:**
  - Voltar percorre **somente pontos do projeto aberto**.
  - Abrir outro projeto limpa a pilha anterior: o Voltar começa desabilitado no projeto aberto e passa a registrar os pontos dele a partir do primeiro movimento.
  - Nada do projeto anterior passa para o novo: nem pontos, nem área ou vista, nem inspeção.
  - Cada projeto conserva a própria navegação guardada (`librarian.nav.v1:<key>`), e trocar de projeto não a altera.
  - Voltar nunca reabre outro projeto: retornar a ele exige abri-lo explicitamente, o que restaura a navegação guardada dele, com a pilha vazia.
  - Não há pilha global de projetos. A troca de run no mesmo projeto segue T14.
- **A política do modo nunca é guardada no navegador.**

### 5.4 Conteúdo do rascunho

| Parte | Campos |
|---|---|
| Cabeçalho | Esquema, `key`, `run`, `catalog_sha256`, `saved_at`, `{tab, seq}` |
| Builder | Alvos; as quatro listas com os campos de fonte do formato 2; `decision`, `decision_scope`; `suggestions`; `with_facts`; entrada (`kind`, `summary`); "chegou por" (local) |
| Achado (I-6) | `target`, `kind`, itens, marcados |
| Compositor | Texto, escada ligada, **critérios do rascunho** (só se diferem do padrão), **modo necessário** (só quando uma entrada o define: Otimizar e cartão → Melhorar; achado → Localizar) |
| Vínculo | `prepared_as: {turn, digest}` |

- A parte do Builder é o mesmo objeto que a página monta hoje para `origin.selection`.
- O modo necessário é um requisito da pergunta em montagem, não a política: restaurar o rascunho nunca altera a política (3.13).
- O modelo escolhido fica como preferência por tipo [C].

### 5.5 Restaurar

| Passo | Conferência | Se falhar |
|---|---|---|
| 1 | Esquema conhecido | "Rascunho em formato desconhecido": não aplica; Guardar ou Descartar |
| 2 | `key` e `run` | Não aplica |
| 3 | `catalog_sha256` igual | "Rascunho de outra edition: não aplicado"; mostra os nomes; Descartar. **Nunca aplica** |
| 4 | Ids existem no grafo | "N fontes não existem nesta edition: {nomes}"; não aplica; Descartar, ou recuperar só o texto do compositor, com confirmação |
| 5 | Provedor, Modelfile e skills existem | "Indisponível" no compositor; o resto se aplica |
| 6 | Hashes das sugestões | Sem rota de leitura: `accept_selection` confere ao preparar. A recusa aparece e o rascunho fica |
| 7 | Critérios do rascunho e padrão atual | Se o padrão mudou desde a gravação: "O padrão do projeto mudou desde este rascunho", mostrando os dois |
| 8 | Modo necessário e política atual | Se diferem, o caminho previsto mostra a diferença (3.11). Nada é gravado |

### 5.6 Troca de run e retorno [A, I-2]

- **Sair de uma run** guarda o rascunho dela.
- **Entrar numa run** restaura o rascunho daquela run pela regra de 5.5, se houver.
- **Nunca transfere** um rascunho para outra run.
- Não há descarte, então não há confirmação de perda. Aparece o aviso "O rascunho da run X ficou guardado".
- A política do modo é do projeto e continua a mesma (3.13.3).

### 5.7 Duas abas

- Gravação com `{tab, seq}` e escuta do evento `storage`.
- Se outra aba gravar o mesmo rascunho, esta entra em **"Rascunho alterado em outra aba"**: só leitura até a pessoa escolher "Carregar a versão da outra aba" ou "Manter a minha (sobrescrever)".
- **Nunca há mesclagem.**
- Perfil: antes de Salvar ou de alterar a política, a página relê o perfil do servidor e mostra as diferenças, se houver (3.13.3). A janela de corrida que sobra é a limitação de 3.13.5.

### 5.8 Armazenamento indisponível, cheio ou limpo

| Situação | Comportamento |
|---|---|
| Indisponível | "Rascunho só nesta página"; aviso do navegador ao sair com rascunho não vazio |
| Cota excedida | Mesmo estado, "Não foi possível guardar o rascunho"; mantém a última versão guardada |
| Limpo | Nada a restaurar; abre normalmente. O modo continua vindo do perfil |

### 5.9 Pergunta preparada e não enviada [A]

Fica registrada. Só sai por Apagar, explícito e com confirmação. Desistir, recarregar, desmarcar no lote e trocar de run não apagam.

### 5.10 O navegador nunca é prova

Pergunta, seleção, pacote, critérios usados e política usada são sempre do servidor.

---

## 6. Fluxos e transições

| # | Origem | Ação | Destino | Dados preservados | Condição e confirmação |
|---|---|---|---|---|---|
| T1 | Explorar (lista) | Selecionar uma função | Inspeção sobre a lista | Lista, rolagem, filtros, dobras, vista central, rascunho | — |
| T2 | Fora de Explorar | Selecionar uma função | **Mesma área**, com "Selecionado: X [Inspecionar]" | Tudo | "Inspecionar" leva a T1 e grava o ponto no Voltar |
| T3 | Inspeção de nó | Inspecionar uma relação | Inspeção da aresta, empilhada | Pilha | "←" volta um nível |
| T4 | Inspeção ou Fluxograma | Juntar ao Builder | Mesma vista; contador sobe | Vista e seleção | Bloqueado, com o motivo, durante uma entrada que recomeça |
| T5 | Inspeção | Abrir fluxograma | Centro › Fluxograma | Câmera, seleção, Builder | Confirma substituir se houver escolhas |
| T6 | Onde pesa ou cartão | Otimizar ou Pedir esta mudança | Builder com rascunho novo (modo necessário Melhorar) | Explorar e a lista; política inalterada | Rascunho não vazio: confirmar |
| T7 | Onde pesa ou cartão | Separar ou Conferir fatos | N perguntas independentes e Revisão de lote | Rascunho do Builder intacto | Criação pelo comportamento atual; T26 se a política não for Localizar |
| T8 | Inventário | Investigar um achado | Builder em modo achado (fluxo atual) | Inventário e a lista; política inalterada | Rascunho não vazio: confirmar |
| T9 | Linha do tempo, Régua ou Acervo | Abrir uma pergunta antiga | Pergunta ativa; realce da busca | **Rascunho e compositor intactos** | Outra run: T13 |
| T10 | Compositor | Preparar | Revisão (rotas 3 e 4); a pergunta entra na linha do tempo | Rascunho, marcado "preparado como #N" | Critérios editados: bloqueio de 3.12. Modo necessário diferente: T26. Não aciona Salvar |
| T11 | Revisão | Revisar | Leitura e conferências na consulta | — | H1 a H3 falhando desabilitam Enviar |
| T12 | Revisão | Enviar | Resposta na pergunta; o hash do vínculo é mostrado | Rascunho | Aviso de fato revogado (4.3), sem bloqueio |
| T13 | Acervo ou origem em outra run | Abrir a origem de um fato | Troca de run e ativa a pergunta | Rascunho guardado na run de saída | Confirmação: "A origem está na run X. Trocar?" |
| T14 | Cabeçalho ou Gerar | Trocar de run | Grafo, perguntas e rascunho da nova run; vistas de leitura da anterior fecham | Rascunho por run; área; vista de Explorar; política do projeto | Aviso de rascunho guardado |
| T15 | Pergunta com rodada na sessão | Comparar | Comparação temporária | Tudo | Sem rodada na memória: mensagem do D5 |
| T16 | Camada 3 | Aprovar um fato | Claim "Aprovado por pessoa"; fato no Acervo | Tudo | LACUNA e INFERENCE sem botão |
| T17 | Qualquer | Recarregar | Projeto, run, pergunta ativa, área, vista de Explorar, vista central quando dá para refazer; modo sincronizado com o perfil | Rascunho, por 5.5 | Fluxograma, filtros, câmera, cartões, rodada e edições não salvas recomeçam (aviso do navegador se houver edições não salvas) |
| T18 | Qualquer | Voltar | Ponto anterior | Rascunho e registros | Sem ponto anterior: desabilitado. Só pontos do projeto aberto; abrir outro projeto limpa a pilha (5.3) |
| T19 | Revisão | Voltar ao rascunho | Builder | A pergunta continua preparada | — |
| T20 | Compositor | Alterar a política do modo | Compositor, com o seletor atualizado só depois do sucesso | Rascunho; edições não salvas continuam não salvas | Grava só os dois campos (3.13.2). Falha: seletor no modo anterior |
| T21 | Builder | Limpar rascunho | Builder vazio | — | Confirmar |
| T22 | Projeto | Gerar | Nova run (T14) | Rascunho da anterior | Não aciona Salvar. Com edições não salvas: "Gerar com as gravadas", "Salvar e gerar", "Cancelar" |
| T23 | Proposta ou Comparação | Copiar diff | Mesma vista | Tudo | Confirmação da cópia (4.8) |
| T24 | Configurações, Léxico ou compositor | Salvar | Perfil gravado | Rascunho; política do modo | Mostra os campos que mudam antes de gravar. Falha: nada gravado, edições ficam |
| T25 | Projeto | Abrir ou trocar de projeto | Projeto novo; run guardada ou a mais recente | Rascunhos guardados de cada projeto e run | **Modo sincronizado com a política do projeto aberto**; o modo do projeto anterior nunca é aplicado. Edições não salvas do projeto anterior: confirmar antes de trocar |
| T26 | Ação que exige um modo (Preparar com modo necessário, escada, lotes, caminho, cadeia) | Executar com a política diferente | A ação, depois de alterar a política | Rascunho; edições não salvas continuam não salvas | Confirmação: "Esta ação usa o modo {X}; a política do projeto está em {Y}. Alterar a política para {X} e continuar?" Executa a operação de 3.13.2. Falha ou cancelar: a ação não acontece |

---

## 7. Estados e mensagens PT/EN

### 7.1 Estados de cada informação [A]

Sempre por texto.

| Estado | PT | EN | Definição |
|---|---|---|---|
| Estrutural | Estrutural | Structural | Relação do grafo |
| Medido | Medido | Measured | Informação instrumentada |
| Proposto | Proposto pelo modelo | Proposed by the model | Hipótese |
| Conferido | Conferido mecanicamente | Mechanically checked | Referência conferida |
| Aprovado | Aprovado por pessoa | Approved by a person | Fato aprovado |
| Ambíguo | Ambíguo | Ambiguous | Candidatos sem resolução automática |

**Fatos:** Vigente / Current · Desatualizado / Stale · Revogado / Revoked · Aplicada / Applied [C].

**Proibido:**

- "aprovado" ou "validado" para H1 a H4, claims ou portão;
- "vencedora" ou "melhor" para `passed`;
- "garantido" para a revisão parcial;
- "bloqueado pelo servidor" para um aviso do cliente;
- "salvo" para uma alteração de política, ou "política alterada" para um Salvar.

### 7.2 Estados das operações [P]

Cada mensagem diz o quê, por quê e o próximo passo.

| Estado | PT | EN |
|---|---|---|
| Carregando | Carregando {o quê}… | Loading {what}… |
| Vazio | Nenhum {o quê} ainda | No {what} yet |
| Indisponível | Não disponível: {motivo} | Not available: {reason} |
| Falha de leitura | Não foi possível ler {o quê} | Could not read {what} |
| Erro | {mensagem do servidor como veio} | idem |
| Não salvo | Alterações não salvas | Unsaved changes |
| Em andamento | {Ação}… (k de N) | {Action}… (k of N) |
| Concluído | {Ação} concluída: {o que foi gravado} | {Action} done: {what was recorded} |

### 7.3 Mensagens fixas das limitações e das gravações

| Situação | PT | EN |
|---|---|---|
| Escada sem revisão | Esta rota envia sem revisão (depende do C1) | This route sends without review (depends on C1) |
| Revisão parcial | Não conferido nesta revisão: identidade com o envio (D3), SYSTEM e skills (D4), recusa por fato revogado (D7) | Not checked in this review: identity with the send (D3), SYSTEM and skills (D4), refusal for a revoked fact (D7) |
| Fato revogado | O fato F foi revogado depois da preparação. Recomendado: preparar de novo. | Fact F was revoked after preparation. Recommended: prepare again. |
| Comparação | Rodada desta sessão — não sobrevive ao recarregamento | This session's round — does not survive a reload |
| Critérios editados | Os critérios editados ainda não podem valer só para esta pergunta (depende do D10) | Edited criteria cannot yet apply to this question only (depends on D10) |
| Achado | Achado do inventário: sem registro de seleção no servidor | Inventory finding: no selection record on the server |
| Diff copiado | Diff copiado: N linhas, arquivos: … Nada foi aplicado ao projeto. | Diff copied: N lines, files: … Nothing was applied to the project. |
| Salvar | Salvar estas alterações no perfil do projeto? {campos} | Save these changes to the project profile? {fields} |
| Salvo | Configurações salvas: {campos} | Settings saved: {fields} |
| Política alterada | Política do projeto alterada: {modo} ({o que permite}) | Project policy changed: {mode} ({what it allows}) |
| Política não alterada | A política não foi alterada: {mensagem do servidor}. O modo continua {modo anterior} | The policy was not changed: {server message}. The mode stays {previous mode} |
| Modo necessário | Esta ação usa o modo {X}; a política do projeto está em {Y}. Alterar a política para {X} e continuar? | This action uses the {X} mode; the project policy is {Y}. Change the policy to {X} and continue? |
| Política personalizada | Política personalizada: hipótese {sim/não}, patch {sim/não} | Custom policy: hypothesis {yes/no}, patch {yes/no} |
| Perfil mudou fora | O perfil foi alterado fora desta página: {campos} | The profile was changed outside this page: {fields} |

### 7.4 Matriz por área

| Área | Vazio | Indisponível | Falha de leitura | Em andamento | Não salvo |
|---|---|---|---|---|---|
| Projeto | Sem projeto ou run | — | Item do resumo | Abrindo, gerando | Entradas |
| Onde pesa | Sem medição | Linguagem sem contador; sem comando aprovado | Medição | Medindo | — |
| Inventário | Sem achados | — | Inventário | Montando | — |
| Fluxograma | Sem usos | Sem servidor | — | Desenhando | Escolhas de homônimos |
| Builder | Sem fontes | Sem servidor | Sugestão; medição (D9) | Sugerindo | Só nesta página |
| Compositor | Sem run | Sem comando aprovado (escada) | — | Preparando; alterando a política | Critérios do rascunho |
| Revisão | — | Rota sem revisão | Pacote (H1) | Enviando k de N | — |
| Conversa | Sem perguntas | — | Pergunta | Enviando | Resposta colada não conferida |
| Régua | Sem comandos | Run com alterações | Execuções | Rodando | — |
| Comparação | Sem rodada na sessão | Rodada não gravada (D5) | — | — | — |
| Acervo | Sem fatos | — | Fatos | — | — |
| Ferramentas | Léxico vazio | Sem servidor | — | Sugerindo; salvando | Texto editado; configurações |

**Regras:** português e inglês com o mesmo sentido. Mensagens do servidor aparecem como vieram [C].

---

## 8. APIs e dependências

### 8.1 Rotas por área (todas existentes [C], nenhuma muda)

| Área | Leitura | Escrita |
|---|---|---|
| Projeto | `GET /api/status`, `/api/browse`, `/api/project`, `/api/graph`, `/api/turns`, `/api/facts`, `/api/commands`, `/api/hotspots` | `POST /api/project`, `POST /api/generate` |
| Explorar | `/api/hotspots`, `/api/measure-detail`, `/api/inventory`, `/api/value-sources`, `/api/used-by`, `POST /api/calls`, `POST /api/inventory/investigate` | `POST /api/instrument/run` |
| Builder | — | `POST /api/envelope/suggest` |
| Compositor | `POST /api/performance-question`, `GET /api/project` (antes de alterar a política) | `POST /api/gathered-question`, `/api/ask`, `/api/ladder`, `/api/path-questions`, `/api/continue`, `PUT /api/project` (só pelas duas operações de 3.13) |
| Revisão | `/api/turns`, `/api/evidence`, `/api/facts` | `POST /api/send`, `/api/paste`, `DELETE /api/turn` |
| Conversa | `/api/turns`, `/api/executions`, `/api/facts` | `POST /api/review`, `/api/commands/run` |
| Régua | `/api/commands`, `/api/executions` | `POST /api/commands/approve`, `/api/commands/run` |
| Acervo | `/api/facts` | `POST /api/review` |
| Ferramentas | `/api/modelfiles`, `GET /api/project` (antes de Salvar) | `POST /api/lexicon/suggest`, `/api/lexicon/check`, `PUT /api/project` (Salvar), `POST /api/project/export`, `PUT /api/modelfiles` |

### 8.2 Dependências D1 a D11

| # | Habilita | Classificação | Etapa | Limitação aceita na v1 | Rota ou contrato afetado | Risco de compatibilidade | Testes | Ordem |
|---|---|---|---|---|---|---|---|---|
| D1 | Política por pergunta | Backend | Sem etapa no plano; autorização própria | O modo é a política do projeto: alterá-lo grava os dois campos no perfil, por operação explícita (3.13) | Campo opcional de política nas rotas de pergunta; na ausência, vale o perfil | Baixo se opcional. O H3 refaz a query: a política gravada precisa entrar na reconstrução | Com e sem o campo; H3, gabarito e manifesto iguais | B2 |
| D2 | Achado no Builder com cadeia e motivos registrados | Backend | Autorização própria | `investigationView` e `ticked` mantidos | `accept_selection` (origem `loop`/`operation` com seleção); proposta do inventário registrada por hash | **Alto:** registro da seleção, H1 e formato. Origens `inventory` antigas lidas como eram | Mutações por asserção; H3 histórico; fixture congelada | B6 |
| D3 | Revisão completa: escada com revisão (C1), recusa no envio, fonte (C2) | Backend | **Etapa 5** | Revisão parcial; escada sem revisão | `/api/ladder` (`turn`), `/api/send` (hash esperado), `GET /api/source` | Médio. Os campos opcionais preservam os clientes atuais | Com e sem `turn`; hash certo, errado e ausente; segurança de leitura | B3 |
| D4 | SYSTEM e skills conferidos | Backend | **Etapa 5** | Mostrados como escolha, sem garantia | Escolha fixada na revisão ou esperada no envio | Baixo, aditivo | Escolha trocada → recusa | B3 |
| D5 | Rodada da escada persistente; Comparação depois de recarregar | Backend | **Etapa 6** | Comparação só da sessão | Registro novo, imutável, da resposta de `/api/ladder`, com rota de leitura; cálculo em `ladder.rs` | Baixo (novo). **Não enfraquece o H3 nem os contratos históricos.** Rodadas antigas sem registro | Registro igual à resposta; leitura depois de recarregar; H3 histórico igual | B4 |
| D6 | "Chegou por" gravado | Backend | Opcional; autorização própria | Só na tela e no rascunho local | Formato novo da seleção (o `OriginKind` é fechado; o formato 2 recusa campos desconhecidos) | **Alto** | Formatos 1, 2 e novo; H3 histórico | B7 |
| D7 | Recusa por fato revogado | Backend | **Etapa 5** | Aviso do cliente (4.3) | `verify_for_send` confere `reviewed_facts` | Médio: reenvio de perguntas antigas com fato revogado passa a ser recusado, e isso precisa estar documentado | Revogar entre preparar e enviar → recusa | B3 |
| D8 | Língua do léxico explícita | **Frontend com contrato existente** (`language`) | v1 | — | Nenhum | Nenhum | O pedido gravado leva a língua | F11 |
| D9 | Falha de leitura da medição na sugestão | Backend aditivo | Autorização própria | Aparece "sem medição" | Resposta de `/api/envelope/suggest` expõe o estado que o registro já tem | Baixo | Medição corrompida → estado aparece; sugestão normal igual | B1 |
| D10 | Critérios do rascunho valendo só para a pergunta (Q9) | Backend | Autorização própria (pode ir com o D1) | Preparar bloqueado com critérios editados; só Salvar grava o padrão | Campo opcional de critérios nas rotas de pergunta; o servidor copia para a pergunta (`criteria-before.md` e hash, como hoje) sem tocar o perfil | Baixo se opcional. O envio já confere os critérios pelo arquivo da pergunta | Com critérios no corpo: o perfil continua igual e a pergunta grava os enviados; sem eles, igual a hoje | B2 |
| D11 | Gravação do perfil sem corrida | Backend | Autorização própria (pode ir com o D1) | Releitura antes de gravar; resta a janela de 3.13.5 | Gravação parcial (só os campos enviados) ou com versão do perfil (recusa se mudou desde a leitura) em `PUT /api/project` ou rota nova | Baixo se a forma atual continuar aceita | Duas gravações concorrentes: a segunda recebe recusa ou só altera os próprios campos; a gravação atual continua igual | B2 |

### 8.3 Resumo da separação

| Grupo | Itens |
|---|---|
| Frontend com contrato existente | Toda a estrutura (F1 a F9, F11); revisão parcial (4.1); lotes (4.5); aviso de fato revogado; Comparação temporária; Copiar diff; Salvar e Alterar a política como operações distintas (3.13); sincronização do modo com o projeto; D8 |
| Exige backend | D1, D2, D3, D4, D5, D6, D7, D9, D10, D11 |
| Etapas futuras do plano | Etapa 5: D3, D4, D7. Etapa 6: D5 e a Comparação persistente. Etapa 7: critérios de aceitação (seção 9) |
| Limitações aceitas na v1 | Política do modo é do projeto e alterá-la grava o perfil; janela de corrida na gravação do perfil; critérios editados bloqueiam o preparo; escada e cadeia sem revisão; revisão parcial; aviso de fato revogado só do cliente; Comparação só da sessão; achado sem seleção registrada; "chegou por" só local; falha de leitura vista como "sem medição"; "revisada" não é estado gravado |

### 8.4 Mudanças só no frontend pressupostas

1. `selectNode` sem troca forçada de área.
2. `loadRun` guarda o rascunho e ativa a pergunta guardada.
3. `saveSettings()` deixa de ser chamada fora do botão Salvar e das opções explícitas "Salvar e …": saem as chamadas de Gerar, `askModel`, escada, cadeia, Separar, Conferir fatos, perguntas por caminho, Sugerir léxico, Aplicar léxico e Exportar perfil.
4. Operação nova "Alterar a política do modo": relê o perfil, troca só os dois campos, grava, e só então atualiza o seletor.
5. O seletor do modo sai da política do projeto ao abrir e trocar de projeto; `librarian.mode` deixa de decidir o modo; `applyMode()` deixa de sobrescrever a política carregada.
6. Os fluxos que forçam um modo (`setMode` em Separar, Conferir fatos, caminho, achado, Otimizar, cartão, cadeia) passam a pedir a confirmação de T26 em vez de trocar o modo em silêncio.
7. Sequência nos pedidos de sugestão e investigação.
8. Guardas `sameRun` em `askFacts`, `askHotFacts`, `continueTurn`, `askPath`, `drawPath`.
9. O texto do compositor não é apagado ao preparar.
10. Lista de fontes fora do compositor; o achado mostrado no Builder.
11. Laço dos lotes sem parar na primeira falha (se hoje para).

---

## 9. Critérios de aceitação

Esta seção é a forma da Etapa 7 do plano para esta implementação. Testes automáticos passando não bastam [A].

**9.1 Navegação**

1. Quatro áreas e quatro ferramentas com `aria-current`; o Léxico se alcança com um clique de qualquer lugar.
2. Selecionar um nó fora de Explorar não muda a área. "Inspecionar" muda e grava o ponto no Voltar.
3. Inspeção e volta restauram vista, rolagem, filtro e dobras.
4. Ir e voltar entre as vistas centrais preserva a câmera (com tolerância numérica) e a seleção.
5. Fechar o Builder não altera o rascunho.
6. Voltar nunca altera rascunho nem registros.
7. Sem estado guardado, o Studio abre em Projeto, com o resumo e a cidade.

**9.2 Rascunho por projeto e run**

1. Recarregar restaura o mesmo objeto.
2. Trocar de run guarda o rascunho, e voltar o restaura.
3. Nenhum rascunho aparece em outra run.
4. Com `catalog_sha256` diferente ou fonte inexistente, o rascunho não é aplicado.
5. Duas abas: estado "alterado em outra aba", sem mesclar.
6. Com o armazenamento bloqueado, a página funciona, mostra "só nesta página" e avisa ao sair.
7. Toda substituição de rascunho não vazio pede confirmação, e Cancelar não muda nada.
8. Juntar durante o carregamento não se perde em silêncio.
9. Restaurar um rascunho nunca grava o perfil.

**9.3 Critérios**

1. Editar critérios no compositor não muda `librarian.json` nem o perfil em `.studio` (comparação de bytes antes e depois de preparar, gerar e alterar a política).
2. Com critérios editados, Preparar fica bloqueado com as três opções.
3. Só "Salvar" grava, e mostra o que muda.
4. A pergunta mostra os critérios usados, separados do padrão atual.

**9.4 Salvar e Alterar a política do modo**

1. **Salvar** grava as configurações editáveis e só depois de mostrar a lista dos campos. Gerar, Preparar, Perguntar, escada, cadeia, lotes, caminho, Sugerir léxico e Exportar não gravam o perfil (comparação de bytes do `project.json` antes e depois de cada uma, com edições não salvas na tela).
2. **Alterar a política** grava só `allow_hypothesis` e `allow_patch`: com edições não salvas em Configurações, Léxico e critérios, o `project.json` depois da operação difere do anterior só nesses dois campos, e as edições continuam marcadas "não salvas".
3. **Abrir projetos diferentes:** com o projeto A em Melhorar e o B em Localizar (gravados no perfil), abrir A mostra Melhorar e abrir B mostra Localizar, em qualquer ordem, com recarregamento entre eles e com qualquer valor em `librarian.mode`. Nenhuma das aberturas grava o perfil.
4. **Alternar modos:** trocar de modo em A não muda o perfil de B; trocar de run em A mantém o modo de A.
5. **Falha de gravação simulada** (servidor recusando `PUT /api/project`, por exemplo com entradas vazias numa cópia de teste, ou rede interrompida): o seletor continua no modo anterior, a mensagem "A política não foi alterada" aparece e o perfil fica igual em bytes.
6. **Política personalizada:** um perfil com `allow_hypothesis` diferente de `allow_patch` mostra "Política personalizada" e não é alterado ao abrir.
7. **Ação que exige outro modo:** Separar com a política em Melhorar pede a confirmação de T26. Cancelar não cria perguntas nem grava; confirmar grava só os dois campos e cria as perguntas.
8. **Perfil alterado fora:** com o perfil gravado por outra aba depois da leitura, Salvar e Alterar a política mostram "O perfil foi alterado fora desta página" antes de gravar.

**9.5 Corpos das APIs**

1. Para cada sequência de ações gravada no F0, o corpo de `/api/gathered-question` (seleção e achado) e de `/api/ladder` (Builder, Otimizar, Pedir esta mudança sem edição) é byte a byte igual.
2. O H3 e o manifesto do histórico continuam iguais à linha de base (`.studio/arquivo-copias/linha-base-etapa4-20261107`).

**9.6 Revisão parcial sem falsas garantias**

1. "A pergunta preparada" é igual ao `turn.query`.
2. H1 a H3 falhando desabilitam Enviar com o texto do servidor.
3. O bloco "Não conferido nesta revisão" está sempre presente.
4. Nenhum texto afirma identidade revisão-envio, garantia de SYSTEM e skills ou bloqueio pelo servidor (busca automática pelas frases proibidas de 7.1 nos dois idiomas).
5. A escada mostra o rótulo do C1.
6. Revogar um fato entre preparar e enviar mostra o aviso e não bloqueia.

**9.7 Lotes**

1. N itens geram N perguntas, cada uma com uma fonte e o próprio pacote.
2. Nenhum pacote contém fonte de outra pergunta: conferência pelos `evidence.json`.
3. As desmarcadas ficam preparadas e não enviadas.
4. Uma falha simulada (provedor de teste ou pacote adulterado numa cópia) não impede as outras, e cada falha aparece na própria pergunta.

**9.8 Comparação temporária**

1. Mostra exatamente `steps` da resposta, com os rótulos.
2. Nenhuma função do cliente calcula ganho, piora ou resultado (revisão de código e teste com `steps` sintéticos cujos valores não batem com as execuções: a tela mostra os do servidor).
3. Depois de recarregar, aparece a mensagem do D5 e nenhum lado a lado.

**9.9 Copiar diff**

1. O texto copiado é igual a `check.diffs[claim]` (comparação de bytes).
2. A confirmação mostra linhas e arquivos.
3. Nenhum arquivo do projeto nem de `.studio` muda (comparação de árvore antes e depois).
4. Com a área de transferência indisponível, o diff fica selecionado.

**9.10 Projetos e capacidades**

1. Ray tracer em Rust, com medição: Onde pesa, Otimizar, escada com provedor de teste, Comparação da sessão, Copiar diff.
2. Logística API em Python, sem `instrument`: "Não disponível" com o motivo, e o Inventário como caminho.
3. Cada um com e sem medição gravada.

**9.11 Histórico e formatos**

1. Perguntas anteriores à Etapa 4, pacotes de formato 1 (`tests/evidence-v1`) e de formato 2, seleções de formato 1 e 2.
2. Execuções sem `change`, fatos revogados e desatualizados aparecem corretamente.

**9.12 Estados, idioma, tema, tela e teclado**

1. Cada célula da matriz 7.4 tem um caso reproduzido.
2. As chaves existem em PT e EN (verificação automática).
3. Sem cor, o texto do estado continua presente.
4. Tema claro, escuro e troca manual.
5. Tela estreita: uma coluna por vez, sem rolagem horizontal, estado preservado.
6. Áreas, barra de vistas, listas, Builder, Revisão, decisões, seletor de modo e Copiar diff funcionam pelo teclado, e os diálogos prendem e devolvem o foco.

**9.13 Sem regressão**

1. `cargo test` passa.
2. As baterias de `tests/tela` passam com os seletores atualizados.
3. Modo arquivo, `#overlayCard`, `/api/ask`, `askPath`, colar resposta, cartão de desempenho e Léxico funcionam.
4. Os dados mostrados batem com `/api/*`.
5. Nenhuma chamada real a LLM.

---

## 10. Plano de implementação F0 a F13

Todos os passos F são **só frontend**, em `crates/librarian-studio/web/index.html` e nas baterias de `crates/librarian-studio/tests/tela`. Nenhum exige backend. Cada um tem autorização, relatório, diff e validação próprios. **Nenhum está autorizado por este documento.**

| Passo | Objetivo | Depende de | Aceitação | Testes | Riscos |
|---|---|---|---|---|---|
| **F0** Linha de base | Baterias verdes; corpos das rotas gravados para a 9.5; bytes do `project.json` antes e depois de cada fluxo que hoje grava o perfil; inventário das chaves i18n | — | Linha de base registrada | Baterias atuais | Baterias gravam turnos: rodar em cópias das raízes |
| **F1** Estado e persistência local | Módulo de navegação, rascunho, armazenamento protegido, duas abas; sem mudança visível | F0 | 9.2.5, 9.2.6 em teste isolado | Armazenamento bloqueado, duas abas, grafo trocado | Arquivo único sem build (mantido) |
| **F2** Estrutura de navegação | Cabeçalho, áreas, ferramentas, Voltar, ponto de partida; painéis movidos sem mudar comportamento | F1 | 9.1.1, 9.1.6, 9.1.7 | Baterias com seletores atualizados | Seletores das baterias e dos scripts de vídeo mudam |
| **F3** Exploração e inspeção | Explorar unificado; inspeção como estado; regra da seleção | F2 | 9.1.2, 9.1.3 | Bateria de navegação | Ações que supõem a aba Mapa aberta |
| **F4** Vistas centrais | Barra de vistas; Fluxograma como vista; `sameRun` | F2 | 9.1.4 | Bateria da barra | Camadas e redimensionamento do canvas |
| **F5** Evidence Builder | Builder no centro; lista fora do compositor; achado em modo achado (fluxo atual); confirmação; sequência; "Pedir esta mudança" pelo Builder; modo necessário no rascunho | F1, F4 | 9.1.5, 9.2.7, 9.2.8, **9.5** | Corpos byte a byte contra o F0 | Maior risco: registro da seleção |
| **F6** Rascunho ligado ao Builder | Restaurar, trocar de run, incompatíveis | F5 | 9.2 | Recarregar, troca de run, edition trocada | — |
| **F7** Compositor e gravação do perfil | Caminho previsto; botão por rota; **Salvar e Alterar a política como operações distintas; sincronização do modo com o projeto; T26**; critérios com bloqueio e Salvar explícito; texto mantido; modelos fixos; remoção das chamadas implícitas a `saveSettings()` | F5 | 9.3, 9.4 e a tabela 3.11 | Bateria por rota; bytes do perfil; falha simulada de gravação; dois projetos | Gravar a partir do perfil relido do servidor, não da tela; muitos fluxos chamam `saveSettings()` hoje |
| **F8** Conversa e histórico | Linha do tempo, estados, três camadas, execuções e fatos por pergunta, critérios e política usados, Copiar diff | F2 | 9.9, 9.11 | Registros históricos | `renderTurn` extenso |
| **F9** Projeto, Régua e Acervo | Resumo de estado, Gerar sem Salvar implícito, links para a pergunta, T13, T25 | F2, F7, F8 | 9.10 | Rust e Python | Muitas leituras ao abrir |
| **F10** Revisão parcial e lotes | 4.1 a 4.5 | F5, F7, F8 | 9.6, 9.7 | Provedor de teste ou Nenhum; cópias adulteradas | Prometer o que depende de D3, D4 e D7 |
| **F11** Ferramentas | Léxico com língua explícita (D8) e Aplicar pelo Salvar; Configurações com a lista do que muda; Exportar sem Salvar implícito; Modelfiles; Abrir arquivos | F2, F7 | Fluxos do Léxico; modo arquivo | Léxico com Nenhum | — |
| **F12** Comparação temporária | 4.6 | F8 | 9.8 | `steps` sintéticos; escada com provedor de teste | Tentação de calcular no cliente |
| **F13** Aceitação completa | Seção 9 inteira; depois, atualizar o STUDIO.md com autorização | Todos | Seção 9 | Windows e Ubuntu, CRLF | — |

**Podem começar sem backend:** todos, de F0 a F13. **Nenhum passo F espera dependência de backend.** As dependências só limitam o que cada passo promete (8.3).

**Passos de backend:** cada um com autorização própria, e nenhum faz parte da v1.

| Passo | Dependência |
|---|---|
| B1 | D9 |
| B2 | D1, D10 e D11 |
| B3 | D3, D4 e D7 (Etapa 5) |
| B4 | D5 (Etapa 6) |
| B5 | Comparação persistente sobre o D5 |
| B6 | D2 |
| B7 | D6 |

Quando um passo B for feito, um passo F de retirada remove a limitação correspondente da tela. Por exemplo, depois do B6 o achado passa ao Builder e `investigationView` sai, com validação própria.

---

## 11. Questões fechadas, abertas e dependências

| Questão | Decisão incorporada | Estado | Consequência para a implementação |
|---|---|---|---|
| Organização | Três colunas; 4 áreas, 4 ferramentas; vistas centrais | Fechada | F2 e F4 |
| I-1 Seleção fora de Explorar | Não muda a área; "Inspecionar" explícito | Fechada | F3 |
| I-2 Rascunho por projeto e run | Guardado por run; nunca transferido; `catalog_sha256`; servidor confere | Fechada | F1 e F6 |
| I-3 Revisão parcial | Sim, com os limites escritos | Fechada; a completa depende de D3, D4 e D7 | F10 |
| I-4 Fato revogado | Aviso e recomendação, sem bloqueio | Fechada; a recusa depende do D7 | F10 |
| I-5 Comparação antes do D5 | Só da sessão, sem recálculo | Fechada; a persistente depende do D5 | F12 |
| I-6 Achado antes do D2 | Fluxo atual mantido, sem disfarce | Fechada; a substituição depende do D2 | F5 |
| I-7 Ponto de partida | Projeto, resumo e cidade | Fechada | F2 |
| I-8 Reordenar fontes | Fora da v1 | Fechada | F5 |
| Q4 Lotes | Independentes; desmarcar depois de criar; falha isolada | Fechada (desmarcar antes de criar fica fora) | F10 |
| Q5 Fluxograma → Builder | Só Juntar uma fonte; perguntas por ponto mantidas | Fechada | F4 e F5 |
| Q6 Cadeia | Como hoje, sem afirmar revisão | Fechada até o contrato da Etapa 5 | F8 |
| Q8 Modelos | qwen e Haiku fixos; escolha livre só onde já existe | Fechada | F7 |
| Q9 Critérios | Padrão no perfil; cópia na pergunta; ajuste no rascunho; só Salvar grava | **Parcialmente incompatível com o contrato atual:** preparar sempre usa o perfil gravado. Na v1, Preparar fica bloqueado com critérios editados, com Salvar explícito ou desfazer; o efeito completo depende do D10 | F7; B2 |
| Q10 Lugar do ciclo | A pergunta; Régua e Acervo apontam para ela | Fechada | F8 e F9 |
| Q11 Registro da escada | Persistir na Etapa 6; até lá, só a sessão; cálculo no backend | Depende do backend (D5) | F12; B4 |
| Q12 Copiar diff | `check.diffs[claim]` exato, com confirmação, sem escrita | Fechada | F8 e F12 |
| Q1, Q2, Q3, Q7 (Etapa 1) | Ponto de partida, rascunho no navegador, confirmação, preparada não enviada fica | Fechadas | F1, F2, F5 |
| Salvar × Alterar a política do modo | Duas operações explícitas e distintas; política do projeto como fonte de verdade; sincronização ao abrir e trocar de projeto; seletor só muda depois do sucesso | Fechada; a corrida restante depende do D11 | F7, F9, F11; B2 |
| Preferência `librarian.mode` | Deixa de decidir o modo; sem migração; a chave não é apagada | Fechada | F7 |
| Aberta 1 | Quando fazer o D10 (com o D1 e o D11 no B2, antes ou depois da v1) | **Aberta** | Define por quanto tempo vale o bloqueio da 3.12 |
| Aberta 2 | Quando autorizar o B3 (Etapa 5) em relação ao F10 | **Aberta** | Define por quanto tempo a revisão parcial é a única |
| Aberta 3 | Remover a chave `librarian.mode` do navegador | **Aberta** | Sem efeito funcional; só limpeza |
