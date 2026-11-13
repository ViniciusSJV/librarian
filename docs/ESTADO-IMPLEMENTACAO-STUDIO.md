# Estado da implementação do novo Librarian Studio

**Situação em 13/11/2026 (fechamento documental do F10):** as etapas **F0 a F10** da reformulação do frontend estão concluídas. F0 a F8 foram aprovadas pelo usuário nas sessões seguintes a cada uma; o **F9 (Projeto, Régua e Acervo)**, com o fechamento de 13/11 (seção 12.13), foi **aprovado no início da sessão do F10**. O **F10 (Revisão parcial e lotes)** foi implementado e verificado em 13/11/2026 (seção 13) e registrado como concluído neste fechamento documental, pedido pelo usuário; as decisões de implementação do F10 (13.7) não tiveram aprovação específica. Continuam **parciais**, por dependerem do servidor, a revisão (D3, D4, D7), a aprovação de outra versão de um comando, o Python com medição e parte do 9.10.1. **Nenhuma etapa foi commitada nem publicada.** **O F11 não foi iniciado**; segundo o usuário, ele é a próxima etapa autorizada, a começar em sessão própria, com prompt próprio (seção 19).

## Marcas usadas aqui

| Marca | Significado |
|---|---|
| **[Implementado]** | Existe no código local (sem commit), na página `crates/librarian-studio/web/index.html` ou nas baterias |
| **[Verificado]** | Conferido por bateria, teste ou execução registrada |
| **[Aprovado]** | Decisão do usuário, registrada na conversa da etapa ou na especificação |
| **[Pendente]** | Ainda não feito, ou à espera de decisão |
| **[Limitação]** | Restrição conhecida e aceita até agora |
| **[Recomendação]** | Proposta do Claude, **sem** decisão do usuário |
| **[Indisponível]** | Dado histórico que não pôde ser recuperado |

---

## 1. Finalidade e hierarquia documental

Este documento registra **o que foi efetivamente implementado e verificado** no novo frontend do Studio, etapa por etapa, com as decisões, os testes, as limitações e as pendências de cada uma. Ele existe para que uma sessão futura entenda o estado real do projeto lendo três coisas:

1. a [especificação funcional](ESPECIFICACAO-FUNCIONAL-STUDIO.md);
2. este documento;
3. o prompt da etapa seguinte.

Assim, ninguém precisa colar relatórios antigos nem depender do histórico de sessões anteriores. Este histórico consolidado substitui os relatórios de F0 a F10 como contexto operacional.

**A divisão de responsabilidades:**

| Documento | Responde a |
|---|---|
| [ESPECIFICACAO-FUNCIONAL-STUDIO.md](ESPECIFICACAO-FUNCIONAL-STUDIO.md) | O que **deve** existir: comportamento aprovado, transições, critérios de aceitação e plano F0–F13 / B1–B7 |
| Este documento | O que **já existe** no código local, como foi verificado e o que ficou pendente |
| [STUDIO.md](../STUDIO.md) | O funcionamento documentado da página **antes** da reformulação (ainda não atualizado: a atualização é do F13, com autorização própria) |
| [README do Studio](../crates/librarian-studio/README.md) | Detalhes técnicos do servidor e das rotas |
| [README das baterias](../crates/librarian-studio/tests/tela/README.md) | Como rodar as baterias de tela e o que cada uma protege |

Em caso de divergência sobre **comportamento esperado**, vale a especificação. Em caso de divergência sobre **o que está no código**, vale o código. Este documento deve ser corrigido para acompanhar os dois.

**Este documento não autoriza nenhuma implementação.** Cada etapa (F11 a F13, B1 a B7) continua exigindo prompt e autorização próprios do usuário, com relatório, diff e validação próprios.

---

## 2. Estado atual do projeto

### 2.1 Git (conferido em 13/11/2026, no fechamento documental do F10)

- **Branch:** `main`.
- **HEAD:** `9bc92b0` ("docs: atualiza documentação da etapa 4"), o mesmo da origem: `git log origin/main..HEAD` vem vazio.
- **Commit e push:** nenhum desde F0. **Todo o trabalho de F0 a F10 está só na árvore de trabalho local.** As alterações abaixo são o acumulado de todas as etapas, não só do F10.
- **Alterações locais** (`git status --short`), todas intencionais: 10 modificados e 12 não rastreados (no início do F10 eram 10 e 11; o F10 acrescentou `tela-revisao.js`).

```text
 M crates/librarian-studio/README.md                 seção do frontend: reformulação em andamento (situação até o F10), links para a especificação e este documento
 M crates/librarian-studio/tests/tela/README.md      baterias novas, contagens e a linha da tela-evidencia; F5: tela-builder e tela-4b 56; F6: tela-rascunho; F7: tela-perfil e as gravações de perfil nas cópias; F8: tela-conversa; F9: tela-projeto; F10: tela-revisao
 M crates/librarian-studio/tests/tela/etapa2c.js     F5: auxiliar que confirma "Substituir o rascunho atual?"; F7: T26 ao preparar, política devolvida, Item 5 (seção 10.11)
 M crates/librarian-studio/tests/tela/rodar.js       baterias novas na execução completa (F5: tela-builder; F6: tela-rascunho; F7: tela-perfil; F8: tela-conversa; F9: tela-projeto; F10: tela-revisao)
 M crates/librarian-studio/tests/tela/tela-4b.js     F2: mostrar a coluna Pergunta antes de medir; F5: abrir o Builder na tela estreita (+1)
 M crates/librarian-studio/tests/tela/tela-evidencia.js  F2: idem
 M crates/librarian-studio/tests/tela/tela-nomes.js  F5: auxiliar que confirma a substituição
 M crates/librarian-studio/tests/tela/tela-origem.js F5: idem
 M crates/librarian-studio/tests/tela/tela-passo6.js F2: Explorar › Inventário no lugar da aba Investigar; F7: setMode fora do gancho
 M crates/librarian-studio/web/index.html            F1 a F10
?? crates/librarian-studio/tests/tela/tela-builder.js    F5 (8 expectativas atualizadas no F6; seção 9.11; adaptada no F7, seção 10.11)
?? crates/librarian-studio/tests/tela/tela-centro.js     F4 (item 1 trocado no F5; casos 1, 7 e 8 ajustados ao F9, seção 12.11; verificação 1 ajustada ao F10, seção 13.9)
?? crates/librarian-studio/tests/tela/tela-conversa.js   F8
?? crates/librarian-studio/tests/tela/tela-estado.js     F1 (B1 ajustado ao F9, seção 12.11)
?? crates/librarian-studio/tests/tela/tela-inspecao.js   F3
?? crates/librarian-studio/tests/tela/tela-navegacao.js  F2 (nome de uma verificação no F9)
?? crates/librarian-studio/tests/tela/tela-perfil.js     F7 (+2 linhas no F8: espera pela medição, seção 11.10; 2 textos do caminho previsto no F10, seção 13.9)
?? crates/librarian-studio/tests/tela/tela-projeto.js    F9
?? crates/librarian-studio/tests/tela/tela-rascunho.js   F6 (adaptada no F7, seção 10.11; caso 1 ajustado ao F9, seção 12.11)
?? crates/librarian-studio/tests/tela/tela-revisao.js    F10
?? docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md                especificação aprovada (anterior ao F0; editada na correção do Voltar)
?? docs/ESTADO-IMPLEMENTACAO-STUDIO.md                   este documento
```

- **Página atual:** `index.html` com 8233 linhas, hash `b89f8d4117637ade` (primeiros 16 dígitos do SHA-256; conferido no fim do F10 e de novo no fechamento documental, 13/11). No fechamento do F9 eram 7676 linhas, `693f69dd2fbcda3b`; na auditoria do F9 eram 7536 linhas, `b6fcff72261582de`; ao fim do F8 eram 7171 linhas, `dbc72629b06f4b86`; ao fim do F7 eram 6774 linhas, `fa167e09dabf1540`; ao fim do F6 eram 6356 linhas, `a5c0023ba643bb50`; ao fim do F5 eram 6008 linhas, `0f47bab49561a1d2`; ao fim do F4, 5600 linhas, `a0d5140f8bfcefa2`.
- **Fechamento do F9:** mudaram a página, `tela-projeto.js`, `tela-centro.js`, `tela-builder.js`, `tela-perfil.js`, os dois READMEs e este documento; a lista de arquivos acima não mudou.
- **F10:** mudaram a página, `tela-perfil.js`, `tela-centro.js`, `rodar.js` e o README das baterias; `tela-revisao.js` é novo. **Fechamento documental do F10:** só este documento e o README do Studio (seção 14, linha 35).
- **Código Rust:** não foi alterado em nenhuma etapa. Nenhuma rota mudou.

### 2.2 Etapas

| Etapa | Situação |
|---|---|
| F0 — Linha de base | [Aprovado] concluído, sem mudança no repositório |
| F1 — Estado e persistência local | [Aprovado] [Implementado] |
| F2 — Estrutura e navegação | [Aprovado] [Implementado], com correções da revisão |
| F3 — Exploração e inspeção | [Aprovado] [Implementado], com duas correções posteriores |
| F4 — Vistas do espaço central | [Aprovado] [Implementado] |
| F5 — Evidence Builder | [Aprovado] [Implementado] (seção 8) |
| F6 — Rascunho e persistência | [Aprovado] [Implementado] (seção 9) |
| F7 — Compositor e gravação do perfil | [Aprovado] [Implementado] (seção 10) |
| F8 — Conversa e histórico | [Aprovado] [Implementado] (seção 11) |
| F9 — Projeto, Régua e Acervo | [Aprovado] [Implementado] [Verificado], auditado e fechado em 13/11 (seções 12.10 e 12.13); aprovado no início da sessão do F10; requisitos parciais por dependência do servidor |
| F10 — Revisão parcial e lotes | [Implementado] [Verificado] (seção 13); concluído no fechamento documental de 13/11; decisões de 13.7 sem aprovação específica; revisão parcial (D3, D4, D7) |
| F11 — Ferramentas | [Pendente] **não iniciado**; próxima etapa, autorizada pelo usuário para uma sessão própria (seção 19) |
| F12 e F13 | [Pendente] não iniciados |
| B1 a B7 (backend) | [Pendente] não iniciados; fora da v1 |

### 2.3 Linha de base do F0

Arquivada fora do Git em `.studio/arquivo-copias/linha-base-f0-20261111/` (a regra `/.studio` do `.gitignore` a exclui). Detalhes na seção 3.

---

## 3. F0 — Linha de base

**Objetivo na especificação (seção 10):** baterias verdes; corpos das rotas gravados para a 9.5; bytes do `project.json` antes e depois de cada fluxo que grava o perfil; inventário das chaves i18n.

**Resultado:** [Verificado] concluído sem alterar nenhum arquivo do repositório. Página de partida: hash `adc84d54ed0e0a60`, 4777 linhas.

### 3.1 Inventário da página original (antes do F1)

A página era um arquivo único: CSS, um `<script>` clássico com o dicionário de idioma e um módulo com Three.js.

| Região | Conteúdo original |
|---|---|
| Layout | Com servidor, `body.studio` montava três colunas (`340px 1fr 420px`). Até 800 px, **empilhava** cidade, lateral e conversa, sem alternador |
| Coluna esquerda | Seis abas `role="tab"` com `aria-selected` e sem `aria-current`: Projeto, Léxico, Investigar, Régua, Acervo, Mapa |
| Aba Projeto | Caminho, Procurar, Abrir, Recomeçar, Entradas, Gerar, seletor de run e `#settingsBox` (com `#allowHypothesis` e `#allowPatch` escondidos) |
| Aba Investigar | "Onde pesa" (Medir tudo, ranking, Separar, Otimizar) e Inventário |
| Aba Mapa | Encontrar, inspetor, `#overlayCard` e Abrir arquivos (`#filesCard`, fechado no Studio) |
| Centro | Cidade (canvas), `#status`, legenda e `#pathView`, o fluxograma como camada absoluta sobre a cidade, fechada por `#pathClose` ("Voltar ao mapa") |
| Direita | Conversa (`#turns`) e compositor, com o envelope e a investigação (`#gathered`), Localizar/Melhorar, critérios, LLM, Modelfile, skills e escada |
| Diálogos | `#browseDialog`, `#modelfileDialog`, `#askDialog` |

**Comportamentos observados no F0** (vários deles a especificação muda depois):

- A inicialização gravava `librarian.mode=locate` e `librarian.tab=project` mesmo sem nenhuma ação.
- Abrir um projeto carregava a run mais recente (`runs[0]`). Recarregar voltava sempre à run mais recente, não à aberta.
- `selectNode` forçava a aba Mapa.
- O fluxograma não tinha guarda `sameRun`; `askPath`, `askFacts`, `askHotFacts` e `continueTurn` também não.
- O compositor apagava o texto ao preparar.
- "Pedir esta mudança" enviava direto à escada, sem passar pelo compositor.
- O laço de Separar parava na primeira criação que falhava.
- A Régua não olhava `git.dirty` antes do clique.
- O tema só alternava entre claro e escuro, sem guardar e sem opção "sistema".
- A chave `librarian.modelfile` era gravada e **nunca lida**.
- `POST /api/performance-question` não grava nada (sonda): isso resolveu o [Validar] da 3.11 da especificação.

### 3.2 Estado no navegador (F0)

| Chave | Gravada em | Lida em |
|---|---|---|
| `librarian.lang` | `setLang` | ao iniciar |
| `librarian.mode` | `setMode`, inclusive na inicialização | `initStudio` |
| `librarian.tab` | `showTab` | `initStudio` |
| `librarian.project` | `openProject`, Recomeçar | `initStudio` |
| `librarian.llm.<lexicon\|locate\|improve\|chain\|resend>` | seletores | `choiceFor` |
| `librarian.skills` | caixas das skills | `selectedSkills` |
| `librarian.modelfile` | `mfSave` | **nunca** |

### 3.3 Defeitos reproduzidos

- **Modo global entre projetos (T25, 3.13.1 da especificação):** [Verificado] reproduzido. O modo escolhido em A aparecia em B, também depois de recarregar, e era **gravado no perfil de B** no primeiro Salvar ou em qualquer fluxo que chama `saveSettings()`. Causa: `renderProject` → `applyMode()` sobrescreve a política carregada com a chave global `librarian.mode`.
- **Casos de modo M1–M7** (cópia do renderer): M1 sem chave e perfil em Localizar mostra Localizar; M2 chave Melhorar e perfil Localizar mostra Melhorar e Salvar grava `true/true`; M3 o inverso grava `false/false`; M5 política personalizada é mostrada como Localizar e Salvar iguala os dois campos; M6 trocar de run mantém o modo e não grava; M7 o seletor muda a tela e a chave e não grava; M4 dois projetos: o modo de A passa para B e é gravado em B.
- **Gravações implícitas do perfil:** a captura registrou **18 casos de fluxo**. Quatro deles **não** gravam: abrir projeto e trocar de run, trocar o modo no seletor, Otimizar sem perguntar e Investigar um achado sem perguntar. Os outros 14 gravam o perfil, levando o modo da tela e as edições não salvas: Salvar, Salvar léxico, Gerar, `askModel` por `/api/ask` e pelo Builder, escada (duas gravações), cadeia, Separar, Conferir fatos, caminho, Sugerir léxico, Aplicar léxico (duas gravações), Exportar perfil e "Pedir esta mudança". O relatório do F0 os agrupa como os "doze fluxos" que chamam `saveSettings()`, a lista da 3.13.1 da especificação.
- **T26:** não havia confirmação; os fluxos trocavam o modo em silêncio.
- **D11:** confirmado: `PUT /api/project` grava o objeto inteiro, sem versão, a partir do `studio.project.settings` carregado na abertura.
- **Achado novo:** `collectSettings` lança erro se o texto do Léxico não for JSON válido. Como `askModel`, a escada, Gerar e os lotes chamam `saveSettings()` antes, **um léxico mal digitado bloqueia essas ações.** Lido no código, não exercitado.

### 3.4 Linha de base dos testes

| Verificação | Resultado no F0 |
|---|---|
| `cargo test --locked -p librarian-studio` | 185 aprovados (184 unitários + 1 `agnostic`) |
| `cargo test --locked --workspace` | 432 aprovados, 2 ignorados (`gabarito` e 1 do `llm-engine`) |
| `cargo build --release --locked -p librarian-studio` | sucesso |
| `node tests/tela/rodar.js` (cópias das raízes) | **310 PASSA, 0 FALHA**: cartão 37, zero 29, origem 32, nomes 12, passo2 9, 4a 30, 4b 55, passo6 36, etapa2c 45, evidência 25 |
| Inventário i18n | 812 chaves, todas com PT e EN, sem marcador divergente |

### 3.5 Corpos de requisição (critério 9.5)

**21 corpos brutos** gravados byte a byte em `captura/corpos/`, de dez sequências:

| Sequência | Rota |
|---|---|
| S1 juntar à mão, sem sugestão | `gathered-question` (`origin: null`) |
| S2 juntar, sugerir, juntar as sugeridas | `gathered-question`, seleção formato 2 |
| S3 com remover, incluir deixada de fora e fatos | `gathered-question`, `with_facts: true` |
| S4 achado do inventário | `gathered-question`, `{inventory, kind:"loop", summary}` sem seleção |
| S5 escada pelo Builder | `ladder`, `origin.kind: "ladder"` |
| S6 Otimizar | `ladder` |
| S7 Pedir esta mudança | `ladder` |
| S8 pergunta livre | `ask` |
| S9 perguntas por caminho | `path-questions` |
| S10 Separar | `gathered-question` ×3, `origin.kind: "hot"` |

A mesma captura registra também os 18 casos de fluxo, os casos de modo M1–M7, 16 sondas de escrita e o laço de Separar.

### 3.6 Preservação do material

- **Arquivado** em `.studio/arquivo-copias/linha-base-f0-20261111/`: `LEIA-ME.md` (índice), `PRESERVACAO.md`, `SHA256SUMS` (175 arquivos, conferidos contra a origem), `baterias.sh` e logs das baterias, `captura.js`, `captura/f0-captura.json`, `captura/corpos/` (21), `captura/shots/`, `i18n.js`, `i18n-inventario.json` e os logs de cargo.
- **Não arquivados:**
  - `target/` (3,7 GB; binário release `31d347afe390e42d`): refazer com `cargo build --release --locked -p librarian-studio` em `9bc92b0`.
  - `raizes/` (1,3 GB, já alteradas pela bateria `etapa2c`). As raízes **intactas** de onde todas as etapas copiaram estão em `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\df77c0d0-d1b8-4cd1-97d0-ad87f1add956\scratchpad\raizes\tela` (1420 `question.json`, 844 `send.json`; `project.json` `8c68fc0b…` no rs e `85f987ad…` no py). **[Limitação]** essa pasta fica num scratchpad temporário e pode desaparecer.
- **Origem do F0:** `…\a0aa6f80-26d4-49aa-b2a9-25e2b99e2bcb\scratchpad\f0\` (não alterada).

---

## 4. F1 — Estado e persistência local

**Objetivo na especificação:** "Módulo de navegação, rascunho, armazenamento protegido, duas abas; sem mudança visível." Aceitação: 9.2.5 e 9.2.6 em teste isolado.

### 4.1 O que foi implementado

[Implementado] Um `<script>` clássico novo em `index.html`, entre o i18n e o módulo Three.js, que expõe `localState`. Fica separado para ser testado sem a cidade. O objeto devolvido tem `tab`, `SCHEMA`, `nav`, `draftName`, `openDraft` e `storage: {read, write, remove}`.

**Navegação — `librarian.nav.v1:<key>`:**

```text
{ run, area, explore_view, center: { view, turn? }, active_turn: { <run>: turn } }
```

- Campos fora desse formato são descartados na leitura e na gravação. Por isso `mode`, `allow_*` e qualquer política nunca chegam ao navegador (5.3).
- Uma navegação ilegível volta vazia, marcada `invalid`.
- `nav.update(key, patch)` mescla o ponto guardado; `active_turn` mescla por run.

**Rascunho — `librarian.draft.v1:<key>:<run>`:**

- Cabeçalho: `schema`, `key`, `run`, `catalog_sha256`, `saved_at`, `tab`, `seq`, e o `body`. **O conteúdo do `body` fica para F5/F6.**
- `openDraft({key, run, catalog})` devolve `status()`, `update(body | null)`, `discard()`, `loadTheirs()`, `keepMine()`, `onChange()` e `close()`.

### 4.2 Comportamento

- **Restaurar (5.5, passos 1 a 3):** só aplica com esquema conhecido, o mesmo `key`/`run` e o mesmo `catalog_sha256`. Senão, o estado é `unknown-format` ou `incompatible` (motivo `identity` ou `edition`): o rascunho não é aplicado, o guardado fica visível em `kept`, e nunca é sobrescrito sem Descartar explícito.
- **Duas abas (5.7):** cada gravação leva `{tab, seq}`, e a página escuta o evento `storage`. Quando outra aba grava, esta entra em `conflict` (só leitura, sem mesclagem). Antes de gravar, o módulo confere a versão guardada, então nem um evento perdido sobrescreve a outra aba. Limpar o armazenamento em outra aba também dá `conflict`.
- **Armazenamento bloqueado ou cheio (5.8):** o rascunho fica `page-only` (motivo `unavailable` ou `quota`; na cota cheia, a última versão guardada é mantida). Sair com rascunho não vazio e não guardado aciona o aviso do navegador. Nenhuma leitura ou gravação lança exceção.
- **Servidor:** o módulo não fala com o servidor e não grava o perfil. Nada do navegador vale como registro (5.10).

### 4.3 Situação logo depois do F1

[Verificado] **Nenhum fluxo da página usava o módulo** ao fim do F1 ("sem mudança visível"): nenhuma chave `nav` ou `draft` era gravada, e as chaves do navegador eram as de antes. A navegação passou a usá-lo no F2. O rascunho ficou sem uso pela página até o F5; **o F6 passou a usá-lo** (seção 9).

### 4.4 Critérios e testes

- 9.2.5 (duas abas) e 9.2.6 (armazenamento bloqueado): [Verificado] atendidos em teste isolado do módulo, e o 9.2.6 também com o servidor real (abrir projeto e trocar de run sem erro).
- No nível do módulo, também: 9.2.1 (recarregar restaura o mesmo objeto), 9.2.3 (nenhum rascunho em outra run ou projeto) e 9.2.4 (`catalog_sha256` diferente não aplica). **[Limitação]** são testes do módulo isolado, não da página com o Builder: os critérios completos são do F6.
- Bateria nova `tela-estado.js`: 58 verificações (47 isoladas + 11 com servidor). Total: **368 PASSA**.
- Página: `adc84d54…` → `340a5c473ae77447`.

### 4.5 Limitações e escopo reservado

- Passos 4 a 8 da 5.5 (fontes inexistentes, provedor indisponível, critérios, modo necessário): **F6**. Feitos no F6, menos o passo 7 (critérios, F7) e o 5 (não se aplica); seção 9.4.
- Ligação do rascunho ao Builder: **F5/F6**. Feita no F6.
- Restaurar a run guardada ao recarregar: não feito; atribuição ambígua entre F6 e F9 (seção 16, pendência 3). [Aprovado] decidido antes do F6: fica para o F9 (seção 9.1). **Feito no F9** (seção 12.2).

---

## 5. F2 — Estrutura geral e navegação

**Objetivo na especificação:** "Cabeçalho, áreas, ferramentas, Voltar, ponto de partida; painéis movidos sem mudar comportamento." Aceitação: 9.1.1, 9.1.6, 9.1.7.

### 5.1 O que foi implementado

- **Três regiões** [Implementado]: esquerda (cabeçalho e navegação), centro (cidade, sem mudança no F2), direita (conversa e compositor, sem mudança).
- **Cabeçalho** fixo no alto da coluna esquerda: Voltar; o projeto (com o caminho completo no `title`; o clique leva a Projeto); o seletor de run com id, git curto e "com alterações"; o aviso de run com alterações, que leva à Régua.
- **Navegação** em dois grupos nomeados, com `aria-current="page"` no item ativo e um painel por vez:
  - **Áreas:** Projeto, Explorar, Régua, Acervo.
  - **Ferramentas:** Léxico (mostra "vazio" quando o léxico gravado está vazio), Configurações, Modelfiles, Abrir arquivos.
- **Explorar** com três vistas: Estrutura, Onde pesa, Inventário.
- **Funções:** `goTo(area, view)`, `goBack()` e `showTab(id)`, um adaptador dos ids antigos pela tabela 1.3 (as baterias antigas continuam funcionando por ele).

**Onde ficou cada funcionalidade antiga:**

| Antes | Depois do F2 |
|---|---|
| Aba Projeto | Projeto, sem as configurações e sem o seletor de run |
| `#settingsBox` | Ferramenta Configurações |
| Seletor `#runs` | Cabeçalho |
| Aba Investigar | Explorar › Onde pesa e Explorar › Inventário |
| Aba Mapa | Explorar › Estrutura (Encontrar, inspetor, `#overlayCard`) |
| `#filesCard` | Ferramenta Abrir arquivos (fechado no Studio, após a correção) |
| Diálogo de Modelfiles | Ferramenta Modelfiles, que abre o mesmo diálogo; continua acessível pelo compositor |
| Aba Léxico | Ferramenta Léxico |

### 5.2 Integração com o F1 e `librarian.tab`

- Só `localState.nav`, só com `area` e `explore_view`. Nenhum armazenamento novo. O rascunho não é tocado. A pilha do Voltar fica só na memória.
- **`librarian.tab`** [Aprovado]: convertida uma vez pela tabela 1.3 no primeiro projeto sem navegação guardada, e **removida só depois que a gravação da navegação dá certo**. Navegação já guardada prevalece, e então a chave antiga fica como está. Um valor sem destino na tabela não é convertido: a chave fica, com aviso no console. Se a gravação falha, a chave antiga é preservada. A página não grava mais `librarian.tab`.

### 5.3 Tela estreita, idiomas, temas

- **Até 800 px** [Implementado]: uma coluna por vez, escolhida num alternador de três posições (Navegação, Centro, Pergunta), cada uma com resumo ("Navegação: Explorar › Inventário", "Centro: Fluxograma", "Pergunta: #N"). O cabeçalho fica visível. Trocar de coluna não descarta nada. Uma ação que leva a outra coluna (abrir o fluxograma; Otimizar e Investigar, que levam à Pergunta) muda a coluna e anuncia a troca numa região `aria-live` (`#announce`).
- **Correção feita no F2:** a cidade montada com o canvas escondido ficava em branco (proporção 0 virava NaN). O `resize` passou a ignorar tamanho zero, e a cidade é reenquadrada ao aparecer.
- [Verificado] PT e EN; tema claro, escuro e troca manual, com contraste ≥ 4,5; Enter, Espaço e Tab nas áreas; contorno visível no foco; sem rolagem horizontal a 390 px. Inventário i18n: 841 chaves.

### 5.4 Decisões do usuário durante o F2

- [Aprovado] A seleção de um nó continuaria levando a Explorar › Estrutura **até o F3**, gravando o ponto no Voltar. (Substituída pela regra do F3.)
- [Aprovado] `librarian.tab` convertida e apagada, nas condições da 5.2 acima.

**Decisões de implementação apresentadas no relatório do F2, sem decisão específica registrada** (a etapa foi aprovada como um todo): trocar a vista de Explorar entra no Voltar; na tela estreita, selecionar um nó não troca de coluna; abrir um projeto pelo botão aplica a navegação guardada e põe o ponto anterior no Voltar (esta última foi **substituída** pela correção do Voltar entre projetos, seção 6.5).

### 5.5 Testes do F2

- Bateria nova `tela-navegacao.js`: 88 verificações. Total: **456 PASSA**.
- Expectativas antigas alteradas: `tela-estado` B1 passou a esperar a navegação gravada (revertido na correção, abaixo); `tela-passo6` passou a citar "Explorar › Inventário" no lugar de "Investigar" (+6/−4); `tela-4b` e `tela-evidencia` mostram a coluna Pergunta antes de medir (+1 cada).
- Página: `340a5c47…` → `9cde9b73c69b41d0`.

### 5.6 Correções da revisão do F2

Pedidas pelo usuário na revisão, dentro da autorização do F2:

1. **Vazamento de navegação entre projetos** [Implementado] [Verificado]. Causa: quando B não tinha navegação guardada, `applyKeptNav()` chamava `keepNav()` com o ponto de A ainda em memória, e o ponto de A ia para a chave de B. Com armazenamento indisponível, B também herdava a área de A. Correção: `navigation.key` diz de qual projeto é o ponto em memória; um projeto sem navegação utilizável começa em Projeto (o mesmo projeto mantém o da memória); **abrir não grava navegação**; `keepNav()` só grava quando o ponto em memória é do projeto aberto. Testes A → B → A, com e sem armazenamento (B fictício, resposta de `POST /api/project` simulada).
2. **`#announce` acessível na tela estreita** [Implementado]: o seletor que esconde a coluna passou a `#side > :not(#head):not(#announce)`. O elemento continua `sr-only`, com `aria-live="polite"`, e aparece na árvore de acessibilidade.
3. **Abrir arquivos fechado no Studio** [Implementado]: a linha `$('#filesCard').open = false` voltou ao `initStudio`, como no F0. No modo arquivo, continua aberto.
4. **Voltar entre projetos:** levantado como questão, não alterado nesta correção (decidido depois, seção 6.5).

Testes: `tela-navegacao` 88 → 103; `tela-estado` 58 → 59 (o B1 passou a conferir que abrir, trocar de run e recarregar **não** gravam navegação, e que um clique em Régua grava só a do projeto). Total: **472 PASSA**. Página: `9cde9b73…` → `312a970effc1300c`.

---

## 6. F3 — Exploração e inspeção

**Objetivo na especificação:** "Explorar unificado; inspeção como estado; regra da seleção." Aceitação: 9.1.2, 9.1.3.

### 6.1 Decisões do usuário antes de implementar

[Aprovado] Nas duas divergências entre o prompt e a especificação, valeu a especificação:

- **Clique dentro de Explorar** abre a inspeção sobre a lista atual (T1), não só a barra.
- **Inspecionar** vai para a vista de Explorar **guardada**; Estrutura só quando não há vista válida guardada.

### 6.2 O que foi implementado

| | Antes (F2, provisório) | F3 |
|---|---|---|
| Clique num nó em Explorar | Ia para Explorar › Estrutura | **T1:** inspeção sobre a lista da vista atual; o ponto anterior vai para o Voltar |
| Clique fora de Explorar | Ia para Explorar › Estrutura | **T2:** a área não muda; seleção e realce, e a barra "Selecionado: X [Inspecionar]" no cabeçalho; nada gravado; nenhum ponto no Voltar |
| Lugar da inspeção | Cartão dentro de Estrutura | Camada `#inspection` de Explorar sobre a vista atual |

- **Inspecionar** (`inspectSelected`): leva a Explorar na vista guardada, abre a inspeção sobre essa lista, grava a área de origem no Voltar e põe o foco no título da inspeção.
- **"← Voltar à lista ({vista})"** (`backToList`): fecha a inspeção e mostra a lista exatamente como estava; o foco volta ao item que abriu a inspeção. **Não grava ponto no Voltar** [Aprovado]: o Voltar seguinte leva à área de origem.
- **Voltar global:** com a inspeção aberta, volta ao ponto anterior (a lista, em T1; a área de origem, em T2). Pula pontos iguais ao que está na tela. A seleção acompanha a inspeção restaurada.
- **T3, pilha de relações:** uma relação ou uma ponta de aresta empilha um nível (`navigation.inspect`, por id); "← Um nível acima: {nome}" volta um. Nenhum dos dois grava ponto no Voltar.
- **Preservação das listas:** as listas ficam no DOM, só escondidas. Rolagem da coluna e das listas internas guardada por vista (`listScroll`). [Verificado]: Estrutura (consulta, mesmo HTML dos resultados, rolagem); Onde pesa (ranking, dobras, escolha em Separar, rolagem); Inventário (filtro, busca, termos, achados, dobras, rolagem).
- **Em Onde pesa,** o nome de uma linha do ranking inspeciona no lugar ("Inspecionar (no lugar)" da 2.2), sem botão separado.

### 6.3 Estado temporário e persistido

| Estado | Onde fica |
|---|---|
| Área e vista de Explorar | `librarian.nav.v1:<key>` (persistido por projeto) |
| Pilha de inspeção, seleção, rolagem das listas, consultas, filtros, dobras, pilha do Voltar, elemento que abriu a inspeção | **Só na memória da página** (somem ao recarregar) |

A navegação gravada continua com as mesmas cinco chaves (`active_turn, area, center, explore_view, run`), sem nada da inspeção.

### 6.4 Tela estreita, teclado, acessibilidade

- Fora de Explorar: o clique não troca de coluna nem de área e não anuncia nada; a barra fica visível no cabeçalho.
- Inspecionar leva à coluna Navegação e anuncia "Coluna mostrada: Navegação: Explorar › Estrutura › Inspeção".
- Dentro de Explorar com a cidade na frente: a inspeção abre por trás, e a barra aparece porque a inspeção não está à vista.
- A barra de seleção é um grupo nomeado "Seleção", com o texto numa região `aria-live`; a seta "←" fica escondida do leitor de tela.
- Enter num resultado abre a inspeção; "voltar à lista" devolve o foco. Os resultados de Encontrar, os acertos de `#overlayCard` e as relações da inspeção passaram a responder ao teclado.
- PT e EN; temas claro e escuro com contraste ≥ 4,5; modo arquivo inalterado (inspeção sempre visível, na ordem Encontrar, inspeção, Abrir arquivos).

**Decisões de implementação apresentadas no relatório do F3** (onde a especificação não diz; a etapa foi aprovada como um todo): a barra fica no cabeçalho; trocar de vista em Explorar fecha a inspeção (que continua guardada no ponto do Voltar); voltar a Explorar com uma inspeção aberta passa a seleção para o item inspecionado (seleção única, 1.4); Escape limpa a seleção, fecha a inspeção e limpa a consulta de Encontrar (como antes); trocar de run fecha a inspeção do grafo anterior.

### 6.5 Testes do F3

- Bateria nova `tela-inspecao.js`: 98 verificações (casos A a E, T3, modo arquivo), com cliques de mouse de verdade na cidade.
- `tela-navegacao` 103 → 102 (+15/−12), com três expectativas alteradas: o elemento conferido em Estrutura passou de `#inspector` para `#query`; as verificações do comportamento provisório deram lugar às do 9.1.2; na tela estreita, depois de selecionar, o alternador espera "Navegação: Projeto".
- Total: **569 PASSA**. Página: `312a970e…` → `a03a62525d0264e7`.
- **[Indisponível]** o log dessa rodada (569) foi sobrescrito pelas correções seguintes, que usaram a mesma pasta. O número vem do relatório do F3.

### 6.6 Correções posteriores ao F3

**a) Voltar entre projetos** [Aprovado] [Implementado] [Verificado]

- Decisão do usuário: **o Voltar funciona só dentro do projeto aberto; trocar de projeto limpa a pilha.**
- Antes: ao abrir outro projeto, a pilha era mantida (sem as inspeções). Em B, o Voltar mostrava pontos de A, aplicava as áreas de A em B e as gravava na navegação de B.
- Código (`applyKeptNav`): projeto diferente do que está em memória → `navigation.back = []`, `inspect = []`, `opener = null`; depois o projeto restaura a própria navegação guardada, ou começa em Projeto. Reabrir o mesmo projeto não muda nada. O Voltar nunca abre projetos; não há pilha global; a troca de run segue T14.
- **A especificação foi editada nesta correção** (+8/−1): esclarecimento [A] na 5.3 ("Voltar e troca de projeto [A, esclarecimento de 11/11/2026]") e nota na T18.
- Testes: cenários A a D em `tela-navegacao`, com e sem armazenamento (B fictício). `tela-navegacao` 102 → 119. README das baterias corrigido: `tela-estado` "46 + 13" (eram "47 + 11"), `tela-navegacao` 119 (dizia 88). Total: **586 PASSA**. Contraprova: a página anterior falha 9 verificações. Página: `a03a6252…` → `5989913a24b86064`.

**b) Isolamento do grafo ao trocar de projeto** [Implementado] [Verificado]

- Problema (já existia antes do F1): abrir um projeto **sem runs** não limpava o grafo do anterior. A cidade, a seleção e a barra "Selecionado" podiam mostrar um nó de A em B. E, com ou sem runs, um fluxograma aberto em A continuava visível no projeto novo.
- Correção em `openProject`:
  - `closePathView()` (função nova) quando o projeto muda;
  - no ramo sem runs: `run`, `turns` e `activeTurn` zerados, envelope (`gathered`) e investigação zerados, `#chatNote` limpo, `clearGraph()` e o status "Nenhum grafo gerado ainda para este projeto.";
  - `setLang` passou a mostrar esse texto também ao trocar de idioma.
- **O que é limpo:** grafo, nós, arestas, cena e rótulos; seleção de nó e aresta, barra, inspeção e pilha; busca, resultados e procedência; `#overlayCard`; medição (`resetInventory`); fluxograma; envelope, investigação, pergunta ativa e lista de perguntas.
- **O que é preservado:** a navegação guardada de cada projeto, o perfil (bytes iguais), o rascunho de A, idioma, tema e preferências de LLM.
- Testes: cenários A (com grafo → sem runs), B, C (volta a A, mesmo `catalog_sha256`) e D (com grafo → C fictício com runs, `catalog_sha256` de outra run). `tela-navegacao` 119 → 139. Total: **606 PASSA**. Contraprova: a página anterior falha 9 verificações. Página: `5989913a…` → `8a7592f2a5078dec`.
- **[Limitação]** B e C são **fictícios**: a resposta de `POST /api/project` é simulada, e as leituras deles são respondidas pelo projeto renderer. **Não houve teste com dois projetos reais com runs na mesma origem.**

---

## 7. F4 — Vistas do espaço central

**Objetivo na especificação (seção 10):** "Barra de vistas; Fluxograma como vista; `sameRun`." Depende de F2. Aceitação: **9.1.4** (ir e voltar entre as vistas centrais preserva a câmera, com tolerância numérica, e a seleção). Regras usadas: 1.4, 2.3, 5.3, T5, T14, T17.

### 7.1 O que foi implementado

- **Barra de vistas** `#centerBar` [Implementado]: um `<nav>` chamado "Vistas do centro", no topo do `#viewport`, sempre presente no Studio e ausente no modo arquivo.
  - **Cidade:** sempre na barra, não fecha.
  - **Fluxograma:** aparece na barra só enquanto está aberto. A aba mostra "Fluxograma: {nome curto}" (nome completo no `title`), o estado (carregando…, N pontos, N pontos; há mais, vazio, erro) e um botão × ("Fechar o fluxograma").
  - A vista da frente tem `aria-current="true"`.
- **Estado:** `navigation.center` (`'city'` ou `'path'`), com `showCenter(view, {keep})` e `renderCenter()`. O fluxograma está aberto enquanto `pathSeed != null`.
- **Cidade como base** [Verificado]: prédios, materiais, cores, luz e câmera não foram tocados. O canvas continua ocupando o centro inteiro; a barra fica por cima. Ajustes estruturais: `#status` desce 38 px no Studio (`--center-bar`), e `#pathView` começa abaixo da barra.
- **Não implementados no F4:** Evidence Builder (F5), Revisão (F10) e Comparação (F12). A barra **não** tem botões nem lugares reservados para eles (a bateria confere).

### 7.2 Abrir, trocar e fechar

- **Ir para Cidade:** esconde o fluxograma, que continua aberto na barra. **Voltar ao Fluxograma** mostra o mesmo desenho (o mesmo elemento SVG), sem novo pedido ao servidor, com semente, modo, profundidade, opções (filtros), escolhas de homônimos e rolagem preservados.
- **Abrir de novo o mesmo conteúdo** (mesma semente e mesmo modo): traz o aberto para a frente, como estava.
- **Abrir outro conteúdo:** substitui. Com escolhas de homônimos, pede confirmação ("Substituir o fluxograma?"); Cancelar não muda nada. O duplo clique num ponto (que mantém as escolhas) não pede confirmação.
- **Fechar** (× da barra ou "Fechar o fluxograma" da barra do próprio fluxograma, que antes dizia "Voltar ao mapa" e só escondia): descarta semente e escolhas; com escolhas, pede confirmação ("Fechar o fluxograma?"). Depois de fechar, o foco vai para "Cidade".
- **Respostas atrasadas** [Implementado] [Verificado]: `drawPath` numera cada desenho (`pathDrawing`) e usa `sameRun()`. Resposta de um desenho mais antigo (outra semente, profundidade ou opção, ou fluxograma fechado) ou de outra run é ignorada.
- **Troca de run (T14):** fecha o fluxograma da run anterior, **sem confirmação**, e sem gravar a navegação.
- **Troca de projeto:** fecha o fluxograma (correção do F3, mantida), sem gravar.
- **`askPath` depois de troca de run** [Implementado]: perguntas criadas que chegam depois da troca não entram na conversa da run nova, e nada mais é enviado dali. O aviso fica em `#chatNote` como erro: "A run mudou: as perguntas criadas na run {run} ficaram preparadas e não foram enviadas daqui." O corpo de `/api/path-questions` não mudou, e `askPath` continua chamando `setMode('locate')` e `saveSettings()` como antes (correção do F7). **Feita no F7** (seção 10.3): `askPath` pede o T26 e não grava mais o perfil.

### 7.3 Integração com `localState`

- Usa o campo `center: {view}` já existente em `librarian.nav.v1:<key>`. Nenhuma chave nova.
- **Grava `center.view` só em movimento da pessoa** (clique em Cidade ou na aba, abrir ou fechar pela pessoa). Abrir projeto, trocar de run e os fechamentos que eles causam **não gravam**.
- **Restauração (T17):** no mesmo projeto, vale a vista em memória; em outro projeto, a guardada, **mas só se ela estiver aberta agora**. Como o fluxograma nunca é refeito ao recarregar (2.3), um `center.view = "path"` guardado restaura a **Cidade**, sem regravar.
- O campo `center.turn` não é usado (fica para a Revisão, F10).

### 7.4 Integração com F2 e F3, tela estreita, acessibilidade

- Trocar de vista central não muda área, inspeção nem a pilha do Voltar. "← voltar à lista" não mexe na vista central.
- Clique num ponto do fluxograma segue a regra do F3 (fora de Explorar só seleciona; a seleção vale na cidade).
- Isolamento entre projetos mantido (testado com B fictício sem runs, inclusive com `center.view = "path"` guardado em B).
- **Tela estreita:** abrir o fluxograma leva ao Centro e anuncia "Coluna mostrada: Centro: Fluxograma"; o resumo da coluna segue a vista da frente (`navigation.center`); trocar de coluna preserva a vista; sem rolagem horizontal da página (a barra rola dentro de si).
- **Acessibilidade:** `<nav>` nomeado; Tab percorre Cidade → aba → ×; Enter aciona; contorno de foco pelo teclado; o diálogo prende o foco e o devolve ao botão que o abriu; contraste ≥ 4,5 nos temas claro e escuro e na troca manual.
- **Idiomas:** 17 chaves novas (`center.*`, `path.runChanged`) e `path.close` alterada, todas em PT e EN. Trocar o idioma atualiza a barra e redesenha o fluxograma aberto, mesmo atrás da cidade. Inventário i18n atual: 864 chaves, sem chave mal formada nem marcador divergente.

### 7.5 Testes do F4

- Bateria nova `tela-centro.js`: **93 verificações**. Contraprova: contra a página do fim do F3, falha 4 verificações e aborta (saída 1).
- `rodar.js` +1 bateria. Nenhuma expectativa antiga alterada.
- Total: **699 PASSA**. Página: `8a7592f2…` → `a0d5140f8bfcefa2`.
- Material: `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\464af051-9ee4-49bf-9b9b-cba66ea7a254\scratchpad\f4\` (`final.sh`, `uma.sh`, `contra-antes.sh`, `f4-completo.diff`, `log\final.log`).

---

## 8. F5 — Evidence Builder

**Objetivo na especificação (seção 10):** "Builder no centro; lista fora do compositor; achado em modo achado (fluxo atual); confirmação; sequência; 'Pedir esta mudança' pelo Builder; modo necessário no rascunho." Depende de F1 e F4. Aceitação: **9.1.5, 9.2.7, 9.2.8 e 9.5**. Regras usadas: 1.2, 1.4, 1.5, 2.3, 2.4, 3.1 a 3.10, 5.4, 7.1, 7.3, T4, T6, T8, T21.

**Situação:** [Implementado] [Verificado] em 12/11/2026. [Aprovado] pelo usuário como etapa, no início da sessão do F8 (12/11/2026). As decisões de 8.7 foram tomadas pelo Claude e apresentadas no relatório do F5; foram aprovadas com a etapa, sem decisão específica sobre cada uma.

### 8.1 O que foi implementado

- **Builder como vista central** [Implementado]: `#builderView`, uma região nomeada pelo título, sobre a cidade e sob a barra. Na barra `#centerBar`, a aba "Builder (N)" (ou "Builder: achado"), com estado ("carregando…") e um × "Fechar o Builder (o rascunho continua)". `CENTER_VIEWS` passou a ter `city`, `path` e `builder`.
- **Fechar só esconde** (1.4, 9.1.5): o × tira o Builder da barra e da frente, e o rascunho não muda. Ir para "Cidade" esconde o Builder, que continua na barra. O Builder volta pela aba, pela linha do compositor ("Abrir no Builder") e pelo cabeçalho.
- **Lista fora do compositor** (1.5, 3.10): o `#gathered` (fichas, alvos, Sugerir, "levar fatos", painel da sugestão) passou do compositor para o Builder. O compositor mostra uma linha só, `#draftLine`: "Rascunho: N fonte(s) · alvo X", "Achado do inventário: N item(ns) marcado(s), sem registro de seleção", "Rascunho: carregando …" ou "Builder: sem fontes", com "Abrir no Builder".
- **Cabeçalho** (1.2): `#headDraft`, "Rascunho: N fonte(s) · só nesta página" (ou "Rascunho: achado do inventário · só nesta página"), que abre o Builder. O título diz que o rascunho ainda não é guardado no navegador. O indicador "Não salvo" continua sem implementar.
- **Estados do Builder** (3.9): vazio ("Junte uma fonte pela inspeção ou use Otimizar"), carregando, pedindo sugestão, achado, "Já preparado como #N, sem mudanças desde então", "Alterado depois de preparar #N", e a recusa do servidor mostrada como veio (por exemplo "Seleção recusada (…); nenhuma pergunta foi criada"), com o rascunho intacto. Os estados "Restaurado", "Incompatível" e "Alterado em outra aba" são do F6.
- **Limpar rascunho** (T21): pede confirmação, e Cancelar não muda nada. Limpa as fontes, o achado, a entrada e o modo necessário; o texto do compositor e a marca "levar fatos" ficam.
- **Entrada e modo necessário** (5.4): a linha do Builder mostra a entrada ("Otimizar · …", "Pedir esta mudança · …", "Investigar achado · …") e o modo necessário ("Melhorar" ou "Localizar", "registrado neste rascunho"). Se o modo da página for outro, acrescenta "; o modo da página está em …". Nada disso grava.

### 8.2 Relação com Cidade, Fluxograma, Explorar e compositor

- **Cidade e Fluxograma:** preservados como no F4. O Builder é mais uma vista da barra, com as mesmas regras de restauração (T17): um `center.view = "builder"` guardado só volta se o Builder estiver aberto na página; ao recarregar, vale a Cidade.
- **Juntar** (T4): o botão da inspeção ("Juntar ao rascunho", antes "Juntar à pergunta") e o novo `#pathPoint` do fluxograma (um ponto clicado mostra "Juntar ao rascunho"; Q5) somam uma fonte **sem trocar a área nem a vista central**. A aba do Builder aparece na barra com o contador, sem vir para a frente.
- **Selecionar na cidade não junta nada.** A regra da seleção do F3 continua.
- **Entradas que recomeçam o rascunho** (T6, T8): Otimizar, "Pedir esta mudança" e Investigar um achado abrem o Builder na frente. Na tela estreita, mostram a coluna Centro e anunciam a troca ("Coluna mostrada: Centro: Builder (N)").
- **Compositor:** continua formulando a pergunta, o modo e quem responde; só a lista de fontes saiu dele. O caminho previsto (3.11) e T26 não foram feitos (F7).

### 8.3 Seleção no formato 2 e as quatro listas

- As listas **Incluídas** (vão ao modelo), **Removidas**, **Deixadas de fora** (com o motivo) e **Não juntadas** são lidas de `selectionOnRecord()`, a mesma função que monta o corpo. A função não foi alterada.
- **Remover** leva a fonte a `removed` (decisão `single`). **Incluir** traz de `removed`, `left_out` ou `not_joined` para `included`, pelo mesmo `toggleGathered` de antes. Nenhuma semântica nova.
- **Sem sugestão não há registro** (`origin` nulo): o Builder diz isso e mostra só Incluídas e Removidas, com a observação de que as removidas ficam só na página.
- **Orçamento** (3.4): "Conta da tela: … KB nas incluídas com tamanho conhecido, de … KB do orçamento da sugestão · N item(ns). A tela só mostra a conta; quem confere é o servidor, ao preparar."
- **"Só você vê"** (3.3): as lacunas e os candidatos (`gaps`), marcados como "Ambíguo". O painel da sugestão continua mostrando lacunas e comportamento.
- **Estado da informação por texto** (7.1): "Estrutural" (motivos do grafo), "Medido" (cobertura da sugestão) e "Aprovado por pessoa: N fato(s)".

### 8.4 Proveniência registrada × informação local do rascunho

- **"no registro: sugerida / à mão / de fora"**: a `provenance` que vai no registro da seleção e que o servidor grava. Só aparece quando há registro.
- **"chegou por: inspeção / fluxograma / Otimizar / Pedir esta mudança / sugestão / Builder / à mão · só nesta página"**: com estilo próprio e o título "Só a tela sabe por onde a fonte chegou: isso não vai ao servidor e não é registro".
- O "chegou por", a cobertura de cada fonte, a entrada, o modo necessário e `prepared_as` ficam num objeto `draft` **só na memória**, separado de `selection`. A bateria confere que o registro não ganhou campo novo (o formato 2 recusa campos desconhecidos; gravar o "chegou por" depende do D6).

### 8.5 Sugestões, sequência e respostas antigas (3.6, 3.7)

- **Confirmação** (9.2.7): uma entrada que recomeça sobre um rascunho não vazio (fontes, achado ou texto no compositor) pergunta "Substituir o rascunho atual? N fonte(s), alvo X, texto '…'", com "Substituir" e "Cancelar". Cancelar não muda nada e não faz pedido ao servidor. Rascunho vazio não pergunta.
- **Sequência:** cada pedido de sugestão ou investigação leva um número e a run. Uma resposta mais antiga é descartada com "Uma sugestão anterior foi descartada: outra foi pedida depois."; uma de outra run, com "Uma resposta pedida na run X foi descartada: a run mudou."
- **Juntar durante o carregamento** (9.2.8): fica desabilitado, com o motivo no título e escrito na inspeção ("Aguarde: … está carregando um rascunho novo"). Uma tentativa mesmo assim mostra "Nada foi juntado: …". A aba e a linha do compositor mostram o carregamento.
- **"Pedir esta mudança"** (2.4.1): o cartão não envia mais na hora. Monta um rascunho novo como Otimizar (alvo, sugestão, texto do cartão, escada marcada, modo necessário Melhorar), e a escada sai por "Perguntar" no compositor. Sem edição da pessoa, o corpo de `/api/ladder` é o mesmo do F0.
- **Achado em modo achado** (2.4.2): o fluxo atual (`investigationView`, `investigation.ticked`, origem `{inventory, kind, summary}` sem seleção) aparece no Builder com o rótulo "Achado do inventário: itens marcados, sem registro de seleção no servidor". Nunca é convertido em seleção, e o corpo é o de antes.

### 8.6 Limite entre F5 e F6

- O rascunho do F5 **não é guardado no navegador**: nenhuma chave `librarian.draft.*` é gravada (a bateria confere), e `localState.openDraft` continua sem uso pela página. Por isso o cabeçalho diz "só nesta página".
- **Recarregar ou trocar de run perde o rascunho**, como antes do F5. Na troca de run, respostas em andamento são descartadas e o Builder continua na barra, vazio. Trocar de projeto tira o Builder da barra.
- Restaurar, guardar por run, incompatíveis (5.5), duas abas no Builder (5.7) e o aviso ao sair ficam para o **F6**.
- **Depois do F6** (seção 9): o rascunho passou a ser guardado e restaurado; os três itens acima descrevem a situação ao fim do F5.

### 8.7 Decisões tomadas durante a implementação

Todas foram apresentadas no relatório do F5 e aprovadas com a etapa, sem decisão específica sobre cada uma:

| # | Decisão | Motivo |
|---|---|---|
| 1 | O modo necessário fica registrado no rascunho, mas Otimizar, o cartão e o achado **continuam chamando `setMode`**, como antes (**substituída no F7**, seção 10.3) | Sem isso a escada não aparece; a confirmação de T26 que substitui essa troca é do F7 |
| 2 | O rascunho fica só na memória; o cabeçalho diz "só nesta página" | Guardar e restaurar é do F6. Na especificação, "só nesta página" é também o texto do armazenamento indisponível (5.8): o F6 deve rever o rótulo. **Revisto no F6** (seção 9.7) |
| 3 | Juntar faz a aba do Builder aparecer na barra, sem trazê-lo para a frente; o carregamento de uma entrada também mostra a aba | T4 pede "mesma vista; contador sobe" |
| 4 | Limpar mantém o texto do compositor e a marca "levar fatos"; trocar de run também mantém "levar fatos" | Preservar os corpos de antes; o texto é do compositor |
| 5 | "Recolher/Mostrar" saiu do envelope e do achado | No centro há espaço; o recolhimento existia para economizar o compositor |
| 6 | O alvo aparece pelo nome da ficha (tipo e método), não só pelo nome curto | Homônimos têm o mesmo nome curto |
| 7 | Se o registro não puder ser lido (estado interno inconsistente que só uma bateria monta), o Builder desenha a partir do envelope, com o rótulo "sem sugestão" | A página não quebra; `selectionOnRecord()` não foi alterada. Nesse caso o rótulo fica impreciso |
| 8 | O texto do botão da inspeção virou "Juntar ao rascunho" / "Tirar do rascunho"; a nota sem alvo diz "Escolha um alvo no Builder" | 2.2 e 3.11 |

### 8.8 Critérios de aceitação

| Critério | Situação |
|---|---|
| 9.1.5 Fechar o Builder não altera o rascunho | [Verificado] atendido (`tela-builder`, caso 3) |
| 9.2.7 Toda substituição de rascunho não vazio pede confirmação; Cancelar não muda nada | [Verificado] atendido para **Otimizar, Investigar e Limpar**. **"Pedir esta mudança" usa o mesmo caminho (`restartDraft`), mas não foi testado com um rascunho não vazio** |
| 9.2.8 Juntar durante o carregamento não se perde em silêncio | [Verificado] atendido |
| 9.5.1 Corpos de `/api/gathered-question` e `/api/ladder` byte a byte | [Verificado] atendido: 21/21 corpos iguais ao F0 |
| 9.5.2 H3 e manifesto do histórico iguais à linha de base | **Não verificado separadamente** no F5. Nenhum código Rust nem rota mudou, mas a comparação com `.studio/arquivo-copias/linha-base-etapa4-20261107` não foi feita |

Testes automáticos passando não comprovam a seção 9 da especificação inteira: os critérios acima são os atribuídos ao F5, e a aceitação completa é do F13.

### 8.9 Testes do F5

- Bateria nova `tela-builder.js`: **102 verificações**. Nenhuma escrita chega ao servidor: `PUT /api/project` e `POST /api/gathered-question` são respondidos pela própria bateria, que guarda os corpos para conferir, e `/api/ladder` é abortado. O perfil fica igual em bytes, e a run não ganha perguntas.
- Baterias antigas alteradas, sem afrouxar expectativa:
  - `etapa2c.js`, `tela-nomes.js`, `tela-origem.js`: um auxiliar `substitui` confirma o diálogo "Substituir o rascunho atual?" onde a bateria encadeia Otimizar e Investigar (comportamento novo do 9.2.7).
  - `tela-centro.js`, item 1: conferia que **não** havia Builder na barra; passou a conferir que não há Revisão nem Comparação e que o Builder fica escondido sem rascunho.
  - `tela-4b.js` (+1): na tela estreita, o painel da sugestão fica no Builder (coluna Centro). A bateria abre o Builder por "Abrir no Builder" e confere que o painel está à vista; sem isso, a medida de transbordo mediria um painel escondido.
- Correções feitas durante a validação: o contraste do texto auxiliar do Builder no tema claro (3,9 → ≥ 4,5); o estado de carregamento na aba; o nome do alvo; e o desenho tolerante do item 7 de 8.7, que derrubava a `tela-4b` na primeira execução completa (`tela-4b` abortada depois de 9 verificações).
- Execução completa final: **802 PASSA, 0 FALHA** (699 do F4 + 102 + 1).
- `cargo test --locked -p librarian-studio`: 185 aprovados. `cargo test --locked --workspace`: 432 aprovados, 2 ignorados. `cargo build --release --locked -p librarian-studio`: sucesso.
- **Corpos 9.5:** 21/21 iguais ao F0. A captura foi adaptada só em S7 e no fluxo do cartão: depois do cartão, ela clica "Perguntar", porque o cartão agora só monta o rascunho.
- **Fluxos, modos e sondas:** M1–M7, as 16 sondas e o laço de Separar iguais ao F0. Os fluxos são iguais, **menos `pedir_esta_mudanca_cartao`**: no F0 o clique no cartão fazia 1 `PUT /api/project` e a escada; agora o clique não grava nada, e "Perguntar" faz os 2 `PUT` que a escada pelo compositor já fazia no F0. A eliminação dessas gravações implícitas é do F7.
- Página: `a0d5140f…` → `0f47bab49561a1d2` (6008 linhas). O diff só do F5 tem cerca de +1057/−114 linhas em 9 arquivos (`index.html` +515/−107; `tela-builder.js` com 512 linhas).
- Material: `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\efa1aaa8-d3a1-44eb-abdb-d99e44f8212b\scratchpad\f5\` (`final.sh`, `uma.sh`, `captura.js` adaptada, `f5.diff`, `base-f4\`, `log\final.log`, `log\final-1.log`).

### 8.10 Limitações conhecidas

- O rascunho se perde ao recarregar e ao trocar de run (F6). **Resolvido no F6** (seção 9).
- O "chegou por" fica só na tela (D6). A falha de leitura da medição aparece como "sem medição" (D9).
- Não foi validado se o servidor recusa uma seleção acima do orçamento (o [Validar] da 3.4).
- "Já preparado" vale só para perguntas preparadas pelo Builder sem escada; a escada não registra `prepared_as`.
- Não foram feitos no F5: juntar a declaração de um fato pelo Acervo (2.4.1 e 2.6, F9); o caminho previsto e T26 (F7); o indicador "Não salvo" (F7/F11); a Revisão (F10).

---

## 9. F6 — Rascunho e persistência

**Objetivo na especificação (seção 10):** "Rascunho ligado ao Builder: restaurar, trocar de run, incompatíveis." Depende do F5. Aceitação: **9.2** (9.2.1 a 9.2.9). Regras usadas: 1.2, 3.5, 3.9, 5.3 a 5.8, T14, T17 e a parte do rascunho de T25.

**Situação:** [Implementado] [Verificado] em 12/11/2026. [Aprovado] pelo usuário como etapa, no início da sessão do F8. As decisões de 9.1 são do usuário; as de 9.8 foram tomadas pelo Claude e aprovadas com a etapa, sem decisão específica sobre cada uma.

### 9.1 Decisões do usuário antes de implementar

[Aprovado] Duas questões levantadas antes de qualquer alteração de código, porque mudavam o comportamento e a especificação não as resolvia sozinha:

**Critérios do rascunho: ficam para o F7.**

- O F6 **não guarda nem restaura** critérios editados no rascunho.
- A parte "critérios do rascunho" da 5.4 e o **passo 7 da 5.5** ("O padrão do projeto mudou desde este rascunho") ficam pendentes para o F7, junto com o bloqueio de Preparar da 3.12.
- Motivo: hoje o campo `#criteria` do compositor é o próprio `settings.criteria` do perfil, e `askModel` e a escada ainda gravam o perfil implicitamente (pendência 5, do F7). Restaurar critérios do navegador nesse campo faria o Preparar seguinte gravar no perfil um texto vindo do navegador.
- **Não é funcionalidade esquecida nem defeito do F6:** é escopo do F7 por decisão do usuário.

**Restauração da run ativa: fica para o F9.**

- O F6 **não altera** a escolha da run ao abrir o projeto ou recarregar: continua sendo aberta **a run mais recente** (`runs[0]`).
- Os rascunhos das demais runs continuam guardados e voltam quando a pessoa escolhe a run correspondente no seletor.
- Restaurar a última run ativa fica para o **F9**, ligado a 2.1, T17 e T25. Isso resolve a atribuição da pendência 3. **Feito no F9** (seção 12.2).

### 9.2 O que foi implementado

- **Infraestrutura do F1 reaproveitada** [Implementado]: o rascunho usa `localState.openDraft`, na chave `librarian.draft.v1:<key>:<run>` que já existia. Nenhuma chave nova e nenhuma segunda infraestrutura. O módulo ganhou só duas funções:
  - `reject(motivo)`: o passo 4 da 5.5 (e um corpo ilegível) torna incompatível um rascunho que passou nos passos 1 a 3, sem aplicá-lo e sem sobrescrevê-lo;
  - `keepMine(body)`: "Manter a minha" grava o rascunho atual da página (as edições recusadas durante o conflito nunca chegaram ao módulo).
- **Na página:** `draftBody()` (serializa), `readDraftBody()` (confere a forma e lê de volta), `applyDraftBody()`, `persistDraft()`, `takeKept()`, `enterDraft()` e `leaveDraft()`, mais os estados e as escolhas em `renderDraftState()`.
- **Gravação a cada mudança** (3.5): `renderGathered()` chama `persistDraft()`, assim como a digitação do texto, a escada, "levar fatos" e o texto apagado depois de preparar ou de um cartão de desempenho. Só um conteúdo diferente do último gravado é escrito, para não disparar conflito à toa em outras abas. Rascunho vazio (sem fontes, sem achado e sem texto) não deixa chave.

### 9.3 Conteúdo do rascunho guardado

| Parte (5.4) | Guardado |
|---|---|
| Cabeçalho | `schema`, `key`, `run`, `catalog_sha256`, `saved_at`, `{tab, seq}` (do F1) |
| Builder | envelope (incluídas, na ordem), alvos, a seleção inteira (fontes com todos os campos, decisões e escopos, `suggestions`, lacunas, `suggested_from`, orçamento, `measured`), `with_facts`, a entrada, o "chegou por" e a cobertura (só o `coverage_state`) |
| Achado (I-6) | o achado como veio do servidor, com os itens marcados |
| Compositor | texto, escada ligada, modo necessário |
| Vínculo | `prepared_as: {turn, digest}` |

**Fica de fora:** a política do modo (5.3; a bateria confere que não há `allow_*` nem `mode` no rascunho); os critérios (F7, 9.1); provedor, Modelfile e skills (5.4: o modelo é preferência por tipo de ação); o painel de sugestão aberto (decisão 1 de 9.8). Nada disso é registro (5.10): pergunta, seleção e pacote continuam sendo os do servidor.

### 9.4 Restaurar (5.5)

| Passo | Implementação |
|---|---|
| 1 Esquema conhecido | Esquema ou corpo desconhecido: "Rascunho em formato desconhecido: não aplicado", com **Guardar** (fica como está, sem aplicar) e **Descartar** |
| 2 `key` e `run` | Não aplica ("Rascunho de outro projeto ou run") |
| 3 `catalog_sha256` | "Rascunho de outra edition: não aplicado. Fontes: {nomes}", só **Descartar**. Nunca aplica |
| 4 Ids no grafo | "N fonte(s) não existem nesta edition: {nomes}. O rascunho não foi aplicado.", com **Descartar** e **Recuperar só o texto do compositor** (com confirmação). Ids de cabeçalho de arquivo (`header:`) não são conferidos no grafo |
| 5 Provedor, Modelfile, skills | Não se aplica: o rascunho não os guarda (5.4) |
| 6 Hashes das sugestões | Como no F5: o servidor confere ao preparar, e a recusa aparece com o rascunho intacto |
| 7 Critérios | **Pendente para o F7** (9.1) |
| 8 Modo necessário × política | Mostrado no Builder ("…; o modo da página está em …"), sem trocar o modo e sem gravar nada |

- Um rascunho guardado que não se aplica **nunca é sobrescrito** sem Descartar: enquanto isso, o rascunho que a pessoa monta na página fica "só nesta página".
- Descartar e Recuperar o texto pedem confirmação.
- O restaurado mostra "Rascunho restaurado deste navegador ({data})" até a primeira mudança. "Já preparado como #N" sobrevive ao recarregamento, porque o registro reconstruído é o mesmo.
- **Restaurar não grava o perfil, não chama `setMode` e não regrava o rascunho** (9.2.9).

### 9.5 Troca de run, de projeto e Recomeçar (5.6, T14, T25)

- **Sair de uma run** (outra run, outro projeto, projeto sem runs, Gerar) guarda o rascunho dela e mostra no compositor "O rascunho da run X ficou guardado neste navegador." (ou "ficou só nesta página", ou, num conflito entre abas, que ficou guardada a versão da outra aba).
- **Entrar numa run** restaura o rascunho dela pela 5.5. Nunca há transferência para outra run. **O texto do compositor e a escada passam a ser de cada run** (no F5 eles passavam de uma run para outra).
- **Projeto sem runs:** o compositor fica vazio.
- **Recomeçar** (2.1): limpa a página e **não apaga** rascunhos guardados.
- A política do modo não é relida nem gravada em nenhuma dessas trocas.

### 9.6 Duas abas e armazenamento (5.7, 5.8)

- **Outra aba grava o mesmo rascunho:** esta entra em "Rascunho alterado em outra aba: só leitura até você escolher. Nada é mesclado." O texto fica só leitura; a escada, os controles do Builder e Juntar (inspeção e fluxograma) ficam desabilitados; as entradas que recomeçam, Sugerir, Limpar e Preparar recusam com o motivo escrito. Nada é gravado. As escolhas são **"Carregar a versão da outra aba"** e **"Manter a minha (sobrescrever)"**. **Nunca há mesclagem.**
- **Correção feita durante a validação:** em "Carregar a versão da outra aba", o redesenho disparado pelo próprio módulo gravava a versão desta aba antes de aplicar a outra. Um bloqueio (`draftTaking`) impede escrita enquanto a versão guardada é aplicada.
- **Armazenamento bloqueado:** "Rascunho só nesta página: o armazenamento do navegador não está disponível. Ao sair, o navegador avisa." O rascunho de uma run deixada fica na memória da página (`pageDrafts`) e volta ao entrar de novo nela; sair da página com rascunho só nela aciona o aviso do navegador.
- **Cota excedida:** "Não foi possível guardar o rascunho…"; a última versão guardada é mantida, e a mudança seguinte tenta gravar de novo.

### 9.7 Cabeçalho, compositor e vista central

- **Cabeçalho** (1.2): "Rascunho: N fonte(s) · guardado neste navegador" ou "· só nesta página" (o mesmo para o achado), e também "Rascunho alterado em outra aba" ou "Rascunho guardado não aplicado". O clique abre o Builder, onde estão as escolhas. Isso resolve a pendência 27 (o rótulo "só nesta página" do F5).
- **Compositor:** uma nota curta (`#draftNote`) com o aviso da run deixada e o estado do rascunho.
- **Builder:** o bloco `#builderDraft` com o estado e os botões. Um rascunho guardado não aplicado põe o Builder na barra.
- **Vista central (T17):** se a navegação guardada do projeto tinha o Builder na frente e o rascunho foi restaurado, o Builder volta à frente ao recarregar, sem gravar a navegação. Isso muda a regra do F5 ("ao recarregar, vale a Cidade") só para o Builder, que agora pode ser refeito; o fluxograma continua não sendo refeito.
- **"Chegou por":** o rótulo passou de "· só nesta página" a "· só local" (e o título explica que fica na tela e no rascunho deste navegador). Continua sem ir ao servidor e não é registro.

### 9.8 Decisões tomadas durante a implementação

Todas foram apresentadas no relatório do F6 e aprovadas com a etapa, sem decisão específica sobre cada uma:

| # | Decisão | Motivo |
|---|---|---|
| 1 | O painel de sugestão aberto não é guardado | As quatro listas trazem a mesma informação; o painel é uma resposta do servidor |
| 2 | Da cobertura, só o `coverage_state` é guardado | É o que a tela usa |
| 3 | Texto do compositor e escada passam a ser de cada run; a marca "levar fatos" de um rascunho vazio continua como no F5 | A 5.6 proíbe transferir o rascunho para outra run, e o texto é parte dele (5.4) |
| 4 | Sair da run durante um conflito entre abas mantém guardada a versão da outra aba, e o aviso diz isso | Não há mesclagem; a versão guardada é a única válida no navegador |
| 5 | Descartar um rascunho guardado pede confirmação | É irreversível |
| 6 | "Chegou por: X · só local" no lugar de "· só nesta página" | Com o F6 a informação também fica no rascunho do navegador |
| 7 | Para o T17, a vista central é lida diretamente da navegação guardada | Uma navegação guardada só com `center` (sem área) era substituída por "Projeto" no F2/F4; a lógica do F2/F4 não foi alterada |

### 9.9 Critérios de aceitação

| Critério | Situação |
|---|---|
| 9.2.1 Recarregar restaura o mesmo objeto | [Verificado] atendido para **a run aberta pelo recarregamento** (a mais recente) e para uma run escolhida depois de recarregar. Inclui o "já preparado", e o corpo de `/api/gathered-question` depois de recarregar é byte a byte o de antes. **Ressalva:** recarregar não volta à run em que se estava (F9, 9.1). **Resolvida no F9:** recarregar volta à run guardada, com o rascunho dela (seção 12.2) |
| 9.2.2 Trocar de run guarda; voltar restaura | [Verificado] atendido |
| 9.2.3 Nenhum rascunho em outra run | [Verificado] atendido |
| 9.2.4 Edition diferente ou fonte inexistente não se aplica | [Verificado] atendido |
| 9.2.5 Duas abas, sem mesclar | [Verificado] atendido, com as duas escolhas |
| 9.2.6 Armazenamento bloqueado | [Verificado] atendido, com o aviso ao sair |
| 9.2.7 e 9.2.8 | Preservados do F5 (`tela-builder`). A ressalva de 8.8 (9.2.7 com "Pedir esta mudança" sobre rascunho não vazio, não testado) continua |
| 9.2.9 Restaurar nunca grava o perfil | [Verificado] atendido: nenhum `PUT /api/project`, `librarian.mode` igual, `project.json` igual em bytes |

**Não atendidos no F6, por decisão do usuário ou por não se aplicarem:** passo 7 da 5.5 (F7); passo 5 (não se aplica, 5.4); passo 6 (como no F5). Testes automáticos passando não comprovam a seção 9 da especificação inteira: a aceitação completa é do F13.

### 9.10 Testes do F6

- Bateria nova `tela-rascunho.js`: **73 verificações** (casos 1 a 8: guardar e recarregar; trocar de run; outro projeto e Recomeçar; incompatíveis; duas abas; armazenamento bloqueado e cheio; a run aberta pelo recarregamento e o T17; tela estreita de 390 px, PT e EN, temas claro e escuro, contraste ≥ 4,5, teclado e as chaves `draft.*` e `head.*` com PT e EN). Nada é gravado no servidor: `PUT /api/project` e `POST /api/gathered-question` são respondidos pela bateria, e as outras escritas são abortadas e contadas.
- `tela-builder.js`: **8 verificações** com expectativas atualizadas (9 substituições de texto; duas na mesma linha), porque conferiam o comportamento do F5 que o F6 substitui: "nenhum rascunho guardado (guardar é o F6)", "· só nesta página" no cabeçalho e no "chegou por", e o título "não é guardado no navegador". Nenhuma ficou mais frouxa. Contagem igual (102). Rastreabilidade em 9.11.
- `rodar.js` +1 bateria.
- Execução completa final: **875 PASSA, 0 FALHA** (802 do F5 + 73). As 15 baterias anteriores mantiveram as mesmas contagens.
- `cargo test --locked -p librarian-studio`: 185 aprovados (184 + 1). `cargo test --locked --workspace`: 432 aprovados, 2 ignorados. `cargo build --release --locked -p librarian-studio`: sucesso (binário `08219b917fa90870`).
- **Corpos 9.5:** 21/21 iguais ao F0. **Fluxos:** iguais aos do F5 (a diferença de `pedir_esta_mudanca_cartao` continua, pendência 26). **M1–M7, sondas e laço de Separar:** iguais. A captura registra "1 erro", o `file_mode` conhecido (pendência 18).
- Raízes de teste: `send.json` 844 → 844, `question.json` 1420 → 1428 (as 8 do `etapa2c`), perfis iguais em bytes.
- Página: `0f47bab4…` → `a5c0023ba643bb50` (6356 linhas; +379/−31 em relação ao F5).
- Material: `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\6179a9b1-b16f-404f-9d6a-632b4398d997\scratchpad\f6\` (`final.sh`, `uma.sh`, `base-f5\index.html`, `f6-index-e-docs.diff`, `tela-builder-f5-do-diff.js`, `tela-builder-f5-para-f6.diff`, `log\final.log`, `log\rodar.log`).

### 9.11 Rastreabilidade de `tela-builder.js`

Auditoria feita na consolidação do F6:

- **O arquivo continua não rastreado** pelo Git (`??`), como todas as baterias novas desde o F1. O Git não guarda nenhuma versão dele.
- **As 8 expectativas novas estão na versão atual** (linhas 172, 179, 180, 186, 222, 292, 327 e 476) e **executaram e passaram** na execução final (`log\rodar.log`).
- **A versão do F5 é recuperável.** O `f5.diff` do material do F5 traz o arquivo inteiro como arquivo novo, com o blob `5c54b06`. A cópia extraída para `tela-builder-f5-do-diff.js` (512 linhas) tem exatamente o hash Git `5c54b067029dacb08636eeb0662499c506fbf607`. A comparação com a versão atual (`e8049b52…`) está em `tela-builder-f5-para-f6.diff` e mostra **só** as 8 linhas trocadas.
- **Correção do relatório do F6:** o relatório dizia "9 expectativas" e que a versão anterior não tinha sido guardada. São 8 verificações (9 substituições), e a versão anterior existe no material do F5.
- **Limitação:** a cópia e o diff ficam em scratchpads temporários, que podem desaparecer. Enquanto as baterias não forem commitadas, a única prova da versão anterior é esse material. O `f6-index-e-docs.diff` não inclui `tela-builder.js`, porque foi gerado com `git diff`, que não vê arquivos não rastreados.

### 9.12 Limitações conhecidas

- Recarregar abre a run mais recente; o rascunho de outra run reaparece ao escolhê-la (F9). **Resolvida no F9** (seção 12.2).
- Critérios do rascunho e T26 ficam para o F7. **Feitos no F7** (seção 10).
- Os testes de outro projeto continuam usando o projeto B fictício (pendência 11).

---

## 10. F7 — Compositor e gravação do perfil

**Objetivo na especificação (seção 10):** "Caminho previsto; botão por rota; Salvar e Alterar a política como operações distintas; sincronização do modo com o projeto; T26; critérios com bloqueio e Salvar explícito; texto mantido; modelos fixos; remoção das chamadas implícitas a `saveSettings()`." Depende do F5. Aceitação: **9.3, 9.4 e a tabela 3.11**. Regras usadas: 1.2, 2.1 (Gerar), 2.4.1, 2.10, 3.10 a 3.13, 5.3, 5.4, 5.5 (passos 7 e 8), 7.1, 7.3, 7.4, 8.4 (itens 3 a 6 e 9), T20, T22, T24, T25 (só o modo), T26.

**Situação:** [Implementado] [Verificado] em 12/11/2026. [Aprovado] pelo usuário como etapa, com esta consolidação, no início da sessão do F8. As decisões de 10.1 são do usuário; as de 10.8 foram tomadas pelo Claude e aprovadas com a etapa, sem decisão específica sobre cada uma.

### 10.1 Decisões do usuário antes de implementar

[Aprovado] Três questões de escopo, levantadas antes de qualquer alteração de código porque a especificação as distribui entre etapas:

1. **Gravações implícitas fora do compositor:** o F7 remove **todas**, inclusive em Gerar, Sugerir léxico, Aplicar léxico e Exportar, com o mínimo que a especificação descreve para cada uma (as três escolhas de T22 em Gerar e as de 2.10 em Exportar; o aviso "A sugestão usa o léxico gravado"; Aplicar leva a proposta ao texto e abre Salvar). Motivo: a seção 10 dá ao F7 a remoção das chamadas, e a 9.4.1 (aceitação do F7) exige que Gerar, Sugerir léxico e Exportar não gravem, embora F9 e F11 também citem esses itens. **O resto do F9 e do F11 continua com eles** (10.9).
2. **Não salvo:** o F7 marca as edições não salvas em cada lugar e mostra o indicador no cabeçalho (1.2). **A confirmação ao trocar de projeto com edições não salvas (T25) e o aviso do navegador ao recarregar (T17) ficam para o F9.**
3. **Critérios só no compositor:** o campo do compositor é o critério do rascunho. "Salvar como padrão do projeto" grava **só** `criteria`, sobre o perfil relido; as outras edições não salvas continuam não salvas. O Salvar de Configurações nunca toca nos critérios, e Configurações **não** ganhou um segundo campo de critérios (a 2.10 o prevê; fica com o F11).

### 10.2 O que foi implementado: gravação do perfil (3.13)

- **Base e edições** [Implementado]: `studio.project.settings` é o perfil lido por último do servidor. Um campo da tela com valor diferente da base (comparado pelo significado: "a,b" e "a, b" são as mesmas entradas) é uma edição não salva. Os campos editáveis estão em `SETTINGS_FIELDS`: Entradas, Léxico, Instruções, top, depth, hits, radius, linhas numeradas, aceitar diagnósticos, deixar os testes de fora, formato da resposta e inferência permitida. A política e os critérios ficam fora.
- **Só duas funções gravam o perfil** (a bateria confere que há exatamente dois `PUT /api/project` na página):
  - **Salvar** (`saveProfile`, T24): relê `GET /api/project`; se o perfil mudou fora desta página, diz "O perfil foi alterado fora desta página: {campos}"; mostra "Salvar estas alterações no perfil do projeto?" com cada campo, antes e depois; grava o perfil **relido** mais **só** as edições. Sem edições: "Nada a salvar", sem gravar (antes, Salvar sempre gravava o objeto inteiro). Falha: nada gravado, as edições ficam, a mensagem do servidor aparece. Sucesso: "Configurações salvas: {campos}". Usado por Salvar (Configurações), Salvar léxico, "Salvar e gerar", "Salvar e exportar", Aplicar léxico e "Salvar como padrão do projeto" (este, só com `criteria`).
  - **Alterar a política do modo** (`changePolicy`, T20): o seletor Localizar/Melhorar ou a confirmação do T26. Relê o perfil, mostra o modo novo e o que ele permite ("Melhorar: hipóteses e mudanças permitidas. Grava só hipótese e patch no perfil do projeto; outras alterações não salvas continuam não salvas."), grava o perfil relido com **só** `allow_hypothesis` e `allow_patch` trocados. O seletor muda **só depois do sucesso**: "Política do projeto alterada: {modo} ({o que permite})". Falha (recusa do servidor ou rede): "A política não foi alterada: {mensagem}. O modo continua {modo anterior}", e o seletor fica.
- **Releitura** (3.13.3): depois de reler, os campos que a pessoa não editou passam a mostrar o valor relido (`takeProfile`); as edições continuam edições. A janela de corrida que sobra é a da 3.13.5 (D11).
- **Removidas todas as outras gravações** (8.4.3): `collectSettings`, `saveSettings` e `setMode` saíram da página. Gerar, `askModel` (pergunta livre e Builder), escada, cadeia, Separar, Conferir fatos, perguntas por caminho, Sugerir léxico, Aplicar léxico e Exportar não gravam mais por baixo.
- **Gerar (T22) e Exportar (2.10)** com edições não salvas: diálogo com "Gerar com as configurações gravadas" / "Salvar e gerar" / Cancelar (e "Exportar o gravado" / "Salvar e exportar" / Cancelar). Sem edições, seguem direto. O diálogo `#askDialog` ganhou escolhas extras (`choices`) para isso.
- **Sugerir léxico** não grava; com o léxico editado, a nota acrescenta "A sugestão usa o léxico gravado, não as edições não salvas." **Aplicar** leva a proposta ao texto e abre Salvar com a lista de **todas** as edições não salvas; Cancelar deixa o texto como não salvo ("A proposta foi levada ao texto do léxico, sem salvar…").
- **Achado do F0 resolvido** (pendência 6): um léxico com JSON inválido na tela só é recusado pelo Salvar ("Léxico: JSON inválido…"); não bloqueia mais perguntar, preparar, a escada, Gerar nem os lotes.

### 10.3 Modo, política e T26 (3.13.3, 3.13.4, T25, T26)

- **O modo é a política gravada no perfil do projeto aberto** (`syncMode`): Localizar (`false/false`), Melhorar (`true/true`) ou, com os dois campos diferentes, **política personalizada**: nenhum modo marcado e "Política personalizada: hipótese {sim|não}, patch {sim|não}". Escolher um modo é a operação explícita, que iguala os dois campos.
- **Abrir ou trocar de projeto** sincroniza o seletor com aquele projeto; o de outro nunca se aplica (a parte de modo do T25). **Trocar de run** não relê nem grava. **Recomeçar** deixa o seletor sem modo. Sem projeto, o seletor fica desabilitado.
- **`librarian.mode`** não é mais lida nem gravada (3.13.4); a chave não é apagada. A escolha de LLM lembrada (`llm.locate`, `llm.improve`) segue o modo sincronizado.
- **Entradas que exigem um modo não o trocam mais** (pendência 25): Otimizar e "Pedir esta mudança" registram Melhorar como modo necessário; o achado, Localizar. O Builder diz "Modo necessário: … (registrado neste rascunho); a política do projeto está em …" (5.5, passo 8).
- **T26** (`ensureMode`): com a política diferente, cada ação que exige um modo pede "Esta ação usa o modo {X}; a política do projeto está em {Y}. Alterar a política para {X} e continuar?" ("Alterar e continuar"). Cancelar, ou uma falha, e a ação não acontece. Vale para: a escada (Melhorar), Preparar um rascunho com modo necessário (Builder ou achado), Separar e Conferir fatos (Localizar), perguntas por caminho (Localizar) e a cadeia (o modo escolhido no diálogo).
- **"Pedir esta mudança"** (pendência 26): o clique no cartão não grava; ao rodar a escada, o T26 pede a mudança da política uma vez e grava **só** os dois campos. Antes eram 2 gravações implícitas do objeto inteiro.
- **"Alterando a política"**: durante a gravação, o seletor e o botão de ação ficam desabilitados; depois, o foco volta ao botão que estava focado.

### 10.4 Critérios (3.12, 5.4, 5.5 passo 7)

| Conceito | Onde fica no F7 | Como muda |
|---|---|---|
| Critérios padrão | Perfil (`settings.criteria`) | Só por "Salvar como padrão do projeto" (Salvar com só `criteria`, mostrando antes e depois) |
| Critérios do rascunho | O campo `#criteria` do compositor e, enquanto diferem do padrão, o rascunho guardado: `composer.criteria = {text, base}`, com `base` = o padrão de quando foram escritos | Editados no compositor; "Diferem do padrão do projeto (não salvos)" |
| Critérios usados pela pergunta | Cópia que o servidor faz do padrão gravado, ao preparar | Não mudam pela página |

- **Bloqueio** (D10): com os critérios do rascunho diferentes do padrão, a escada e as rotas que preparam pergunta (Builder, achado, pergunta livre que chega ao modelo) não seguem: aparece "Os critérios editados ainda não podem valer só para esta pergunta (depende do D10). Escolha:" com **Salvar como padrão do projeto**, **Desfazer a edição** e **Cancelar**. O caminho previsto também avisa o D10.
- **Nenhuma gravação implícita**: Alterar a política, Gerar e preparar nunca levam os critérios editados.
- **Restaurar** (5.5, passo 7): os critérios voltam como do rascunho, nunca para o perfil. Se o padrão mudou desde que foram escritos, "O padrão do projeto mudou desde este rascunho. Padrão quando o rascunho foi escrito: “…”. Padrão atual: “…”." Se ficaram iguais ao padrão atual, passam a segui-lo.
- **Por run e por projeto**: os critérios são parte do rascunho da run (5.6); outra run e outro projeto mostram o próprio padrão.
- **Compatibilidade**: um rascunho guardado antes do F7, sem o campo `criteria`, continua válido e segue o padrão. O esquema (`DRAFT_BODY`) não mudou.
- A caixa de critérios aparece em Melhorar (o modo efetivo da pergunta) e sempre que os critérios do rascunho diferem do padrão.

### 10.5 Compositor: caminho previsto e botão por rota (3.10, 3.11)

| Rota | Texto do caminho (`#route`) | Botão |
|---|---|---|
| 1 Fontes sem alvo | "Escolha um alvo no Builder" | Desabilitado (antes: clicar mostrava uma nota de erro) |
| 2 Escada | "Escada: {modelo}; portão: {comandos}. Esta rota envia sem revisão (depende do C1)"; sem comando aprovado: "Sem comando aprovado no portão (→ Régua)" | "Rodar a escada (N) · {modelo}"; desabilitado sem comando aprovado |
| 3 Achado | "Achado do inventário: N item(ns) marcado(s), sem registro de seleção" | "Perguntar com a evidência (N) · …" (texto anterior; decisão 1 de 10.8) |
| 4 Builder | "Builder: N fonte(s)" | "Perguntar com o envelope (N) · …" (idem) |
| 5 Sem fontes | "Primeiro as regras de desempenho. Se não reconhecerem, pergunta livre com busca" | "Perguntar · …" |

- Mais duas linhas quando se aplicam: "Esta pergunta usa o modo {X}; a política do projeto está em {Y}" e o aviso do D10.
- O modo que decide a rota é o **efetivo**: o necessário do rascunho, se houver; senão, a política.
- **Texto mantido** (3.10): preparar e rodar a escada não apagam mais o texto do compositor.
- **Modelos fixos** (3.10): a escada mostra o seu modelo (Haiku) no caminho e no botão; os lotes continuam com qwen, como antes. A escolha livre continua onde já existia.

### 10.6 Não salvo (1.2, 7.4)

- `#inputsUnsaved` (Projeto), `#settingsUnsaved` e `#lexiconUnsaved`: "Alterações não salvas: {campos}", atualizados enquanto a pessoa digita.
- Cabeçalho, `#headUnsaved`: "Alterações não salvas: Entradas (Projeto), Léxico, Configurações, critérios do rascunho" (na ordem da navegação), com clique que leva ao primeiro lugar (os critérios levam à coluna Pergunta).
- **Não feito no F7** (F9, decisão 2 de 10.1): confirmar ao trocar de projeto e o aviso do navegador ao recarregar com edições não salvas. **Feitos no F9** (seções 12.3 e 12.4).

### 10.7 Textos, idiomas e arquivos

- **Página:** `web/index.html` com 6774 linhas, hash `fa167e09dabf1540`; +531/−115 linhas em relação ao fim do F6 (`a5c0023b`, 6356 linhas).
- **Chaves novas** (PT e EN, mesmas variáveis): `mode.*` (política, T26, mensagens), `field.*`, `profile.*`, `save.*`, `unsaved.*`, `head.unsaved`, `generate.*`, `export.unsavedTitle|withSaved|saveAnd`, `criteria.*`, `route.*`, `lexicon.usesSaved`, `lexicon.appliedUnsaved`.
- **Chaves alteradas:** `composer.askLadder` ("Rodar a escada"), `builder.modeDiffers` ("; a política do projeto está em …"), `settings.byMode`, `settings.criteria`, `help.mode`, `help.criteria`. **Removidas** (sem uso): `settings.saved`, `lexicon.saved`.
- **HTML:** saíram as caixas escondidas `#allowHypothesis` e `#allowPatch`; entraram `#headUnsaved`, `#inputsUnsaved`, `#settingsUnsaved`, `#lexiconUnsaved`, `#modeState`, `#modeNote`, `#criteriaNote`, `#criteriaSaid`, `#criteriaBlock`, `#route` e `#askChoices`.
- **Nada no servidor:** nenhum código Rust, rota ou corpo mudou.

### 10.8 Decisões de implementação tomadas durante o F7

Todas foram apresentadas no relatório do F7 e aprovadas com a etapa, sem decisão específica sobre cada uma. São decisões de implementação, não decisões funcionais aprovadas:

| # | Decisão | Motivo |
|---|---|---|
| 1 | As rotas 3 e 4 mantêm o rótulo "Perguntar com …", e não "Preparar" (3.11) | Elas ainda preparam e enviam na mesma ação; a Revisão que separa os dois passos é do F10. Só a escada passou a "Rodar a escada" |
| 2 | Quem responde é lido no momento do clique, antes do T26 | Achado na validação: a mudança da política traz ao compositor o LLM lembrado do novo modo, e a pergunta saía para um LLM diferente do mostrado ("Nenhum") |
| 3 | O diálogo da cadeia continua começando em Melhorar | Comportamento de antes; escolher um modo diferente da política pede o T26 |
| 4 | Com política personalizada, o compositor usa o LLM lembrado de Localizar | A especificação não diz qual |
| 5 | Os critérios do rascunho sobrevivem a Otimizar, às outras entradas que recomeçam e a Limpar; "Recuperar só o texto" (5.5, passo 4) não recupera os critérios | São parte do compositor, como o texto (decisão 4 do F5); o passo 4 fala só do texto |
| 6 | O cartão de desempenho (só leitura) não é bloqueado pelos critérios editados | Ele não cria pergunta; o bloqueio vale para o que prepara |
| 7 | Salvar sem edições não grava ("Nada a salvar") | 3.13.2: Salvar grava as edições; não há o que gravar |
| 8 | Aplicar léxico abre Salvar com **todas** as edições não salvas, não só o léxico | Salvar é uma operação só (3.13.2), e a lista mostra tudo antes de gravar |
| 9 | Na releitura, os campos não editados passam a mostrar o valor relido | Sem isso, um Salvar seguinte desfaria em silêncio o que outra aba gravou |
| 10 | O aviso do passo 7 aparece também quando os critérios do rascunho ficaram iguais ao padrão novo (e então passam a segui-lo) | O passo 7 pede o aviso sempre que o padrão mudou |
| 11 | O foco volta ao botão do seletor depois de Alterar a política | O botão fica desabilitado durante a gravação e perdia o foco |

### 10.9 O que ficou com outras etapas

- **F8:** mostrar na pergunta os critérios e a política usados, separados do padrão atual (9.3.4, 4.7) e as demais mudanças da conversa. **Feito no F8** (seção 11).
- **F9:** confirmação de T25 com edições não salvas; aviso ao recarregar com edições não salvas (T17); restaurar a última run ativa; o resumo de estado do Projeto (inclusive o item Política) e o restante de "Gerar" na área Projeto. **Feitos no F9** (seção 12).
- **F10:** a Revisão (com ela, o rótulo "Preparar" das rotas 3 e 4 e a coluna "Revisão na v1" da 3.11), os lotes com revisão e falha isolada, e as guardas `sameRun` de `askFacts` e `askHotFacts` (pendência 4).
- **F11:** a língua explícita do léxico (D8); o campo de critérios padrão em Configurações (2.10), se for mantido; as demais ferramentas secundárias.
- **Backend:** D10 (critérios do rascunho valendo só para a pergunta; até lá, o bloqueio da 3.12) e D11 (gravação do perfil sem corrida; até lá, a releitura e a janela da 3.13.5).

### 10.10 Critérios de aceitação

| Critério | Situação |
|---|---|
| 9.3.1 Editar critérios não muda o perfil (antes e depois de preparar, gerar e alterar a política) | [Verificado] atendido na `tela-perfil` (caso 7), pelos corpos dos `PUT` e pelo perfil simulado da bateria. **Ressalva:** com critérios editados, a comparação foi feita no perfil simulado, não nos bytes do `project.json` real; `librarian.json` não foi exercitado (Exportar abortado) |
| 9.3.2 Preparar bloqueado com as três opções | [Verificado] atendido |
| 9.3.3 Só Salvar grava, mostrando o que muda | [Verificado] atendido |
| 9.3.4 A pergunta mostra os critérios usados, separados do padrão | **Não atendido no F7**: é do F8. **Atendido no F8** (seção 11.9) |
| 9.4.1 Salvar só com a lista; Gerar, Preparar, Perguntar, escada, cadeia, lotes, caminho, Sugerir léxico e Exportar não gravam | [Verificado] atendido: a captura compara os bytes do `project.json` real antes e depois de cada fluxo, com uma edição não salva na tela; a `tela-perfil` confere cada fluxo. A escada com a política já em Melhorar não tem verificação isolada: no caso 6 da `tela-perfil`, a escada depois do T26 fez exatamente um `PUT`, o da política |
| 9.4.2 Alterar a política grava só os dois campos; as edições continuam não salvas | [Verificado] atendido, também em bytes no `project.json` de uma cópia (caso 2b) |
| 9.4.3 Projetos com políticas diferentes, com qualquer `librarian.mode`, sem gravar ao abrir | [Verificado] atendido (`tela-perfil` caso 1, com B fictício; captura M1–M4 no servidor real) |
| 9.4.4 Trocar de modo em A não muda B; trocar de run mantém o modo | [Verificado] atendido (perfil por projeto na `tela-perfil`; captura M4 e M6) |
| 9.4.5 Falha simulada | [Verificado] atendido (recusa do servidor simulada e rede interrompida) |
| 9.4.6 Política personalizada | [Verificado] atendido |
| 9.4.7 Separar com a política em Melhorar | [Verificado] atendido |
| 9.4.8 Perfil alterado fora | [Verificado] atendido (outra gravação simulada pela bateria entre a leitura e o Salvar ou a mudança da política) |
| Tabela 3.11 | [Verificado] atendida nas linhas 1, 2, 3, 4 e 5 e no aviso de modo diferente, **menos** o rótulo "Preparar" das rotas 3 e 4 e a coluna "Revisão na v1" (F10; decisão 1 de 10.8) |

Testes automáticos passando não comprovam a seção 9 da especificação inteira: a aceitação completa é do F13.

### 10.11 Testes do F7

- **Bateria nova `tela-perfil.js`: 150 verificações** (casos 1 a 9: abrir projetos e política personalizada; Alterar a política, também em bytes; falha simulada e perfil alterado fora; Salvar e Aplicar léxico; nenhum outro fluxo grava; T26 em cada ação; critérios do rascunho; caminho previsto; tela estreita de 390 px, PT e EN, temas claro e escuro, contraste ≥ 4,5, teclado e as chaves do F7). O perfil é respondido pela bateria, num perfil em memória por projeto com a mesma validação do servidor (`save_settings`); só o caso 2b deixa `PUT /api/project` chegar ao servidor e restaura o `project.json` depois. B (Melhorar) e C (personalizada) são projetos fictícios (pendência 11).
- **Contraprova:** contra a página do fim do F6, a `tela-perfil` faz 15 PASSA e 10 FALHA e aborta (saída 1).
- **Baterias antigas alteradas** (contagens iguais):
  - `etapa2c.js` (+19/−5): `setMode` saiu do gancho; o auxiliar `prepare` confirma o T26 quando preparar depois de Otimizar o pede (no F6, o mesmo caso gravava o perfil sem perguntar), e `voltaLocalizar` devolve a política a Localizar pelo seletor; o "Item 5" passou a conferir o botão desabilitado com "Escolha um alvo no Builder" e habilitado depois de escolher um alvo (antes: a nota de erro ao clicar).
  - `tela-builder.js` (+10/−9): `setMode` saiu do gancho e de um passo; o `PUT` simulado devolve o corpo enviado (continua sem gravar); duas verificações do caso 6 passaram a esperar "; a política do projeto está em Localizar" logo depois de Otimizar (antes, a bateria forçava o modo); a escada do cartão confirma o T26.
  - `tela-rascunho.js` (+13/−4): o mesmo no gancho e no `PUT`; `confirmaT26` nas duas preparações depois de Otimizar; a verificação "1 5.5 passo 8" espera "a política do projeto está em Localizar" (o **nome** da verificação ainda diz "modo da página").
  - `tela-passo6.js` (+1/−1): só o gancho.
  - `rodar.js` (+1/−1): a bateria nova.
- **Execução completa final:** **1025 PASSA, 0 FALHA** (875 do F6 + 150). As 16 baterias anteriores mantiveram as contagens.
- `cargo test --locked -p librarian-studio`: 185 aprovados (184 + 1). `cargo test --locked --workspace`: 432 aprovados, 2 ignorados. `cargo build --release --locked -p librarian-studio`: sucesso (binário `926aedd1424662ab`).
- **Corpos 9.5:** 21/21 iguais ao F0. A captura foi adaptada: sem `setMode`, e responde as confirmações novas (T26, Gerar e Exportar com edição não salva, seletor do modo). Sondas, laço de Separar e restauração das raízes: iguais ao F0. A captura registra "1 erro", o `file_mode` conhecido (pendência 18, não resolvida pelo F7).
- **Fluxos e modos:** todos diferem do F0, como pretendido. Gravam o perfil **só** Salvar configurações, Salvar léxico e Aplicar léxico (explícitos, com a lista: as instruções editadas; no Aplicar, também o léxico) e a cadeia e o cartão (só `allow_hypothesis` e `allow_patch`, depois do T26). Os outros 13 fluxos não gravam. Nos casos de modo, a tela mostra a política do perfil, abrir não grava, M5 não marca nenhum modo, M6 não grava, M7 grava só a política depois da confirmação, e no M4 o projeto B mostra a própria política.
- Raízes de teste: `send.json` 844 → 844, `question.json` 1420 → 1428 (as 8 do `etapa2c`), perfis iguais em bytes.
- Material: `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\652035f0-ab60-4a0a-82f3-d832e119ea0b\scratchpad\f7\` (`final.sh`, `uma.sh`, `contra-f6.sh`, `captura.js` adaptada, `base-f6\` com a página e as baterias do fim do F6, `f7-index.diff`, `f7-baterias.diff`, `f7-tela-perfil.diff`, `f7-rastreados.diff`, `log\final.log`, `log\rodar.log`, `log\contra-f6.log`, logs do cargo). **[Limitação]** o README do Studio não tem cópia do fim do F6: a mudança do F7 nele (o parágrafo da reformulação) só aparece no diff acumulado contra o HEAD.

### 10.12 Limitações conhecidas

- Edições não salvas se perdem ao trocar de projeto ou recarregar, sem aviso (F9). **Resolvida no F9** (seções 12.3 e 12.4).
- Critérios editados bloqueiam o preparo até o D10; a corrida na gravação do perfil continua até o D11.
- A política continua sendo do projeto, e alterá-la grava o perfil (D1).
- Os testes de outro projeto continuam com projetos fictícios (pendência 11).

---

## 11. F8 — Conversa e histórico

**Objetivo na especificação (seção 10):** "Linha do tempo, estados, três camadas, execuções e fatos por pergunta, critérios e política usados, Copiar diff." Depende do F2. Aceitação: **9.9 e 9.11**; o F7 deixou também o **9.3.4** (seção 10.9) e a guarda `sameRun` de `continueTurn` (8.4 item 8; pendência 4). Regras usadas: 1.4 (câmera), 4.4 (cadeia sem revisão), 4.7, 4.8, 5.3 (`active_turn`), 7.1, 7.4 (linha "Conversa"), Q6, Q10, Q12.

**Situação:** [Implementado] [Verificado] em 12/11/2026. [Aprovado] pelo usuário, com esta consolidação, no início da sessão do F9 (12/11/2026). As decisões de 11.8 foram tomadas pelo Claude e aprovadas com a etapa, sem decisão específica sobre cada uma.

### 11.1 Autorização e escopo

- [Aprovado] No início da sessão do F8, o usuário aprovou o F5, o F6, o F7 e a consolidação do F7 (pergunta feita antes de qualquer alteração, porque o prompt do F8 condicionava o início a essa aprovação) e autorizou o F8.
- Nenhuma questão de escopo foi levada ao usuário antes de implementar. Os limites seguiram o prompt: **F9** (T25, aviso ao recarregar, restaurar a última run ativa), **F10** (revisão, "Ver revisão", lotes, guardas de `askFacts` e `askHotFacts`), **F11** (ferramentas), **F12** (Comparação, "Comparar").

### 11.2 Dono de cada pergunta (4.7, 8.4 item 8)

[Implementado] Cada pergunta fica ligada ao projeto e à run de onde foi lida ou criada (`turnHome`, `homeOf`, `isHome`). Tudo o que se faz com ela vai **à run dela**, nunca à run aberta no momento da resposta:

| Caminho | Antes do F8 | No F8 |
|---|---|---|
| Perguntar (`askModel`: `/api/ask` e `/api/gathered-question`) | Sem guarda; a pergunta criada entrava na conversa aberta | A run é a do clique. Criada depois de trocar de run ou de projeto: não entra na conversa, não é enviada, e o "preparado como #N", o achado e o rascunho da run aberta não são tocados. Recusa tardia: aviso, sem tocar o Builder aberto |
| Enviar e reenviar (`sendTurn`, `resendTurn`) | `/api/send` com `studio.run` do momento: depois de uma troca, podia enviar **a pergunta de mesmo número de outra run** | O corpo leva a run da pergunta. Se a run mudou antes do envio (por exemplo durante o diálogo de contexto do Ollama), nada é enviado. Resposta que chega depois: gravada lá, não mostrada aqui |
| Reler a comparação (`refreshSends`) | Relia `/api/turns` da run aberta e procurava pelo número | Relê a run da pergunta |
| Colar resposta (`pasteAnswer`) | Run aberta | Run da pergunta |
| Decidir (`reviewView`) | Run aberta; relia a run aberta | Run da pergunta; atualiza a tela só se ela estiver aberta |
| Apagar (`deleteTurn`) | Trocava `studio.turns` pela lista devolvida, mesmo de outra run | Só atualiza se a run estiver aberta; limpa a pergunta guardada como ativa se for a apagada |
| Rodar com a mudança (`runWithChange`) | Run aberta | Run da pergunta |
| Cadeia (`continueTurn`) | Sem guarda | Run da pergunta de origem; criada depois da troca: não entra nem é enviada |
| Ver evidências (`evidenceView`) | Run aberta | Run da pergunta; mostra só se ela estiver aberta |
| Escada (`askLadder`) e caminho (`askPath`) | `sameRun` desde o F4 (caminho) e na leitura das perguntas (escada) | Mantidos; as perguntas que eles trazem ganham dono e ordem (11.8, decisão 7) |

- **Aviso** quando algo de outra run ou projeto chega: em `#chatNote`, como erro, com onde ficou ("A run mudou: a pergunta #N criada na run X ficou preparada lá e não foi enviada daqui."; com outro projeto, "run X do projeto Y"). Há um texto para cada caminho (`turn.late*`, `turn.notSentMoved`, `turn.movedBeforeAsk`, `chain.movedBefore`).
- **Troca de run:** ao começar, `loadRun` esvazia a conversa e os fatos e execuções da run anterior ("Carregando as perguntas desta run…"), para que nada da outra run apareça como desta enquanto a nova carrega.
- **Não feito no F8 (F10):** os lotes `askFacts` e `askHotFacts` e `approveHotFacts` continuam sem dono e sem `sameRun` (pendência 4). [Verificado no código] uma troca de run **durante a criação** de uma pergunta do lote faz o envio seguinte usar a run aberta com o número da pergunta criada, o mesmo risco que existia antes do F8 em todos os caminhos.

### 11.3 Concorrência

- **Duas perguntas em andamento** (`askSeq`): cada pergunta criada pela pessoa (Perguntar, cadeia, escada) leva um número de pedido. Uma mais antiga que chega depois entra **no lugar dela** na linha do tempo, mas **não toma a ativa** da mais nova; só a mais nova é guardada como ativa.
- **Envios em andamento** (`sendsInFlight`, por run e número): "Enviando: {quem}…" aparece na pergunta e continua ao sair da run e voltar; a resposta, quando chega e a run é a dela, vai para a pergunta que está na tela.
- **Ordem:** a linha do tempo segue o número das perguntas. [Verificado] o servidor lista os números como texto ("100" antes de "11"; `store.rs`, `turns`), e a página antes do F8 mostrava essa ordem e ativava como "a última" a de maior texto.
- **Escada:** a releitura periódica das perguntas (`mergeTurns`) mantém os objetos já na tela (o que a pessoa abriu, envios em andamento).

### 11.4 Linha do tempo e estados (4.7, 7.4)

[Implementado] Cada pergunta mostra, na linha de resumo, os estados que valem para ela, **calculados só do registro** (`/api/turns`, `/api/executions`), como pede a 4.7, mais dois estados de memória da página que a 7.4 prevê para a conversa:

| Estado (4.7) | Regra implementada | Origem |
|---|---|---|
| Preparação recusada | `prepare_error` | Registro |
| Preparada, não enviada | Sem `prepare_error` e sem envios | Registro |
| Aguardando resposta colada | **Não implementado** (pendência 43) | — |
| Respondida, "N aviso(s) da conferência" | Algum envio com texto; avisos = `issues` do último envio respondido | Registro |
| (sem nome na 4.7) "Enviada, sem texto na resposta" | Envios, nenhum com texto | Registro (decisão 4 de 11.8) |
| Decisões pendentes: N | Itens de `check.claims` sem revisão, em todos os envios | Registro |
| Com fato aprovado / Com recusa | Revisões `approved` / `rejected` | Registro |
| Mudança executada (N) | Execuções com `change.turn` = a pergunta | Registro (`/api/executions`) |
| Enviando: {quem}… (7.4 "Em andamento") | Envio em curso nesta página | Memória da página |
| Resposta colada não conferida (7.4 "Não salvo") | Texto na caixa de colar ainda não conferido | Memória da página |

- Vários estados valem ao mesmo tempo (decisão 4 de 11.8). Todos são escritos, não só cor (7.1). "Revisada" não é estado.
- **Estados da conversa (7.4):** "Carregando as perguntas desta run…", "Nenhuma pergunta nesta run ainda." e "Não foi possível ler as perguntas desta run: {erro}". Projeto sem run: nenhuma das três.
- **Pergunta ativa guardada por run** (4.7, 5.3): `active_turn` em `librarian.nav.v1:<key>` (campo previsto desde o F1). Grava **só** o clique da pessoa numa pergunta ou num resultado dela e uma pergunta nova feita pela pessoa; abrir o projeto, trocar de run e recarregar **não gravam**. Ao entrar numa run, a página ativa a pergunta guardada dela, se ainda existir; senão, a de maior número. Recarregar volta à pergunta guardada **da run que abrir** (restaurar a run é do F9). Ativar não toca o rascunho; a câmera voa só com o clique na pergunta (1.4; decisão 2 de 11.8).

### 11.5 A pergunta ativa: o que ela usou e as três camadas (4.7, 9.3.4)

**"Usados por esta pergunta"** [Implementado], com o rótulo da 4.1 "Gravados com a pergunta; podem diferir do perfil atual":

- **Critérios usados:** o texto e o hash gravados ao preparar (`criteria-before.md` e `criteria_sha256`, que `/api/turns` já devolve), comparados com os critérios padrão atuais do projeto ("Iguais aos…" ou "Diferentes dos critérios padrão atuais do projeto", e então o padrão atual à parte, "não usados por esta pergunta"). Os critérios do rascunho do compositor não aparecem aqui.
- **Política usada:** lida da `claims/query.json` gravada ao preparar (a mesma que cada envio entrega e que `/api/turns` devolve em `query`): `policy.allow_hypothesis`, `policy.allow_inference` e `allow_edits` (ou o antigo `allow_patch`). [Verificado no código] o servidor escreve `allow_edits` e `allow_patch` na query **só quando são verdadeiros** (`librarian-llm-engine/src/claims.rs`), então a ausência quer dizer "não". Comparada com a política atual do projeto ("A política atual do projeto é a mesma" ou "é outra: {modo}"). Pergunta em prosa: "nenhuma de claims"; sem query de claims montada: "não gravada". Nenhum contrato mudou (decisão 3 de 11.8).
- O bloco é redesenhado quando o perfil muda (Salvar, Alterar a política).
- Continuam no bloco da pergunta: origem (cadeia, caminho, Builder ou achado), busca e resultados, recusas de preparação, "O que é enviado ao LLM" e o pacote de evidências (Etapa 4). **"Ver revisão" não foi feito** (F10).

**Cada envio em três camadas** [Implementado]:

| Camada | Conteúdo |
|---|---|
| 1. O que chegou | Provedor, Modelfile, skills **com o hash** gravado no envio, modelo, estado, custo, tempo; aviso de entrada truncada; a resposta (bruta, se em claims); "O que o modelo recebeu: {arquivo} (sha256 …)" do vínculo, ou "este envio não registrou vínculo"; onde ficou guardado |
| 2. O que o Librarian conferiu | O vínculo e as conferências feitas antes do envio (`link.checks`); a conferência mecânica (ISSUES, itens com localização e citações, edições e o diff); a proposta de mudança; o aviso fixo "Referência conferida não significa que a relação semântica esteja comprovada."; resposta em prosa: "o Librarian não confere itens nem citações" |
| 3. O que você decide | Um item por linha, com Aprovar e Recusar (as decisões saíram da camada 2). **Sem "Aprovar", com o motivo escrito:** LACUNA, INFERENCE, hipótese sem mudança derivada e mudança sem vínculo ou recusada na mecânica. Recusar continua possível. Prosa: "Nada a decidir aqui" |

- A comparação de conteúdo entre envios (L2b) continua depois dos envios. H1 a H3 e H4 continuam na seção do pacote de evidências da pergunta.
- **Execuções e fatos desta pergunta** (4.7 item 4) [Implementado]: as execuções com `change.turn` = a pergunta (a mais recente aberta; o detalhe de cada uma é montado ao abrir, decisão 6 de 11.8) e os fatos de `/api/facts` desta run e desta pergunta, com o estado e o motivo. Falha de leitura de execuções e de fatos dita à parte, na pergunta. Execuções sem mudança (linha de base) não aparecem em nenhuma pergunta.
- **Ações sobre a pergunta:** reenviar, cadeia, colar resposta e apagar, como antes, agora com dono (11.2). **"Comparar" não foi feito** (F12). Nenhuma usa o rascunho.

### 11.6 Copiar diff (4.8, 9.9)

[Implementado] Na proposta de mudança da camada 2:

- **Conteúdo:** exatamente `check.diffs[claim]` como veio do servidor, sem reformatar.
- **Confirmação:** "Diff copiado: N linhas, arquivos: a, b. Nada foi aplicado ao projeto." (linhas do texto; arquivos das linhas `+++ b/…`, ou `--- a/…` se não houver).
- **Área de transferência recusada:** o diff fica num campo, selecionado e com o foco, com "Não foi possível copiar automaticamente; o diff está selecionado para copiar".
- **Sem diff derivado:** sem botão, com "Esta proposta não tem diff derivado: as edições não passaram na conferência, e não há diff para copiar." `k.patch` continua mostrado como antes e não é o diff copiado.
- **Escrita:** nenhuma. Ao lado do diff, "Aplicar ao repositório é com você: aplique, faça commit e gere uma nova run.", e o caminho do `.diff` gravado no envio continua mostrado.
- **Na Comparação:** não feito (F12).

**Fim de linha no Windows** (pendência 44):

- **Medição:** na bateria, um invólucro em `navigator.clipboard.writeText` registra o texto que a página entrega: ele é **igual em bytes** a `check.diffs[claim]`, com LF. Lido de volta com `navigator.clipboard.readText()`, no Chrome sem janela deste Windows, o texto vem com **CRLF** (394 caracteres contra 384 de um diff de 10 linhas). Sem os CR, é igual ao diff.
- **Onde ocorre a conversão:** depois da página, na camada de área de transferência do navegador e do sistema. [Hipótese, não verificada] o Chromium no Windows converte LF em CRLF ao gravar o texto no formato de texto do sistema. Não foi testado se a troca acontece ao gravar ou ao ler, nem em outro navegador ou sistema.
- **Impacto prático:** [Hipótese, não testada] colar o diff num arquivo no Windows pode deixá-lo com CRLF, e `git apply` pode recusar os trechos se o arquivo do projeto tiver LF. Nenhuma aplicação foi testada.
- **Por que o 9.9.1 ficou parcial:** o critério pede o texto copiado igual em bytes a `check.diffs[claim]`. O texto entregue pela página é igual; o texto que a pessoa obtém da área de transferência neste Windows não é.
- **A quem pertence a correção:** não decidido. A troca não vem do frontend; uma correção poderia ser do frontend (por exemplo, outra forma de entregar o texto) ou só de documentação e orientação. Sem etapa atribuída.

### 11.7 Textos, idiomas, arquivos

- **Página:** `web/index.html` com **7171 linhas**, hash `dbc72629b06f4b86`; +477/−85 linhas em relação ao fim do F7 (`fa167e09`, 6774 linhas). Um hash intermediário (`2d1e9ba0`, 7166 linhas) aparece no primeiro log da execução completa, anterior à correção da 11.10.
- **Chaves novas** (PT e EN, mesmas variáveis): `turn.where*`, `turn.movedBeforeAsk`, `turn.late*`, `turn.notSentMoved`, `turn.idLine`, `chain.movedBefore`, `turns.*`, `state.*`, `used.*`, `records.*`, `layer.*`, `decide.*`, `review.why.*`, `paste.unchecked`, `diff.*` e `reply.received`, `reply.receivedUnlinked`, `reply.noLink`. **Alterada:** `reply.link` (passou a citar as conferências, `{checks}`). **Removidas** (sem uso): `turn.criteria`, `turn.noCriteria`, `turn.notSent`.
- **CSS:** `.state-tag`, `.used`, `.records`, `.layer`, `.layer-title`, `.layer-warning`, `.copy-diff-row`, `.copy-note`, `.copy-box`, `.turns-state`, `.turn-id`; o texto auxiliar da conversa usa `--text-secondary` (contraste ≥ 4,5 no tema claro).
- **Atributos de dados** para a tela e as baterias: `data-turn` em cada pergunta, `data-send` em cada envio, `data-claim` e `data-kind` em cada decisão, `data-state` em cada estado.
- **Nada no servidor:** nenhum código Rust, rota ou corpo mudou.
- **Arquivos alterados no F8:** `web/index.html`, `tests/tela/tela-conversa.js` (novo), `tests/tela/rodar.js` (+ a bateria), `tests/tela/tela-perfil.js` (+2 linhas, 11.10). Conferido contra a cópia do fim do F7 guardada no material: nenhum outro arquivo do repositório mudou na sessão do F8.

### 11.8 Decisões de implementação tomadas durante o F8

Todas foram apresentadas no relatório do F8 e aprovadas com a etapa (12/11/2026), sem decisão específica sobre cada uma. São decisões de implementação, não requisitos funcionais aprovados:

| # | Decisão | Motivo |
|---|---|---|
| 1 | Os avisos de algo que chegou para outra run ou projeto ficam em `#chatNote`, como erro | O mesmo lugar do aviso do `askPath` (F4); as outras notas são reescritas por `refreshComposer()` |
| 2 | Uma pergunta feita pela pessoa vira a ativa e é guardada por run, mas a câmera não voa; voa só no clique na pergunta | 1.4: a câmera muda só por pedido explícito, entre eles o clique numa pergunta. Antes, preparar voava |
| 3 | A política usada é lida da `claims/query.json` gravada, sem mudar contrato; a ausência de `allow_edits`/`allow_patch` quer dizer "não" | O servidor só escreve esses campos quando verdadeiros; nenhuma rota devolve o `claims/context.json` |
| 4 | Vários estados valem ao mesmo tempo; "Decisões pendentes" conta todos os itens sem revisão, também de envios recusados na mecânica; "Enviada, sem texto na resposta" é um estado a mais, do registro | A tabela da 4.7 não diz se os estados se excluem nem nomeia o envio sem texto |
| 5 | A resposta colada ainda não conferida fica só na memória da página, por run e pergunta, sobrevive à troca de run e **não** entra no indicador de não salvo do cabeçalho | A 7.4 a dá como "Não salvo" da conversa; o indicador do cabeçalho (F7) lista ferramentas e critérios |
| 6 | O detalhe de cada execução da pergunta é montado só quando aberto (a mais recente vem aberta) | Uma pergunta tem até dezenas de execuções; ver 11.12 |
| 7 | `askLadder` relê as perguntas mantendo os objetos da tela e só ativa a pergunta nova se nenhuma mais nova foi feita; `askPath` dá dono e ordem às perguntas que cria; `approveHotFacts` passa a ordenar as perguntas relidas | Ordem e dono nos caminhos que já tinham guarda; **sem** guarda nova nos lotes (F10) |
| 8 | Ao entrar numa run, a conversa fica vazia com "Carregando…" até as perguntas dela chegarem; fatos e execuções da run anterior são esquecidos | 4.7: a conversa é da run ativa |
| 9 | Recusa de preparação que chega depois da troca de run vira aviso, sem tocar o Builder aberto | O Builder aberto é de outra run |
| 10 | Apagar a pergunta guardada como ativa limpa a guarda (`active_turn` daquela run) | Senão a run abriria procurando uma pergunta inexistente (cai na de maior número de qualquer forma) |

### 11.9 Critérios de aceitação

| Critério | Situação |
|---|---|
| 9.3.4 A pergunta mostra os critérios usados, separados do padrão atual | [Verificado] atendido: critérios e política usados, cada um com o atual à parte; com o padrão e a política atuais trocados (perfil respondido pela bateria), a pergunta continua mostrando os dela |
| 9.9.1 Texto copiado igual a `check.diffs[claim]` em bytes | **Parcialmente atendido:** [Verificado] o texto entregue pela página é igual em bytes; lido de volta da área de transferência no Windows, vem com CRLF (11.6) |
| 9.9.2 A confirmação mostra linhas e arquivos | [Verificado] atendido |
| 9.9.3 Nenhum arquivo do projeto nem de `.studio` muda | [Verificado] atendido: fotografia (caminho, tamanho, data) de `.studio` da raiz de teste e do projeto `renderer` antes e depois, iguais, e nenhuma requisição de escrita |
| 9.9.4 Sem área de transferência, o diff fica selecionado | [Verificado] atendido (escrita recusada simulada) |
| 9.11.1 Perguntas anteriores à Etapa 4, pacotes de formato 1 e 2 | [Verificado] atendido na run `20261104-020244` (uma de cada: sem pacote, formato 1, formato 2, com pergunta, pacote e envios em três camadas). **Ressalva:** seleções de formato 1 e 2 e o fixture `tests/evidence-v1` não foram exercitados nesta bateria (a `tela-evidencia` cobre o pacote) |
| 9.11.2 Execuções sem `change`, fatos revogados e desatualizados | [Verificado] atendido: execuções sem mudança em nenhuma pergunta; um fato desatualizado e um revogado (acervo respondido pela bateria, com os estados trocados) |
| 4.7 Estados derivados | [Verificado] atendido para as 139 perguntas da run, recalculados pela bateria; **"Aguardando resposta colada" não atendido** (pendência 43) |
| 4.7 Três camadas; LACUNA e INFERENCE sem "Aprovar", com o motivo | [Verificado] atendido. LACUNA com dados reais; INFERENCE e hipótese sem mudança com registros alterados pela bateria (não há INFERENCE nas runs de teste) |
| 4.7 Execuções e fatos por pergunta | [Verificado] atendido |
| 4.7 Pergunta guardada por run; ativar não voa nem toca o rascunho | [Verificado] atendido para a guarda e o rascunho; a câmera não foi medida |
| 4.7 "Ver revisão", "comparar" | **Não atendidos no F8:** F10 e F12 |
| 7.4 Conversa: vazio, falha de leitura, em andamento, não salvo | [Verificado] atendidos |
| 8.4 item 8: `sameRun` em `continueTurn` | [Verificado] atendido (com dono da pergunta, 11.2); `askFacts` e `askHotFacts`: F10 |

Testes automáticos passando não comprovam a seção 9 da especificação inteira: a aceitação completa é do F13.

### 11.10 Correções feitas durante a validação do F8

- **Política usada:** a primeira versão dizia "patch não registrado" quando `allow_edits` faltava na query; a bateria mostrou o caso, e a leitura passou a tratar a ausência como "não" (decisão 3 de 11.8).
- **Contraste:** o texto auxiliar da conversa e o estado "Com fato aprovado" ficavam abaixo de 4,5 no tema claro (4,36 e 2,82); passaram a `--text-secondary` e a borda verde com texto normal.
- **`tela-perfil.js` abortou** na primeira execução completa (99 PASSA, 1 FALHA: `Cannot read properties of null (reading 'rows')`). A função `cartaoComMudanca` lia `__t.hot.rows` sem esperar a medição carregar; sozinha, a bateria passava (150). A hipótese é que a conversa mais pesada atrasou a medição. Correções: (a) na página, o detalhe das execuções passou a ser montado ao abrir (decisão 6); (b) na bateria, **+2 linhas**: a mesma espera por `__t.hot.rows` que ela já usa nas linhas 144 e 152. Nenhuma expectativa mudou; a contagem continuou 150.
- **Porta ocupada:** a segunda execução completa não rodou as baterias (`rodar.js` saída 2: "A porta 8841 já está em uso"), porque um servidor de uma medição de tempo ficou aberto. Ele foi encerrado e a execução completa foi refeita (a terceira, que vale).

### 11.11 Testes do F8

- **Bateria nova `tela-conversa.js`: 102 verificações**, casos 1 a 9:
  1. linha do tempo na ordem dos números; estados das 139 perguntas da run `20261104-020244` recalculados pela bateria a partir de `/api/turns` e `/api/executions`; perguntas sem pacote e com pacote de formato 1 e 2; execuções e fatos por pergunta; fatos desatualizado e revogado (acervo respondido pela bateria);
  2. política e critérios usados, com o perfil atual igual e trocado (respondido pela bateria); prosa e query de claims ausente (registros alterados pela bateria);
  3. três camadas; decisões só na camada 3; LACUNA, INFERENCE e hipótese sem mudança sem "Aprovar";
  4. Copiar diff: bytes entregues, leitura de volta (com a NOTA do CRLF), confirmação, fotografias de `.studio` e do projeto, teclado, área de transferência recusada, proposta sem diff;
  5. pergunta ativa guardada por run, troca de run, recarregar sem regravar a navegação, rascunho intocado;
  6. respostas atrasadas (pedidos segurados pela bateria até a troca): perguntar, enviar, sair e voltar com envio em curso, duas perguntas em andamento, cadeia, decidir, apagar, rodar com a mudança e outro projeto (fictício);
  7. estados carregando, vazio e falha de leitura; falhas de leitura de execuções e fatos; resposta colada não conferida, que sobrevive à troca de run, e recusa ao conferir;
  8. tela de 390 px, PT e EN, temas claro e escuro, contraste ≥ 4,5, frases proibidas da 7.1 nos rótulos da conversa, chaves do F8 em PT e EN;
  9. sem erros de página, perfil igual em bytes, perguntas do servidor iguais, nenhuma escrita abortada.
- **Nada é gravado no servidor:** as escritas que a bateria precisa ver são respondidas por ela; nenhuma chega ao servidor.
- **Contraprova:** contra a página do fim do F7, a bateria **aborta na preparação** (0 PASSA, saída 1), porque espera estados que o F7 não tem. É uma contraprova grosseira: não mostra quais verificações falhariam uma a uma.
- **Baterias antigas alteradas:** `tela-perfil.js` (+2 linhas, 11.10) e `rodar.js` (+ `tela-conversa`). Nenhuma expectativa antiga mudou.
- **Execução completa final:** **1127 PASSA, 0 FALHA** (1025 do F7 + 102), `rodar.js` saída 0 em 1025 s. As 17 baterias anteriores mantiveram as contagens.
- `cargo test --locked -p librarian-studio`: 185 aprovados. `cargo test --locked --workspace`: 432 aprovados, 2 ignorados. `cargo build --release --locked -p librarian-studio`: sucesso (binário `faece1154ff5bf6b`). Rodados no início da sessão, antes de qualquer alteração; nenhum código Rust mudou depois.
- **Corpos 9.5:** 21/21 iguais ao F0. **Contra o F7** (captura do F7 × captura do F8): casos de modo, sondas, laço de Separar e restauração das raízes iguais; **um fluxo difere**, `pedir_esta_mudanca_cartao`, e só no texto de `#llmNote` fotografado ("Escada em andamento…" no lugar da nota do provedor). Requisições, `PUT`, diálogos e bytes do perfil desse fluxo são idênticos aos do F7. [Hipótese] a fotografia é tirada antes de a escada terminar de redesenhar a conversa da run `20261104-014710` (668 perguntas), mais lenta no F8 (11.12).
- `git diff --check`: limpo.
- Raízes de teste: `send.json` 844 → 844, `question.json` 1420 → 1428 (as 8 do `etapa2c`), perfis iguais em bytes.
- Material: `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\1b1a4762-b094-447e-ab02-84372fb6c170\scratchpad\f8\` (`final.sh`, `uma.sh`, `contra-f7.sh`, `captura.js`, `mede.js`, `mede2.js`, `base-f7\` com a página, as baterias e os documentos do fim do F7, `f8-index.diff`, `f8-baterias.diff`, `tela-conversa.js`, `log\final.log` (a execução que vale), `log\final-1.log`, `log\final-2.log`, `log\rodar.log`, `log\contra-f7.log`, `log\fluxos-contra-f7.log`, `log\cargo.log`, `consolidacao\` com as cópias dos documentos antes desta consolidação).

### 11.12 Desempenho

| Medida | F7 | F8 | Natureza |
|---|---|---|---|
| Redesenho da conversa, run `20261104-014710` (668 perguntas) | 178, 165, 173 ms | 276, 245, 259 ms | **Medição**: três chamadas seguidas de `renderTurns()`, uma página de cada versão, Chrome sem janela nesta máquina, depois do detalhe das execuções montado ao abrir |
| Perguntas na tela depois de escolher essa run | 1885 ms | 2044 ms | **Medição**, uma amostra de cada |
| Redesenho, run `20261104-020244` (139 perguntas) | 45, 37, 38 ms | 75, 62, 65 ms | **Medição**, duas amostras de cada (a segunda igual em ordem de grandeza) |
| A página redesenha a conversa mais vezes ao abrir uma run (no início, ao chegar as perguntas, e de novo se mudam fatos ou execuções) | — | — | **Leitura do código**, não medida |
| A conversa mais pesada atrasou a medição e expôs a espera que faltava na `tela-perfil`; e explica a diferença do fluxo do cartão | — | — | **Hipótese**, não verificada |

### 11.13 Limitações conhecidas

- "Aguardando resposta colada" não é derivável (pendência 43).
- Copiar diff no Windows: CRLF ao ler a área de transferência (pendência 44).
- Conversa mais lenta em runs grandes (pendência 45).
- Os lotes continuam sem dono nem `sameRun` até o F10 (pendência 4).
- Uma colada que falha depois da troca de run usa o aviso de envio ("o envio da pergunta #N … falhou"), não um texto de colar (pendência 47).
- O caso 6a da `tela-conversa` só confere "nada é enviado" se o seletor de quem responde tiver Claude CLI; nesta máquina tem (pendência 48).

---

## 12. F9 — Projeto, Régua e Acervo

**Objetivo na especificação (seção 10):** "Resumo de estado, Gerar sem Salvar implícito, links para a pergunta, T13, T25." Depende de F2, F7 e F8. Aceitação: **9.10**. Regras usadas: 2.1, 2.4.1 (fatos do Acervo), 2.5, 2.6, 5.3 (`run` na navegação), T9, T13, T14 e T22 (a run nova), T17 e T25; pendências 3, 29 e 35.

**Atualização (13/11/2026, início da sessão do F10):** [Aprovado] o F9, com o fechamento (12.13) e as decisões das pendências 57, 66, 67 e 68, foi aprovado pelo usuário. O parágrafo seguinte é o registro da auditoria, mantido como estava.

**Situação:** [Implementado] [Verificado] em 12/11/2026 (sessão `8f80f82a`). **Auditado em 13/11/2026** contra a especificação, o código, os diffs e os logs (12.10). **Concluído com ressalvas e aguardando a avaliação do usuário:** os requisitos centrais estão atendidos; há dois defeitos de gravidade média, requisitos parciais e decisões de implementação sem aprovação. As decisões de 12.1 são do usuário; as de 12.8 foram tomadas pelo Claude e **não foram aprovadas**.

**Fechamento (13/11/2026, sessão `6e1931b3`):** M1, M2 e as ressalvas da auditoria corrigidos, com testes de regressão; as decisões do usuário sobre 12.8 aplicadas; requisitos parciais registrados. O texto de 12.1 a 12.12 é o da implementação e da auditoria, mantido como histórico; o que mudou está em **12.13**. Aguarda a avaliação do usuário.

### 12.1 Autorização e decisões do usuário antes de implementar

[Aprovado] No início da sessão, antes de qualquer alteração:

- **F8 e a consolidação do F8 aprovados.** O prompt do F9 condicionava o início a essa aprovação. Isso resolve a pendência 46.
- **Escopo: tudo o que a especificação atribui ao F9.** O prompt destacava T25, T17 e a run guardada; a especificação também dá ao F9 o resumo de estado (2.1), a Régua (2.5) e o Acervo (2.6). Foram feitos todos.
- **T25 com três escolhas:** "Abrir sem salvar", "Salvar e abrir" (o Salvar explícito, com a lista dos campos) e Cancelar. O mesmo vale para Limpar ("Limpar sem salvar", "Salvar e limpar").
- **Aviso do navegador (T17):** só as edições não salvas do perfil (Entradas, Configurações, Léxico), além do aviso que já existia para rascunhos guardados só na página (F1, F6). A resposta colada não conferida (F8) **não** aciona o aviso.

### 12.2 Run guardada por projeto (2.1, 5.3, T17, T25)

[Implementado] A navegação do projeto (`librarian.nav.v1:<key>`) passou a guardar `run`, campo previsto desde o F1.

- **Quem grava:** só uma escolha da pessoa: o seletor de run (`keepRun` antes de `loadRun`), Gerar (a run nova, T22) e "Ir à origem" de um fato em outra run (T13). **Abrir e recarregar não gravam nada** (`keptRun` só lê).
- **Ao abrir ou recarregar:** a run guardada, se ainda existir; senão, a mais recente. Uma run guardada que não existe mais abre a mais recente, com "A run guardada para este projeto ({run}) não existe mais: aberta a mais recente.", e a navegação **não é regravada**.
- **Armazenamento indisponível:** ao abrir de novo o mesmo projeto, vale a run da memória; outro projeto abre a mais recente.
- **Com a run:** a pergunta ativa dela (`active_turn`, F8) e o rascunho dela (F6) voltam; nada de outra run ou projeto se mistura. Uma resposta atrasada da run restaurada não entra na conversa da run escolhida depois (guardas do F8).
- **Nada grava o perfil:** nenhum `PUT /api/project` ao abrir, recarregar ou trocar de run; o modo continua o da política gravada.

### 12.3 T25: sair de um projeto com edições não salvas

[Implementado] `leaveProject` roda **antes** de `POST /api/project` (abrir) e antes de limpar a página (Limpar). Sem edições não salvas do perfil, segue direto. Com elas, o diálogo diz o projeto e os campos e oferece:

| Escolha | Efeito |
|---|---|
| Cancelar | Nada muda: o projeto, as edições e o indicador ficam; a página diz "Nada mudou: o projeto {nome} continua aberto, com as alterações não salvas." |
| Abrir sem salvar | Troca; a nota diz "Alterações não salvas descartadas de {projeto}: {campos}." Nada é gravado |
| Salvar e abrir | Abre o Salvar explícito (3.13.2, com a lista). Troca **só** se o Salvar gravou todas as edições. Salvar cancelado: nada muda. Salvar recusado: "O projeto não foi trocado: o Salvar falhou ({mensagem}). As alterações continuam não salvas." |

- **Caminhos cobertos:** Abrir, Enter no campo do caminho, Procurar › Escolher e Limpar. O restabelecimento ao carregar a página não tem edições a proteger.
- **Correção feita na validação:** o Enter que abria o diálogo também apertava o botão que o diálogo focava ("Salvar e abrir"). O `keydown` passou a chamar `preventDefault()`.
- **Critérios do rascunho** não entram no T25: ficam guardados com o rascunho da run (5.4; `draftBody` guarda os critérios mesmo num rascunho sem fontes nem texto). Ver a decisão 5 de 12.8.

### 12.4 T17: aviso ao recarregar ou sair

[Implementado] O ouvinte `beforeunload` da página, que já avisava pelos rascunhos guardados só na página (F6), passou a avisar também quando `unsavedFields()` não está vazio. O ouvinte do `localState` (F1) continua cobrindo o rascunho aberto não guardado.

- O texto do aviso é o do navegador: a página não escolhe as palavras. O aviso **não guarda nada**; quem aceita perde as edições do perfil (a bateria confere isso).
- Sem edições e com o rascunho guardado neste navegador: nenhum aviso. Com armazenamento bloqueado: avisa só se houver edição ou rascunho só na página. Num conflito entre abas, o rascunho guardado é o da outra aba: avisa só pela edição do perfil. Depois de um Salvar recusado, a edição continua e o aviso também.

### 12.5 Resumo de estado (2.1)

[Implementado] Lista `#projectSummary` na área Projeto, com os oito itens da tabela da 2.1, cada um com o próprio estado escrito (carregando, "Não foi possível ler: {erro}", sucesso). Os dados vêm das leituras que a página já fazia; nenhuma rota nova.

| Item | Fonte | Destino do clique |
|---|---|---|
| Medição | `/api/hotspots` e `measure_all` de `/api/commands` | Explorar › Onde pesa; "Falta aprovar o comando" leva à Régua |
| Comandos | `/api/commands`: "N de M aprovado(s) na versão atual" | Régua |
| Contagens manuais | `manual_counts` (`null` = "Falha de leitura (não quer dizer \"nenhuma\")") | Onde pesa, quando há contagens; "Nenhuma" sem link |
| Léxico | `settings.lexicon` gravado | Léxico |
| Política | política gravada (3.13) | o seletor do modo, no compositor |
| Fatos | `/api/facts` (mais "mudanças aprovadas, não aplicadas", quando há) | Acervo |
| Perguntas | `/api/turns`: preparadas e não enviadas; com decisões pendentes | a primeira com decisões pendentes, ativada; sem pendentes, a coluna Pergunta |
| Run | o registro da run | Régua, só se a run tem alterações; senão, sem link |

- Projeto sem runs: os itens que dependem da run dizem "Sem run: gere a primeira".
- Ir a outra área deixa o ponto no Voltar (`goTo`). O resumo só é redesenhado quando o conteúdo muda (o foco do teclado não se perde).

### 12.6 Régua (2.5)

[Implementado]

- Cada comando: a versão (os 12 primeiros dígitos do hash) e "aprovado nesta versão em {data}" ou "não aprovado nesta versão".
- **Linha de base em vigor** por comando: a última execução desta versão com sucesso, sem mudança e sem instrumentação (o critério de `baseline_of`, `execution.rs`), ou "Sem linha de base em vigor nesta versão". Ver a decisão 10 de 12.8.
- **Run com alterações:** Rodar desabilitado em todos os comandos **antes** do clique, com o motivo no título e, nos aprovados, no texto.
- **Histórico:** cada execução com mudança tem "Abrir a pergunta #N" (T9: ativa a pergunta sem voar e a guarda como escolha); cada medição (`instrumentation`) tem "→ Onde pesa" e aparece como "medição" na linha; as outras são "linha de base".

### 12.7 Acervo (2.6, T13)

[Implementado]

- **Estados:** carregando ("Carregando os fatos…"), falha de leitura ("Não foi possível ler os fatos: {erro}"), vazio (o texto que já existia, "Nenhum fato aprovado ainda…"), sem run.
- **Filtro por estado** (`#factsFilter`): todos e cada estado presente, com as contagens. Só na memória da página.
- **Ir à origem:** na mesma run, ativa a pergunta, abre o envio (inclusive um envio antigo dobrado) e destaca o item na camada "O que você decide". Em outra run, **T13**: "A origem está na run {X}. Trocar? O rascunho desta run fica guardado." Cancelar não muda nada; confirmar guarda a run e a pergunta como escolhas da pessoa e mostra a origem. O rascunho da run de saída fica guardado e não vai para a outra run.
- **Revogar…:** leva à origem e põe o foco no botão "Revogar" do item (`data-decide="revoke"`); a decisão continua num lugar só, com a nota, pelo `POST /api/review` que já existia. Fato revogado: só "Ir à origem".
- **Juntar a declaração ao rascunho** (FACT não revogado): procura no grafo da run a aresta `declares` cuja evidência começa no arquivo e na linha de declaração do fato, ligada a um símbolo com o mesmo nome (rótulo igual ou terminado em `::{nome}`). Uma fonte só: junta como fonte manual, "chegou por: Acervo · só local". Nenhuma: "A declaração … não está no grafo desta run: nada foi juntado." Mais de uma: "Ambíguo: …; nada foi juntado". Recusa também durante uma investigação, durante uma entrada que recomeça o rascunho e num conflito entre abas. **Ver o defeito M2 de 12.10.**

### 12.8 Decisões de implementação tomadas durante o F9

Apresentadas no relatório do F9; **nenhuma foi aprovada até agora**. A coluna "Classificação" é da auditoria de 13/11.

| # | Decisão | Classificação |
|---|---|---|
| 1 | `run` é gravada só por escolha da pessoa (seletor, Gerar, T13); abrir e recarregar não gravam | Coerente com 5.3 e com a regra do F2 ("abrir não grava") |
| 2 | Run guardada que não existe mais: abre a mais recente, avisa e não regrava | Coerente com 2.1 ("senão, a mais recente") |
| 3 | Armazenamento indisponível: reabrir o mesmo projeto fica na run da memória | Coerente com o F2 (a mesma regra da área em `applyKeptNav`) |
| 4 | O T25 pergunta também ao reabrir o mesmo projeto | Coerente (reabrir recarrega o perfil e perderia as edições); precisa de aprovação |
| 5 | Critérios do rascunho não entram no T25 nem no aviso ao sair, porque ficam guardados com o rascunho | Coerente quanto à perda de dados; **precisa de aprovação**: o cabeçalho continua dizendo "Alterações não salvas: … critérios do rascunho", e sair não pergunta nada por eles |
| 6 | Cancelar o T25 não devolve o campo do caminho ao projeto aberto | Precisa de aprovação (o campo pode mostrar outro caminho que o do projeto aberto) |
| 7 | Os links da Régua, do Acervo e do resumo ativam a pergunta **sem voar** e guardam `active_turn` | Precisa de aprovação: a 1.4 deixa a câmera mudar no "clique numa pergunta"; a decisão 2 do F8 restringiu o voo ao clique na linha do tempo |
| 8 | Item "Run" só tem link se a run tem alterações; "Contagens manuais: Nenhuma" sem link; "Perguntas" sem pendentes leva à coluna Pergunta | **Possível divergência** com "cada item do resumo leva ao destino correspondente" (2.1); a especificação não define o destino desses casos |
| 9 | "Política" leva ao seletor do modo, no compositor | Coerente (é onde a política muda, 3.13) |
| 10 | Linha de base em vigor com o critério da comparação (`baseline_of`, mesma versão do comando) | **Divergência com o texto da 2.5**, que diz "o critério da escada": o portão da escada (`main.rs`, `/api/ladder`) aceita uma linha de base de qualquer versão do comando. Com a versão trocada, a Régua diz "sem linha de base em vigor" e a escada não roda uma nova. Precisa de decisão |
| 11 | O filtro inclui "mudança aprovada, ainda não aplicada" e fica só na memória | Coerente (é um estado que a API devolve) |
| 12 | "Revogar…" só leva ao controle e põe o foco; não abre o diálogo | Coerente com 2.6 ("um lugar só para decidir") |
| 13 | Juntar a declaração pela aresta `declares` com arquivo, linha e nome exatos | Coerente na intenção, mas com o defeito M2 (12.10) |
| 14 | O T13 acrescenta "O rascunho desta run fica guardado." à mensagem da especificação | Coerente (5.6) |
| 15 | O resumo de Fatos acrescenta as mudanças aprovadas não aplicadas, quando há | Coerente (o estado existe na API) |
| 16 | Chaves `commands.approved` e `commands.unapproved` removidas (sem uso) | Coerente |

**Depois do fechamento (12.13):** as decisões 4, 5, 6, 7, 8 e 10 foram decididas pelo usuário no prompt do fechamento: 4 mudou (reabrir sem perda não pergunta), 5 ficou com outra apresentação, 6 mudou (o caminho volta), 7 ficou, 8 mudou (Run → Régua, Contagens → Inventário), 10 mudou (critério da escada). A 13 mudou com o M2. As demais (1, 2, 3, 9, 11, 12, 14, 15, 16) continuam como classificadas, sem decisão explícita além da avaliação do F9.

### 12.9 Requisitos e critérios de aceitação

| Requisito | Situação |
|---|---|
| 2.1 Abrir restaura a run guardada, se existir; senão a mais recente | [Verificado] atendido (`tela-projeto` caso 2) |
| 2.1 Sincroniza o modo com a política | Do F7, mantido ([Verificado] "o modo continua o da política gravada") |
| 2.1 Sem runs: "Sem run: gere a primeira" | **Parcial:** o resumo diz isso; a nota da run e a cidade continuam com "Nenhum grafo gerado ainda para este projeto." |
| 2.1 Recomeçar não apaga rascunhos | Do F6, mantido |
| 2.1 Gerar sem Salvar implícito | Do F7, mantido; a run nova guardada como escolha (12.2) está **só no código**, sem teste (as baterias abortam `/api/generate`) |
| 2.1 Resumo de estado: estados escritos por item | [Verificado] atendido para carregando, falha de leitura e sucesso, **menos** o defeito M1 (12.10) |
| 2.1 "Cada item carrega à parte" | **Parcial** (12.10, investigação 2) |
| 2.1 "Uma falha num item não afeta os outros" | [Verificado] atendido entre fatos, comandos e perguntas; **não atendido** quando falha a leitura do grafo (M1) |
| 2.1 Cada item leva ao destino e grava o ponto no Voltar | [Verificado] para as áreas; parcial nos itens sem destino (decisão 8) |
| 2.5 Versão (hash) e "aprovado nesta versão" | [Verificado] atendido |
| 2.5 "Aprovação de outra versão" | **Não atendido** (12.10, investigação 1) |
| 2.5 Linha de base em vigor | [Verificado] atendido com o critério da comparação; diverge do texto (decisão 10) |
| 2.5 Histórico: "Abrir a pergunta #N", "linha de base", "→ Onde pesa" | [Verificado] atendido |
| 2.5 Rodar desabilitado antes do clique numa run com alterações | [Verificado] atendido |
| 2.5 Estados (sem comandos, não aprovado, rodando, falhou, concluído) | Comportamento anterior, mantido; não reconferido no F9 |
| 2.6 Lista reconferida, estados, filtro | [Verificado] atendido; o texto do vazio não é o da especificação ("Nenhum fato aprovado neste projeto" só no resumo) |
| 2.6 Ir à origem (mesma run e T13) | [Verificado] atendido |
| 2.6 Revogar no controle de decisão | [Verificado] atendido (foco; nada gravado) |
| 2.6 Juntar a declaração | **Parcial:** [Verificado] no Rust; falha com funções Python decoradas (M2) |
| T13 | [Verificado] atendido |
| T17 (aviso, run, pergunta ativa, rascunho) | [Verificado] atendido |
| T25 | [Verificado] atendido nos quatro caminhos |
| 9.10.1 Ray tracer: Onde pesa, Otimizar, escada com provedor de teste, Comparação, Copiar diff | **Parcial:** Onde pesa e o ranking verificados no F9; Otimizar (`tela-builder`) e Copiar diff (`tela-conversa`, F8) verificados em outras baterias; a escada nunca roda com um provedor de verdade nas baterias (abortada ou simulada) e não existe um "provedor de teste" no Studio; a Comparação é do F12. A aceitação completa é do F13 |
| 9.10.2 Python sem `instrument`: "Não disponível" e o Inventário | [Verificado] atendido (o motivo é `no_source`; o Inventário fica habilitado, sem ser montado na bateria) |
| 9.10.3 Cada um com e sem medição gravada | **Parcial:** Rust com medição (run `20261104-020244`) e sem (a run mais recente, "Pode medir"); Python sem medição. Python **com** medição não é possível: o servidor só insere a contagem em fontes Rust (12.10, investigação 4) |
| Pendência 3 (restaurar a run) | Resolvida |
| Pendência 29 (juntar a declaração) | Resolvida em parte (M2) |
| Pendência 35 (T25 e aviso ao sair) | Resolvida |

Testes automáticos passando não comprovam a seção 9 da especificação inteira: a aceitação completa é do F13.

### 12.10 Auditoria de 13/11/2026

**Material conferido:** a página atual (7536 linhas, `b6fcff72261582de`), igual à página registrada no log da execução final; os diffs `f9-index.diff` (+407/−49 linhas contra o fim do F8) e `f9-baterias.diff`, iguais ao estado atual da árvore (`cmp`); a base `base-f8/`; os logs `final.log`, `rodar.log`, `captura.log`, `contra-f8.log`, `fluxos-contra-f8.log` e `cargo.log`. O diff da página não acrescenta nenhuma chamada `PUT`, `POST` ou `DELETE`; nenhum arquivo `.rs` ou `.toml` mudou.

**Monitores expirados:** os avisos "Monitor expired" do fim da sessão do F9 vieram dos observadores de log, não dos testes. O `final.log` termina em `FIM`, o `rodar.js` saiu com 0 em 1138 s, o comando em segundo plano terminou com código 0, e os horários conferem: a página e as baterias foram gravadas antes do início da execução final.

**Defeitos, por gravidade:**

| # | Gravidade | Defeito | Evidência |
|---|---|---|---|
| M1 | Média | Se a leitura do grafo (`/api/graph`) falha ao abrir uma run, **Medição, Comandos, Contagens manuais, Fatos e Perguntas ficam em "Carregando…" indefinidamente**, e a conversa em "Carregando as perguntas desta run…". O erro só aparece na nota do projeto. Causa: `loadRun` lê o grafo antes de tudo e não trata a falha; o resumo deriva os estados de leituras que nunca começam. A parte da conversa vem do F8 | Sonda da auditoria (P1): `/api/graph` respondido com 500, resumo lido 15 s depois. Não há bateria |
| M2 | Média | **Juntar a declaração falha com funções Python decoradas e diz algo falso**: "não está no grafo desta run". No grafo, a declaração começa na linha do decorador; o fato guarda a linha do `def`. Com os dados de teste, o fato F7 do projeto Python (`app/api/routes/routing.py`: declaração no grafo em 12–24, fato com `declaration_line` 13) não é achado. Nada errado é juntado; a ação só não funciona e a mensagem engana | Simulação da regra da página sobre os 8 fatos Python e os 9 fatos Rust reais, nos grafos das runs de teste: 7 de 8 e 8 de 9 achados. O F8 Rust não achado é desatualizado (linha 88, declaração em 86), caso em que a mensagem é correta |
| B1 | Baixa | O carregamento do resumo não é independente: os comandos só são lidos depois dos fatos, os fatos depois das perguntas, e as perguntas depois do grafo (o encadeamento de `loadRun`, anterior ao F9) | Leitura do código; a bateria segura `/api/commands` e mostra os outros itens já lidos, mas não o inverso |
| B2 | Baixa | Uma edição feita **durante** o pedido de abertura (depois da confirmação do T25) se perde sem pergunta: os campos não ficam desabilitados | Leitura do código; não exercitado |
| B3 | Baixa | O texto do Acervo vazio não é o da 2.6 ("Nenhum fato aprovado neste projeto"); o resumo usa o da especificação | Leitura do código e da bateria |
| B4 | Baixa | Gerar grava a run nova como escolha, mas isso não tem teste | Leitura do código |

**Investigações pedidas na auditoria:**

1. **Aprovação de outra versão.** Nenhuma rota lista as aprovações: `GET /api/commands` devolve `approved` só para o hash atual (`execution.rs`, `commands`). As execuções guardam `command_sha256` e a data da aprovação usada, então uma execução de outra versão **prova** que houve aprovação de outra versão; mas aprovações sem execução não aparecem, e as execuções são por run. Nas raízes de teste não há nenhuma aprovação de outra versão (três aprovações, todas da versão atual), então o caso não pode ser exercitado com esses dados. "Não aprovado nesta versão" é **verdadeiro e não afirma** que nunca houve aprovação; mas não dá a distinção que a 2.5 pede. Requisito **não atendido**; caminhos possíveis: aceitar a limitação, derivar em parte das execuções (frontend, incompleto) ou expor as aprovações (backend). Precisa de decisão.
2. **Carregamento independente.** A 2.1 diz: "Cada item carrega à parte, e uma falha num item não afeta os outros." São duas exigências. A segunda está atendida entre fatos, comandos e perguntas (verificado), e não atendida quando o grafo falha (M1). A primeira não está atendida no sentido literal (B1): os itens mostram estados próprios, mas as leituras seguem o encadeamento anterior. Requisito **parcial**.
3. **9.10.1.** Verificáveis agora: Onde pesa e o ranking (F9), Otimizar (F5) e Copiar diff (F8). A escada "com provedor de teste" não tem provedor de teste no Studio: as baterias abortam ou simulam `/api/ladder`, e uma escada real chamaria o Claude CLI (proibido nos testes). A Comparação é do F12. A aceitação completa é do F13.
4. **9.10.3 em Python.** O servidor recusa a medição em projetos sem fonte Rust (`measure_all.reason = "no_source"`: "a contagem por função só é inserida em fontes Rust"). Python com medição não é uma falha do F9: é uma capacidade que a v1 não tem. A 9.10.3 pede "cada um com e sem medição gravada"; para Python, a parte "com" é inaplicável enquanto o servidor não instrumentar Python. Precisa do aceite do usuário como limitação.
5. **Declaração ambígua.** Nos 30 grafos das raízes de teste (29 Rust, 1 Python), há 145 linhas que declaram mais de um símbolo, e **nenhum** caso de mesmo arquivo, linha e nome. É o filtro pelo nome que desambigua. A ausência de teste com dados reais é uma **lacuna de cobertura**, não um risco funcional identificado. O risco funcional real está no lado oposto: o nome e a linha exatos fazem a regra **não achar** casos legítimos (M2).
6. **Juntar durante investigação ou conflito.** Há duas proteções: `joinFact` confere investigação, entrada que recomeça e conflito antes de chamar `toggleGathered`, que confere as mesmas três de novo. A sonda da auditoria (P2 e P3, fora das baterias) mostrou a recusa com o motivo, nada juntado, nenhuma escrita e nenhum erro de página. **Não há bateria** para isso: fica como comportamento conferido por sonda, não por teste do repositório.
7. **Decisões.** Classificadas em 12.8.
8. **Contraprova contra o F8.** Contra a página do fim do F8, a `tela-projeto` aborta no caso 1 (o resumo não existe: `Cannot read properties of undefined`), com 1 PASSA (o perfil igual em bytes). Isso mostra que a bateria não passa sem o F9; **não mostra**, uma a uma, quais das outras 137 verificações falhariam. É uma contraprova grosseira.

**Testes que mudaram e o rigor:** as expectativas antigas alteradas conferem o comportamento novo, e não ficaram mais frouxas, com uma ressalva: a verificação 1 da `tela-centro` passou a aceitar dois resultados (navegação nula, ou só a run escolhida pela bateria). Na prática a bateria sempre escolhe a run, então o segundo é o que acontece.

### 12.11 Testes do F9

- **Bateria nova `tela-projeto.js`: 138 verificações** (139 chamadas a `ok`, uma delas só no caminho de exceção). Casos: 1 resumo (itens contra `/api/commands`, `/api/hotspots`, `/api/facts`, `/api/turns` e o perfil; destinos; falhas isoladas de fatos, comandos e perguntas; carregando; projeto sem runs); 2 run guardada (recarregar, pergunta ativa, rascunhos por run e por projeto, run inexistente, armazenamento bloqueado, resposta atrasada); 3 T25 (as três escolhas, Salvar cancelado e recusado, a ordem PUT antes da troca, Enter, Procurar, Limpar); 4 T17 (evento sintético e `beforeunload` real ao fechar e ao recarregar); 5 Régua (inclusive a run com alterações simulada); 6 Acervo (filtro, origem, T13, Revogar, Juntar, estados); 7 Python e Rust (9.10); 8 tela estreita, tema escuro e claro, contraste, teclado, EN e as chaves do F9 em PT e EN. Projetos fictícios: B (pendência 11). Nada é gravado no servidor: os `PUT` que a bateria precisa ver são respondidos por ela.
- **Expectativas antigas alteradas** (contagens iguais), porque conferiam o comportamento anterior ao F9:
  - `tela-estado` B1: "recarregar volta à run mais recente" → volta à run escolhida; "abrir, trocar de run e recarregar não gravam navegação" → abrir e recarregar não gravam, e trocar de run grava só `run`.
  - `tela-centro`: caso 1 (a escolha da run pela bateria grava `run`), caso 7 (a troca de run grava só a run nova; a vista central não) e caso 8 (reabrir o projeto volta à run guardada).
  - `tela-rascunho` caso 1: o Builder volta à frente só se era a vista central guardada (T17); antes do F9 o recarregamento abria outra run. E o comentário do auxiliar `recarrega`.
  - `tela-navegacao`: só o nome de uma verificação (a expectativa continua a mesma: sem run escolhida, recarregar abre a mais recente).
  - `rodar.js`: + `tela-projeto`.
- **Primeira execução completa:** 1262 PASSA, 3 FALHA, exatamente nas três expectativas antigas acima, ajustadas em seguida (log `final-1.log` e `rodar-1.log`).
- **Execução completa final** (12/11, 23:14–23:40): **1265 PASSA, 0 FALHA**, `rodar.js` saída 0 em 1138 s, página `b6fcff72`.
- **Corpos 9.5:** 21/21 iguais ao F0. **Contra a captura do F8:** fluxos, modos, sondas, laço de Separar e restauração das raízes iguais (a diferença de texto da pendência 49 não apareceu). A captura registra "1 erro", o `file_mode` conhecido (pendência 18).
- Raízes de teste: `send.json` 844 → 844, `question.json` 1420 → 1428 (as 8 do `etapa2c`), perfis iguais em bytes.
- **Rust (sessão do F9):** `cargo test --locked -p librarian-studio` 185 aprovados (184 + 1); `cargo test --locked --workspace` 432 aprovados, 2 ignorados; `cargo build --release` com sucesso.
- **Reproduzido na auditoria (13/11):** `cargo test --locked -p librarian-studio` 185 aprovados; `cargo test --locked --workspace` 432 aprovados, 0 falhas, 2 ignorados; `cargo build --release --locked -p librarian-studio` com sucesso; o hash e as linhas da página; os diffs iguais ao estado atual; a sonda P1–P3; as simulações sobre os grafos. **As baterias de tela e a captura não foram reexecutadas na auditoria:** os números delas vêm dos logs da sessão do F9.
- Material: `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\8f80f82a-52dd-4d24-b9bf-1981f62091b4\scratchpad\f9\` (`final.sh`, `so.sh`, `sobe.sh`, `desce.sh`, `so-base.sh`, `captura.js`, `compara.js`, `compara-fluxos.js`, `base-f8\`, `f9-index.diff`, `f9-baterias.diff`, `log\`, `auditoria\sonda.js`, `auditoria\sonda.log`, `auditoria\cargo.log`, `consolidacao\` com os documentos antes desta consolidação).

### 12.12 Limitações conhecidas

- M1 e M2 (12.10), sem correção.
- "Aprovação de outra versão" não é mostrada (investigação 1).
- O resumo não carrega cada item de forma independente (B1).
- Python com medição não existe na v1 (investigação 4).
- Juntar durante investigação ou conflito só foi conferido por sonda; a declaração ambígua não tem caso real.
- Os testes de outro projeto continuam com projetos fictícios (pendência 11).

As limitações acima são as da auditoria. O que mudou com o fechamento está em 12.13.

### 12.13 Fechamento do F9 (13/11/2026)

Sessão `6e1931b3`, depois da auditoria, por prompt próprio do usuário: corrigir M1 e M2, aplicar as decisões do prompt, acrescentar testes de regressão e consolidar. **Sem commit, sem push, sem F10.** Estado de partida conferido: `main`, HEAD `9bc92b0`, as mesmas 10 alterações e 11 arquivos não rastreados de 2.1, página com 7536 linhas e hash `b6fcff72`, igual à da auditoria.

#### 12.13.1 Decisões do usuário aplicadas nesta sessão

[Aprovado] Vieram no prompt do fechamento, mais uma resposta durante a sessão:

| Assunto | Decisão | Onde |
|---|---|---|
| Linha de base em vigor (12.8, decisão 10) | Vale a 2.5: o critério da escada, **qualquer versão** do comando | 12.13.4 |
| Aprovação de outra versão | Não deduzir das execuções; manter "não aprovado nesta versão"; requisito **parcial**; expor o histórico de aprovações é evolução futura do servidor | 12.13.6 |
| Destinos do resumo (decisão 8) | Run leva sempre à Régua. Contagens manuais: o prompt dizia "ao Acervo"; como o Acervo não mostra contagens nem tem filtro para elas, a pergunta foi feita na sessão e o usuário escolheu **Explorar › Inventário, em todos os estados** (onde as contagens aparecem; é o que diz a nota de Onde pesa) | 12.13.4 |
| Critérios do rascunho (decisão 5) | Não provocam T25 nem aviso ao sair; a tela distingue rascunho guardado, critério não salvo como padrão e edição do perfil não salva; nenhuma gravação implícita | 12.13.4 |
| Links sem mover a câmera (decisão 7) | Mantido | — |
| Cancelar o T25 (decisão 6) | O campo do caminho volta ao do projeto aberto | 12.13.4 |
| Reabrir o mesmo projeto (decisão 4) | Sem confirmação quando nada é descartado | 12.13.4 |
| Python com medição (9.10.3) | Limitação da versão atual; nada de backend | 12.13.6 |
| Provedor de teste | Falso e determinístico só na infraestrutura de testes; nenhum provedor novo na interface; não simular integração não exercitada | 12.13.6 |

#### 12.13.2 M1: falha na leitura do grafo

- **Causa:** `loadRun` lia o grafo antes de tudo (`await`), sem tratar a falha; as perguntas, os fatos e os comandos só eram pedidos depois dele. Com `/api/graph` falhando, a exceção saía de `loadRun`, ia para a nota do projeto (`busy`), e as outras leituras nunca começavam: cinco itens do resumo e a conversa ficavam em "Carregando…". A cidade da run anterior continuava na tela, sob o nome da run nova.
- **Correção:** `loadRun` tira o grafo da run anterior da cena ao começar (`unloadGraph`: cidade, seleção, inspeção, resultados de Encontrar, busca acesa; o texto digitado em Encontrar fica e a busca é refeita no grafo novo) e começa juntas as leituras do grafo, das perguntas, dos fatos e dos comandos; cada uma é mostrada quando chega e diz a própria falha. A falha do grafo (`graphFailed`) é dita onde o grafo aparece: o estado da cidade e a nota da run ("Não foi possível ler o grafo da run {run}: {erro}") e o item Run do resumo (como erro, com a mensagem do servidor). Também cobre um grafo que chega e não passa no contrato (`ingest` lança). Sem grafo, o rascunho guardado da run não é restaurado (precisa do `catalog_sha256`, 5.5) e fica como estava; Juntar a declaração diz a falha. Ao trocar o idioma, a cidade e a nota da run continuam dizendo a falha (antes: "abra um projeto").
- **O que depende do grafo** continua esperando por ele: a cidade, a restauração do rascunho e a busca que a pergunta ativa acende (`lightActive`; `activate` só acende com o grafo da run na tela). As perguntas, os fatos, os comandos e a medição não esperam.
- **Testes** (`tela-projeto`, 9a e 9b): grafo ilegível ao trocar de run e ao abrir o projeto: os cinco itens terminam, cada um com o que leu de `/api/*` da run aberta; Run como erro e com link; a cidade e a nota da run dizem a falha, e a nota do projeto não; nada do grafo anterior na tela; a conversa com as perguntas da run; os rascunhos guardados intactos; Régua e Acervo listados; Juntar recusado; EN; de volta a uma run legível, tudo normal; sem erros de página nem escritas.

#### 12.13.3 M2: Juntar a declaração de funções Python decoradas

- **Causa:** a regra procurava a aresta `declares` cujo `start_line` fosse igual à linha de declaração do fato. O intervalo da declaração no grafo começa nos decoradores, atributos ou comentários de documentação (`python.rs`, `declare_in`); a linha do fato é a primeira linha do trecho que não é nenhum deles (`claims.rs`, `symbols`). Com decorador, as duas nunca coincidem (F7 Python: grafo 12–24, fato 13). Além disso, o filtro do nome só aceitava `::nome`; o Python qualifica com `.` (`Settings.emails_enabled`).
- **Correção (`factDeclaration`, `factLineUnsure`):** a declaração é a do mesmo arquivo e do mesmo nome (rótulo igual, ou terminado em `::nome` ou `.nome`) cujo intervalo contém a linha do fato **no cabeçalho**: nenhuma outra declaração do arquivo começa dentro dela, depois do início e até essa linha (senão a linha estaria num corpo ou numa declaração aninhada). Isso só é exato enquanto a linha é a que o servidor reconferiu: o fato precisa estar **vigente** e citar a própria linha de declaração (todos os 17 fatos reais citam). Fato desatualizado: recusa com o motivo do servidor. Nenhuma: "não está no grafo desta run". Mais de uma: "Ambíguo", com os intervalos de cada uma. Nada é juntado por aproximação; o significado de `declares` não muda.
- **Por que o fato desatualizado é recusado:** o F8 Rust (desatualizado) guarda a linha 88, que hoje é corpo de `intersect` (86–101). Uma regra só por "contém a linha" juntaria essa declaração por aproximação. Antes do fechamento, o mesmo fato dava "não está no grafo"; agora diz que não está vigente, com o motivo. **Decisão de implementação desta sessão, sem aprovação.**
- **Testes** (`tela-projeto`, 9c): o dado real F7 (grafo 12–24, fato 13, linha 12 com `@`, linha 13 com `def`) juntado; os 8 fatos vigentes reais do Python acham, cada um, exatamente a declaração cujo cabeçalho contém a linha, conferido no **texto da fonte** do projeto; dois decoradores (`@field_validator` e `@classmethod`, `Settings._use_psycopg_driver` 35–42; `@computed_field  # …` e `@property`, `Settings.emails_enabled` 61–64), com fatos sintéticos sobre declarações reais do grafo; sem correspondência (nome inexistente, linha fora); fato que não cita a própria linha; ambiguidade (caso **sintético**: uma segunda declaração de `plan_route` com o mesmo intervalo, acrescentada à resposta de `/api/graph` na página da bateria; não há caso real em nenhum dos 30 grafos); Rust: os fatos vigentes acham a declaração que começa na linha deles, e o F8 desatualizado é recusado.
- **Achado no servidor (não corrigido, fora do escopo):** o próprio servidor compara a linha do fato com o **início do trecho** (`store.rs`, `target_of`), em dois lugares: os fatos vigentes ligados a cada fonte da sugestão (`/api/envelope/suggest`) e os fatos aprovados que uma pergunta leva (`reviewed_about`, `with_facts`). Com função Python decorada, esses fatos não são ligados. É o mesmo defeito do M2, do lado do backend. Ver a pendência 65.

#### 12.13.4 Outras correções do F9

| Ressalva | Correção | Teste (`tela-projeto` 9, salvo indicação) |
|---|---|---|
| B1 carregamento não independente | As leituras do grafo, perguntas, fatos e comandos começam juntas (12.13.2). `loadFacts` continua lendo fatos e depois comandos para os outros chamadores; `loadRun` usa `readFacts` e `loadCommands` em paralelo | 9b: grafo segurado (os cinco itens e a conversa chegam antes; a cidade diz "Carregando grafo…" e não mostra grafo nenhum); fatos segurados (Comandos e Contagens chegam: antes do fechamento eles esperavam os fatos); perguntas seguradas; falha da medição isolada; as falhas de fatos, comandos e perguntas do caso 1 continuam |
| B2 edição durante a abertura | Da decisão do T25 até o perfil do projeto aberto estar na tela, os campos editáveis do perfil ficam desabilitados (`lockProfile`). Foi escolhido impedir, e não detectar depois, porque a página já desabilita os botões durante cada operação (`busy`) | 9e: com o pedido de abertura segurado, os 12 campos desabilitados e uma edição não acontece; depois, habilitados; abertura recusada pelo servidor: o projeto não muda, a edição continua e os campos voltam |
| B3 textos de vazio | Acervo vazio: "Nenhum fato aprovado neste projeto." mais o próximo passo que já existia (7.2). Nota da run num projeto sem runs: "Sem run: gere a primeira" (2.1). O estado da cidade continua "Nenhum grafo gerado ainda para este projeto." | 6 (Acervo vazio, exato) e 9l |
| B4 Gerar sem teste | — | 9k: abrir e recarregar sem escolha não gravam; Gerar (resposta simulada de `/api/generate`, que aponta para uma run existente como se fosse a nova) guarda `nav.run`; recarregar volta a ela sem regravar; nenhum `PUT` |
| Verificação 1 da `tela-centro` | Voltou a ser exata, em duas verificações: escolher RUN pelo seletor grava só `run` (com a condição de que a mais recente é outra, conferida); abrir sem escolher run não grava nada (página nova) | `tela-centro` 93 → 94 |
| Cancelar o T25 | O campo do caminho volta ao caminho com que o projeto aberto foi aberto (`keepPath`), também quando o Salvar de "Salvar e abrir" falha | 9f: Abrir, Enter, Procurar › Escolher e Salvar recusado. O caso 3 antigo ganhou o preenchimento do caminho de B antes de cada tentativa (ele contava com o caminho deixado no campo) |
| Reabrir o mesmo projeto | Se o caminho é o do projeto aberto (comparado sem diferença de barras nem barra final), não há T25: o registro é relido (`takeProfile`: as edições continuam edições; política, critérios padrão e runs seguem o registro) e a página fica como estava (run, pergunta, rascunho, vistas, fluxograma). Só troca de run se a run aberta saiu do registro; aí abre a guardada ou a mais recente, e nada se perde (as edições ficam, o rascunho da run fica guardado). Nota: "O projeto … já estava aberto: … nada da página foi descartado". Se, depois do pedido, a chave não for a mesma, o T25 é feito antes de aplicar | 9g; `tela-centro` caso 8 continua passando |
| Critérios do rascunho | "Alterações não salvas" do cabeçalho mostra só edições do perfil; a linha do rascunho diz "critérios próprios, não salvos como padrão" (também num rascunho sem fontes, e o clique leva aos critérios); o compositor diz "Diferem do padrão do projeto (não salvos como padrão; ficam no rascunho)". A chave `unsaved.place.criteria` saiu | 9j; `tela-perfil` com 2 expectativas atualizadas (mesma contagem) |
| Linha de base em vigor | Critério da escada: a última com sucesso, sem mudança e sem instrumentação, de qualquer versão; quando é de outra versão, a Régua diz "de outra versão do comando ({sha}): a escada a aceita; a comparação de uma mudança usa só uma desta versão" (`data-baseline-version`) | 5 (expectativa com o critério da escada) e 9h: versão trocada por resposta simulada de `/api/commands` |
| Destinos do resumo | Run → Régua sempre; Contagens manuais → Explorar › Inventário (algumas, nenhuma, ilegíveis, comandos ilegíveis) | 9i |
| Juntar durante investigação e conflito | Sem mudança de código (já recusava); agora com bateria | 9d |

Também: Limpar zera o estado do grafo e o caminho guardado em memória.

#### 12.13.5 Divergências registradas

- **Texto dos critérios do rascunho:** a 3.12 cita "Diferem do padrão do projeto (não salvos)"; a tela diz "(não salvos como padrão; ficam no rascunho)", para cumprir a decisão de distinguir os três casos. A especificação não foi alterada.
- **Destino de Contagens manuais:** o prompt dizia Acervo; o usuário decidiu Inventário durante a sessão (12.13.1).
- **Fato desatualizado em Juntar a declaração:** recusado (12.13.3); a especificação não trata o caso.
- **Troca de run:** a cidade da run anterior sai da cena assim que a troca começa (antes ficava até o grafo novo chegar). Necessário para não mostrar dados antigos como os da run aberta (M1) e para que as baterias que esperam a resposta de `/api/turns` não vejam o grafo anterior.
- Nenhuma contradição entre as decisões 1 a 16 de 12.8 e a especificação além das já classificadas na auditoria (10, resolvida pela 2.5).

#### 12.13.6 Requisitos depois do fechamento

| Requisito | Situação |
|---|---|
| 2.1 "Cada item carrega à parte" | [Verificado] atendido (9b); as dependências reais (cidade, rascunho, busca acesa) esperam o grafo |
| 2.1 "Uma falha num item não afeta os outros" | [Verificado] atendido, inclusive com o grafo ilegível (M1) e a medição ilegível |
| 2.1 Cada item leva ao destino | [Verificado] atendido; Run → Régua, Contagens → Inventário; "Perguntas" sem pendentes continua levando à coluna Pergunta |
| 2.1 Sem runs: "Sem run: gere a primeira" | [Verificado] no resumo e na nota da run |
| 2.1 Gerar guarda a run nova | [Verificado] (9k, com resposta simulada de `/api/generate`) |
| 2.5 Linha de base em vigor | [Verificado] pelo critério da escada (5 e 9h) |
| 2.5 "Aprovação de outra versão" | **Parcial.** "Não aprovado nesta versão" é mostrado e não afirma que nunca houve aprovação; a API não lista aprovações (`GET /api/commands` só diz se a versão atual está aprovada). Evolução futura: o servidor expor o histórico de aprovações. Nada deduzido das execuções |
| 2.6 Vazio | [Verificado] "Nenhum fato aprovado neste projeto." |
| 2.6 Juntar a declaração | [Verificado] Rust e Python, com decoradores (M2 corrigido); desatualizado recusado com o motivo |
| T25 | [Verificado] mais Cancelar com o caminho restaurado, edição impedida durante a abertura e reabrir sem perda |
| 9.10.1 | **Parcial**, sem mudança: Onde pesa, Otimizar e Copiar diff verificados; a escada nunca roda com provedor nas baterias (abortada ou simulada; não há provedor de teste no Studio, e nenhum foi acrescentado à interface); a Comparação é do F12 |
| 9.10.2 | [Verificado] atendido |
| 9.10.3 | **Parcial / limitação da versão atual:** Python com medição não existe (o servidor só instrumenta fontes Rust); nada de backend nesta sessão |

#### 12.13.7 Testes e resultados reproduzidos

- **Página final:** 7676 linhas, hash `693f69dd2fbcda3b` (era 7536, `b6fcff72`). Nenhuma chamada de escrita nova; nenhuma rota, corpo ou código Rust alterado (`git diff` sem `.rs` nem `.toml`); a especificação não foi tocada (última alteração em 11/11).
- **Baterias alteradas:**
  - `tela-projeto.js` 138 → **219**: seção 9 nova (81 verificações) e, nos casos antigos, expectativas que mudaram de propósito: 5 (linha de base pelo critério da escada), 6 (o fato levado para fora do grafo cita a linha nova, porque um fato vigente cita a própria declaração; o Acervo vazio exato), 3 (o caminho de B escrito antes de cada tentativa, porque Cancelar agora devolve o campo). A seção 9 roda em blocos: uma exceção vira a FALHA do bloco e o seguinte continua.
  - `tela-centro.js` 93 → **94**: verificação 1 exata, em duas (12.13.4); no caso 7, a espera fixa de 1 s depois de soltar a resposta de `/api/path-questions` virou espera pela nota de erro (mesma expectativa).
  - `tela-builder.js` 102: os auxiliares `juntaPelaInspecao` e `linha` esperam a medição chegar antes de ler `hot.rows` (antes, a resposta de `/api/turns` só chegava depois do grafo e dava tempo). Mesma contagem, mesmas expectativas.
  - `tela-perfil.js` 150: duas expectativas de texto dos critérios do rascunho (12.13.4). Mesma contagem.
- **Execuções completas** (`rodar.js`, raízes recopiadas das intactas):
  1. Interrompida pelo ambiente: a pasta do `playwright-core` foi apagada durante a execução (pendência 69); as 9 primeiras baterias passaram (311/0), as 10 seguintes não carregaram.
  2. 1323 PASSA, 1 FALHA (`tela-centro` 7, nota do `askPath`) e a `tela-builder` abortada em 79: os dois problemas de espera acima, corrigidos nas baterias. Cada uma passou sozinha depois (94 e 102).
  3. **Final, página `693f69dd`: 1347 PASSA, 0 FALHA**, `rodar.js` saída 0 em 1253 s. Por bateria: cartão 37, zero 29, origem 32, nomes 12, passo2 9, 4a 30, 4b 56, passo6 36, etapa2c 45, evidência 25, estado 59, navegação 139, inspeção 98, centro 94, builder 102, rascunho 73, perfil 150, conversa 102, projeto 219.
  4. Depois da execução final, a `tela-projeto` foi dividida em blocos (só estrutura) e rodada de novo sozinha contra a página final: **219 PASSA, 0 FALHA**.
- **Raízes de teste:** `send.json` 844 → 844, `question.json` 1420 → 1428 (as 8 do `etapa2c`), perfis iguais em bytes.
- **Corpos 9.5:** **21 de 21 iguais ao F0.** A captura da execução final deu 20 de 21: a sequência S8 não registrou o `/api/ask`. Investigado por sonda: depois do clique em Perguntar, `/api/performance-question` sai a ~0,9–1,2 s e `/api/ask` a ~2,2–2,5 s, **também com a página do fim do F9** (0,87 s e 2,2 s), e a captura gravava 1,2 s depois do clique. O pedido não mudou; a espera do instrumento é que não bastava nesta máquina. A captura (instrumento no scratchpad; a referência do F0 não mudou) passou a esperar 6 s e o grafo da run; com isso, **21/21**, e os fluxos, modos, sondas, laço de Separar e restauração das raízes iguais aos da captura do fim do F9. O "1 erro" da captura é o `file_mode` conhecido (pendência 18).
- **Contraprova** (a `tela-projeto` nova contra a página do fim do F9): **152 PASSA, 22 FALHA**. Falham exatamente os comportamentos corrigidos: os blocos 9a (M1) e 9b (carregamento independente) não terminam, porque a página antiga fica carregando; o M2 (F7 decorado não é juntado; o bloco 9c aborta ali); os campos do perfil abertos durante a abertura; Cancelar do T25 sem devolver o caminho (4); reabrir o mesmo projeto pergunta (9g aborta); a linha de base com a versão trocada (2); Run sem link (9i aborta); os critérios do rascunho (3; 9j aborta); os textos de vazio (3, contando o caso 6). Passam o que já funcionava: o dado real do F7, Juntar durante investigação e conflito, Gerar guardando a run, "não aprovado nesta versão", os vazios do resumo. Antes desta divisão em blocos, a mesma contraprova abortava no primeiro caso do M1 (137 PASSA, 2 FALHA).
- **Rust:** `cargo test --locked -p librarian-studio` 185 aprovados; `cargo test --locked --workspace` 432 aprovados, 2 ignorados; `cargo build --release --locked` concluído.
- **`git diff --check`:** sem problemas; os 11 arquivos não rastreados conferidos à parte (sem espaço no fim de linha).

#### 12.13.8 Material

`C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\6e1931b3-8608-4155-8c9d-3699327ae1e0\scratchpad\f9fix\` (`final.sh`, `so.sh`, `sobe.sh`, `desce.sh`, `uma.sh`, `checa.sh`, `captura.js`, `compara.js`, `compara-fluxos.js`, `secao9.js`, `base-f9\` com a página, as baterias e os documentos do fim da auditoria, `m2*.mjs` com as consultas aos dados reais, `pw\` com o `playwright-core`, `log\`).

---

## 13. F10 — Revisão parcial e lotes

**Situação:** [Implementado] [Verificado] em 13/11/2026 (sessão `7c8c75ba-7413-4e03-b80e-5fe43ddf38c1`). Registrado como concluído no fechamento documental do mesmo dia, a pedido do usuário. As decisões de 13.1 são do usuário; as de 13.7 foram tomadas pelo Claude e **não tiveram aprovação específica**: foram registradas a pedido do usuário, mas não aprovadas uma a uma.

### 13.1 Autorização e decisões do usuário antes de implementar

[Aprovado] No início da sessão, antes de qualquer alteração (o prompt do F10 condicionava o início à aprovação do F9):

- **F9 aprovado**, com o fechamento e as decisões das pendências 57, 66, 67 e 68.
- **Escopo: a especificação (4.1 a 4.5) mais o "Aprovar todos" existente**, com prévia conferível e resultado por FACT. Sem nenhuma ação nova de recusa coletiva.
- **Esclarecimento enviado pelo usuário durante a sessão:** são **três operações distintas**, que não se misturam:
  1. **Revisão individual** (4.1 a 4.3): a leitura de uma pergunta preparada **antes** do envio. Preparar não envia; Enviar é explícito. As decisões por claim do F8 continuam existindo **depois** do envio, na conversa.
  2. **Envio em lote** (4.5): N perguntas independentes; "Enviar as marcadas" é a decisão coletiva de **iniciar vários envios**, um por pergunta, e **não** uma aprovação coletiva do conteúdo ou dos fatos.
  3. **"Aprovar todos"**: funcionalidade existente e separada, sobre os FACTs das respostas, com prévia e resultado por item; não contorna nenhuma regra da aprovação individual.
- Proibido: "Recusar as marcadas", recusa coletiva de FACTs, transformar o envio do lote em aprovação, duplicar as decisões por claim do F8, estados ou transições fora da especificação.

### 13.2 Revisão individual (4.1 a 4.3, T10 a T12, T19)

- **Preparar não envia** [Implementado] [Verificado]. Nas rotas 3 e 4 da 3.11 (Builder e achado do inventário), Preparar cria a pergunta (`POST /api/gathered-question`, corpo inalterado) e abre a **Revisão**, sem `POST /api/send`. O botão do compositor diz "Preparar ({n} fonte(s))" ou "Preparar o achado ({n})", e o caminho previsto acrescenta "Preparar abre a Revisão (parcial); o envio é um passo à parte, na Revisão" (coluna "Revisão na v1" da 3.11; resolve a pendência 37). A pergunta livre (rota 5, `/api/ask`) continua preparando e enviando como antes (4.4: sem revisão).
- **Vista central "Revisão"** (1.4): uma instância (uma pergunta ou um lote); abrir outra substitui sem perda; fechar só fecha. Aba "Revisão #N" ou "Revisão do lote (N)" na barra de vistas, com o estado "enviando k de N"; na tela estreita, o alternador diz "Centro: Revisão #N".
- **"Ver revisão"** em cada pergunta de uma rota que tem revisão (Builder, achado, lotes: perguntas com `gathered` que não são da escada). As outras rotas (escada, cadeia, perguntas por caminho, `/api/ask`) dizem "Sem revisão nesta rota" (7.4, "Indisponível").
- **Os sete blocos da 4.1**, cada um com o rótulo obrigatório: a pergunta preparada (o `turn.query` exato, 9.6.1); o pacote gravado ("Pacote como foi gravado ao preparar"); a política e os critérios usados (o bloco do F8, "Gravados com a pergunta; podem diferir do perfil atual"); as conferências desta consulta (H1 a H3 refeitos, H4 informativo; "Conferido nesta consulta; o envio confere de novo. Conferido não quer dizer aprovado"); o escolhido para o envio (provedor, modelo, Modelfile, skills e o texto do SYSTEM lido de `/api/modelfiles`, com o rótulo do D4; acompanha o compositor enquanto a Revisão está aberta); "Só você vê" (do registro da seleção: lacunas e candidatos, deixadas de fora com o motivo, não juntadas, removidas, papel, categoria e justificativas das incluídas; a divergência da medição no formato 2 e o aviso no formato 1; "não vai ao modelo"); e **"Não conferido nesta revisão"**, sempre presente, com o texto da 7.3. As leituras (`/api/evidence` e `/api/facts`) são mostradas cada uma quando chega; uma resposta que chega depois de a Revisão ser substituída ou fechada, ou de a run mudar, é descartada.
- **Enviar** (4.2, T12) é a única ação que envia, com o LLM do compositor. Fica desabilitado enquanto as conferências carregam, quando não puderam ser lidas (com "Ler de novo") e quando **H1, H2 ou H3 foi recusada** nesta consulta, com o texto do servidor como veio (9.6.2). "Not applicable" (pergunta anterior às marcas) não desabilita: o envio decide, como sempre. Com "Nenhum", Enviar mostra a query na pergunta para copiar e colar, sem pedido ao servidor.
- **Depois do envio:** o `evidence_sha256` do vínculo ao lado do hash visto na Revisão (`mark.sha256`), "iguais" ou "diferentes", com o rótulo "Comparação feita pela página, depois do envio" (4.2). Falha do envio: a mensagem do servidor na Revisão e na própria pergunta; a pergunta continua preparada.
- **Fato revogado (4.3):** antes de enviar, a página relê `GET /api/facts`. Um fato de `reviewed_facts` do pacote revogado aparece como "O fato F foi revogado depois da preparação. Recomendado: preparar de novo.", com "Voltar ao rascunho"; **Enviar continua possível** e nada diz que o servidor bloqueou. Releitura que falha: "Não foi possível conferir revogações antes do envio". Ver a decisão 1 de 13.7.
- **As decisões por claim continuam só na conversa** (camada 3 do F8). A Revisão não tem Aprovar nem Recusar; depois do envio diz que a resposta e as decisões ficam na pergunta, e "Abrir na conversa" ativa a pergunta.
- **Voltar ao rascunho** (T19) abre o Builder; a pergunta continua preparada.
- **T17:** a Revisão de uma pergunta é guardada em `center` da navegação do projeto e refeita do registro ao recarregar, se a pergunta existir na run aberta. Um lote não é refeito (só existe na página): recarregar mostra a Cidade.

### 13.3 Lote (4.5, T7)

- **Criação:** Separar e Conferir fatos criam as perguntas **pelo comportamento atual**: uma por item, cada uma com **uma** fonte, o modelo fixo (qwen) e o modo Localizar (T26 se a política for outra; cancelar não cria nada). **Nada é enviado na criação.** As criações são uma depois da outra; **a falha de uma não interrompe as demais** (o [Validar] da 4.5: o laço anterior parava no primeiro erro; agora segue) e aparece no próprio item.
- **Revisão do lote:** cada item com o próprio texto, fonte, query, H1 a H3 e estado; o modelo fixo dito; o texto "Enviar as marcadas inicia um envio por pergunta; não aprova nada"; "Não conferido nesta revisão". Todas vêm marcadas; a pergunta com **H1 a H3 recusada** (ou preparação recusada, ou conferências ilegíveis) é **desmarcada automaticamente, com o motivo, e não pode ser marcada**.
- **Seleção conferível:** marcar e desmarcar depois de criadas e antes de enviar; o botão diz "Enviar as marcadas (k de N)". Desmarcar antes da criação continua fora da v1.
- **Envio:** "Enviar as marcadas" envia **cada pergunta marcada sozinha, uma por vez** ("Enviando k de N" no lote e na aba), cada uma na própria pergunta e na run dela, com o modelo fixo. **A falha de uma não interrompe as outras** e aparece no próprio item e na própria pergunta, nunca em outra. As desmarcadas ficam "Preparada, não enviada" e não são enviadas. No fim, "Envio do lote concluído: {enviadas}, {falhas}, {não enviadas}" (7.2). Nenhum `POST /api/review` sai do lote.

### 13.4 "Aprovar todos" (existente, separado do lote)

- O botão passou a se chamar **"Aprovar todos…"** (era "Aprovar em lote"), em Onde pesa e no cartão de desempenho, e só aparece para o último lote, enquanto a run dele está aberta. Não está na Revisão do lote.
- **Prévia** antes de qualquer decisão: os FACTs que serão aprovados (pergunta, envio, claim, tipo e texto) e os que não serão, cada um com o motivo: já aprovado (com o fato) ou já recusado; LACUNA e INFERENCE (os mesmos motivos da aprovação individual); outro tipo ("Aprovar todos" só aprova FACT); envio que não passou na conferência mecânica; envio sem vínculo; pergunta sem envio ou sem itens. A regra de aprovável é a da aprovação individual (`reviewView`): FACT, envio com conferência mecânica aprovada e vínculo, item ainda não decidido.
- **Nota obrigatória**, como na aprovação individual. Cancelar ou deixar a nota vazia não decide nada. Sem nenhum aprovável, só a prévia, sem nota.
- **Releitura antes de executar:** um item decidido entre a prévia e a confirmação não é aprovado de novo ("decidido depois da prévia").
- **Aprovações individuais:** um `POST /api/review` por item, cada um com resultado próprio; o servidor confere de novo cada um. **Resultado por item**, com falha parcial (a mensagem do servidor como veio), e um resumo na nota.

### 13.5 Guardas (pendência 4, 8.4 item 8)

- Cada lote pertence à run e ao projeto em que foi criado (`hotBatch = {home, ids}`, no lugar de `hotTurns`).
- **Trocar de run ou de projeto fecha a Revisão** (T14). As perguntas do lote que ainda não foram criadas não são criadas; as que ainda não foram enviadas não são enviadas; a resposta de um envio em andamento é gravada na run dela e dita como de outra run, sem entrar na conversa aberta.
- **"Aprovar todos" opera só sobre a run do lote**; trocar de run esquece o lote (ver a decisão 7 de 13.7).
- `sendTurn` passou a devolver o registro do envio, para a Revisão e o lote dizerem o resultado da própria pergunta sem depender de reler `/api/turns`.

### 13.6 Textos, idiomas, arquivos

- Cerca de 110 chaves novas em PT e EN (`review.*`, `lot.*`, `approve.*`, `route.review`, `composer.prepare*`, `col.review`, `help.review`); as chaves `hot.asking`, `hot.asked` e `hot.approved` saíram (sem uso); `help.hotSeparate` e `hot.approveAll` foram reescritas.
- **Página:** 8233 linhas, hash `b89f8d4117637ade` (era 7676, `693f69dd`); +626/−69 linhas contra o fim do F9. O diff não acrescenta nenhuma rota nem tipo de escrita novo: as duas chamadas de escrita que aparecem como acrescentadas (`POST /api/gathered-question` no lote e `POST /api/review` em "Aprovar todos") foram movidas de `askFacts`/`askHotFacts` e de `approveHotFacts`. Nenhum `.rs` ou `.toml` mudou.

### 13.7 Decisões de implementação tomadas durante o F10

Tomadas pelo Claude onde a especificação não decide e apresentadas no relatório do F10; **sem aprovação específica**:

| # | Decisão | Motivo |
|---|---|---|
| 1 | Uma revogação **nova**, encontrada pela releitura no momento do clique em Enviar, impede **aquele** envio: o aviso aparece e o clique seguinte envia | Para o aviso da 4.3 ser visto antes do envio, sem bloquear (a especificação não diz o que fazer quando a revogação aparece no clique) |
| 2 | Pergunta do lote cujas conferências não puderam ser lidas é desmarcada, com o motivo | Sem H1 a H3 lidos, a 4.5 não tem como dizer se a pergunta passa |
| 3 | Pergunta do lote cujo envio falhou continua marcada: um novo "Enviar as marcadas" a reenvia (a contagem do botão a inclui) | Permite nova tentativa sem marcar de novo; as já enviadas não vão de novo |
| 4 | A vista guardada é `{view: 'review', turn}`, sem a run (o formato da 5.3; o módulo do F1 não guarda outros campos). Trocar de run no mesmo projeto com uma Revisão guardada devolve a vista guardada à Cidade | Um ponteiro sem run nunca pode reabrir a pergunta de mesmo número em outra run |
| 5 | As rotas sem revisão dizem "Sem revisão nesta rota" na própria pergunta | 7.4, célula "Indisponível" da Revisão |
| 6 | "Aprovar em lote" passou a "Aprovar todos…" | Não confundir com o lote da 4.5 |
| 7 | Trocar de run esquece o lote (`resetHot`, como já acontecia com `hotTurns`) | Comportamento anterior mantido; "Aprovar todos" deixa de estar disponível para aquele lote |

### 13.8 Requisitos e critérios de aceitação

| Requisito | Situação |
|---|---|
| T10 Preparar (rotas 3 e 4) não envia e abre a Revisão | [Verificado] (`tela-revisao` A1) |
| 4.1 Os sete blocos com os rótulos | [Verificado] (A1); "Só você vê" mostra o que o registro da seleção tem: comportamento e ambiguidades não aparecem como itens próprios (as lacunas incluem os candidatos de homônimos). **Parcial** |
| 9.6.1 "A pergunta preparada" igual ao `turn.query` | [Verificado] (A1, A4) |
| 9.6.2 H1 a H3 recusadas desabilitam Enviar com o texto do servidor | [Verificado] **com resposta simulada** de `/api/evidence` (H2 recusada); os dados reais não têm caso recusado |
| 9.6.3 "Não conferido nesta revisão" sempre presente | [Verificado] (individual e lote, PT e EN) |
| 9.6.4 Nenhuma frase proibida | [Verificado] **em parte**: busca por expressões regulares na Revisão (PT e EN) e em todas as chaves do F10; não é uma busca na página inteira |
| 9.6.5 Escada com o rótulo do C1 | Comportamento do F7, mantido; não reconferido no F10 |
| 9.6.6 Fato revogado entre preparar e enviar: aviso, sem bloqueio | [Verificado] **com respostas simuladas** (`/api/facts` com o fato revogado e o pacote com `reviewed_facts`); os dados reais não têm fato revogado num pacote |
| 4.2 Enviar explícito, hash depois do envio, falha | [Verificado] com envios respondidos pela bateria (nenhum modelo chamado) |
| T19 Voltar ao rascunho | [Verificado] |
| T17 Revisão refeita ao recarregar | [Verificado] (A5), inclusive pergunta inexistente e troca de run |
| 9.7.1 N itens, N perguntas, uma fonte cada | [Verificado] **no pedido**: um corpo por linha, cada um com uma fonte; as perguntas foram respondidas pela bateria, então o pacote gravado pelo servidor não foi conferido |
| 9.7.2 Nenhum pacote com fonte de outra pergunta | **Parcial:** conferido nos corpos dos pedidos (a fonte de cada item no próprio corpo), **não** nos `evidence.json`, como a 9.7.2 pede |
| 9.7.3 Desmarcadas ficam preparadas, não enviadas | [Verificado] |
| 9.7.4 Falha de uma não impede as outras e aparece na própria | [Verificado] **com falha de provedor simulada** (resposta 502 da bateria), **não** com um pacote adulterado numa cópia |
| 4.5 Criação com falha não interrompe | [Verificado] na bateria (500 simulado) e na captura (sonda "separar_falha_no_meio": 3 de 3 criações pedidas; no F9, 2 de 3) |
| "Aprovar todos": prévia, motivos, nota, releitura, resultado por item, falha parcial | [Verificado] com respostas simuladas de `/api/turns` e `/api/review` |
| Guardas de run e projeto, respostas atrasadas | [Verificado] (A4, B5, B6, C6) |
| 9.12 PT/EN, tema escuro, tela estreita, teclado | [Verificado] em parte: EN, contraste do rótulo no tema escuro, tela estreita sem rolagem horizontal, Tab e Enter na aba, foco visível em Enviar, espaço numa casa do lote; diálogos e Aprovar todos pelo teclado não foram conferidos à parte |

### 13.9 Testes do F10

- **Bateria nova `tela-revisao.js`: 158 verificações**, em blocos (uma exceção vira a FALHA do bloco e o seguinte continua), separadas em A (Revisão individual), B (lote) e C ("Aprovar todos"), mais D (EN, tema escuro, tela estreita, teclado). Os casos que os dados reais não têm são **simulados** pela bateria e declarados no próprio arquivo: H recusada, fato revogado num pacote, as perguntas criadas pelo lote (cópias da #135 com outro id), os envios e as respostas; nada chega a um modelo nem ao servidor (as escritas são respondidas ou abortadas e contadas). **Uma verificação com dado simulado prova o comportamento da página diante daquela resposta, não o comportamento do servidor.**
- **Expectativas antigas alteradas** (mesmas contagens), porque conferiam a ausência do F10:
  - `tela-perfil` (2): o texto exato do caminho previsto das rotas 3 e 4 ganhou a linha da Revisão.
  - `tela-centro` 1: a barra de vistas tem 4 vistas (Cidade, Fluxograma, Builder, Revisão); a Comparação continua fora.
- `rodar.js` inclui a `tela-revisao`; o README das baterias ganhou a linha dela.
- **Execução completa** (raízes recopiadas das intactas, binário de release do código atual): **1505 PASSA, 0 FALHA**, `rodar.js` saída 0 em 1254 s. Por bateria: cartão 37, zero 29, origem 32, nomes 12, passo2 9, 4a 30, 4b 56, passo6 36, etapa2c 45, evidência 25, estado 59, navegação 139, inspeção 98, centro 94, builder 102, rascunho 73, perfil 150, conversa 102, projeto 219, revisão 158. As 19 baterias anteriores tiveram as mesmas contagens do fechamento do F9.
- **Raízes de teste:** `send.json` 844 → 844, `question.json` 1420 → 1428 (as 8 do `etapa2c`), perfis iguais em bytes.
- **Corpos 9.5: 21 de 21 iguais ao F0.** Isso **não** quer dizer que nada mudou em relação ao F9: os corpos comparados são os de `/api/gathered-question` e `/api/ladder`, que o F10 não altera. **As diferenças intencionais** aparecem na comparação dos fluxos contra a captura do fechamento do F9: modos, sondas e restauração das raízes iguais; os fluxos `separar_askHotFacts` e `conferir_fatos_askFacts` diferem só no texto da nota (as perguntas ficam preparadas, não enviadas); a sonda `separar_falha_no_meio` passou de 2 para 3 criações pedidas (o laço não para mais na primeira falha). O único erro da captura é o `file_mode` conhecido (pendência 18).
- **Contraprova** (a `tela-revisao` contra a página do fim do F9): **4 PASSA, 12 FALHA.** Mostra que na página anterior o botão diz "Perguntar com o envelope", o caminho previsto não fala da Revisão, não há "Ver revisão" nem "Sem revisão nesta rota", e **Preparar dispara um `POST /api/send`** (a escrita que escapou é esse envio, abortado pela bateria). Cada bloco aborta ao esperar uma Revisão que não existe: é uma **contraprova grosseira**, que não mostra, caso a caso, quais das outras verificações falhariam.
- **Rust:** `cargo test --locked -p librarian-studio` 185 aprovados; `cargo test --locked --workspace` 432 aprovados, 2 ignorados; `cargo build --release --locked` concluído. `git diff --check` sem problemas; os arquivos não rastreados sem espaço no fim de linha.
- **Observação de processo:** duas edições estruturais da `tela-revisao` (o gancho que tolera nomes ausentes e a divisão em blocos) foram feitas durante a execução completa, antes de ela chegar a essa bateria, que é a última; a execução usou o arquivo atual.

### 13.10 Limitações conhecidas

- **Revisão parcial** [Limitação]: a identidade entre a revisão e o envio (D3), SYSTEM e skills (D4) e a recusa por fato revogado no servidor (D7) dependem do backend (B3, Etapa 5).
- 9.7.4 testado só com falha de provedor simulada; 9.7.2 conferido nos pedidos, não nos `evidence.json`.
- Contraprova grosseira (13.9).
- O ponteiro da Revisão guardada não tem a run: se **outra aba** mudar a run guardada do projeto, recarregar pode abrir a Revisão de uma pergunta de mesmo número na run restaurada.
- O lote não sobrevive ao recarregamento nem à troca de run (está só na página).

### 13.11 Material

`C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\7c8c75ba-7413-4e03-b80e-5fe43ddf38c1\scratchpad\f10\` (`final.sh`, `uma.sh`, `so.sh`, `sobe.sh`, `desce.sh`, `contra.sh`, `captura.js`, `compara.js`, `compara-fluxos.js`, `dados.mjs` e `dados2.mjs` com a busca dos casos nos dados reais, `base-f9\` com a página, as baterias e os documentos do fim do F9, `f10-index.diff`, `old-lotes.js` e `novo-lotes.js`, `strings.js`, `log\` com `final.log`, `rodar.log`, `captura.log`, `contra.log`, `cargo.log` e os logs intermediários, `consolidacao\` com os documentos antes deste fechamento documental).

---

## 14. Linha histórica das correções e decisões complementares

Em ordem cronológica (11/11/2026 até a linha 12; 12/11/2026 a partir da 13; nada commitado):

| # | Momento | Correção ou decisão | Página resultante |
|---|---|---|---|
| 1 | Durante o F2 | [Aprovado] Seleção leva a Explorar › Estrutura até o F3; `librarian.tab` convertida e apagada só após gravar a navegação | `9cde9b73` |
| 2 | Durante o F2 | `resize` ignora tamanho zero (cidade em branco na tela estreita) | `9cde9b73` |
| 3 | Revisão do F2 | Vazamento de navegação entre projetos corrigido; abrir não grava navegação | `312a970e` |
| 4 | Revisão do F2 | `#announce` na árvore de acessibilidade na tela estreita | `312a970e` |
| 5 | Revisão do F2 | Abrir arquivos volta a começar fechado no Studio | `312a970e` |
| 6 | Antes do F3 | [Aprovado] Valem T1 e o destino de Inspecionar da especificação | — |
| 7 | Pendências do F3 | [Aprovado] Voltar só no projeto aberto; abrir outro projeto limpa a pilha; especificação 5.3 e T18 editadas | `5989913a` |
| 8 | Pendências do F3 | README das baterias: contagens corrigidas na execução | — |
| 9 | Correção final do F3 | Projeto sem runs limpa o grafo anterior; troca de projeto fecha o fluxograma | `8a7592f2` |
| 10 | F4 | `#chatNote` como lugar do aviso do `askPath` (as outras notas são reescritas por `refreshComposer()`) | `a0d5140f` |
| 11 | Consolidação (este documento) | README das baterias ganha `tela-centro.js`; README do Studio aponta para este documento | `a0d5140f` (sem mudança de código) |
| 12 | Ajustes da consolidação | README do Studio: "ainda não implementado" substituído pela situação até o F4 (pendência 19); README das baterias ganha a linha de `tela-evidencia.js`, 25 verificações, conferidas no script e na execução do F4 (pendência 21) | `a0d5140f` (sem mudança de código) |
| 13 | F5 | Evidence Builder implementado (seção 8), aguardando avaliação | `0f47bab4` |
| 14 | Validação do F5 | Correções antes do relatório: contraste do texto auxiliar no tema claro; estado de carregamento na aba; nome do alvo com o tipo; desenho tolerante do Builder (`tela-4b` abortava) | `0f47bab4` |
| 15 | Consolidação do F5 | Este documento ganha a seção 8 (as seguintes foram renumeradas: 9 a 14); README do Studio passa a descrever a situação até o F5 | `0f47bab4` (sem mudança de código) |
| 16 | Antes do F6 | [Aprovado] Critérios do rascunho (5.4, 5.5 passo 7, bloqueio da 3.12) ficam para o F7; restaurar a run ativa fica para o F9 (seção 9.1) | — |
| 17 | F6 | Rascunho guardado e restaurado por projeto e run (seção 9), aguardando avaliação | `a5c0023b` |
| 18 | Validação do F6 | Correções antes do relatório: aviso da run deixada apagado pela primeira gravação vazia; texto do compositor num projeto sem runs; gravação da própria aba antes de carregar a da outra (`draftTaking`); vista do Builder lida da navegação guardada (T17); uma aspa sem escape no dicionário EN | `a5c0023b` |
| 19 | Consolidação do F6 | Este documento ganha a seção 9 (as seguintes foram renumeradas: 10 a 15); auditoria de `tela-builder.js` (seção 9.11) corrige o relatório do F6: 8 verificações alteradas (não 9) e a versão do F5 recuperável no material do F5; README do Studio e das baterias conferidos | `a5c0023b` (sem mudança de código) |
| 20 | Antes do F7 | [Aprovado] O F7 remove todas as gravações implícitas (inclusive Gerar, Exportar, Sugerir e Aplicar léxico); marca o não salvo e mostra o indicador no cabeçalho (T25 e o aviso ao recarregar ficam para o F9); critérios só no compositor, e "Salvar como padrão" grava só `criteria` (seção 10.1) | — |
| 21 | F7 | Compositor e gravação do perfil (seção 10), aguardando avaliação | `fa167e09` |
| 22 | Validação do F7 | Correções antes do relatório: o seletor, desabilitado durante a confirmação, perdia o foco (passou a ser desabilitado só durante a gravação, e o foco volta); quem responde passou a ser lido no clique, antes do T26 (a pergunta saía para o LLM lembrado do outro modo) | `fa167e09` |
| 23 | Consolidação do F7 | Este documento ganha a seção 10 (as seguintes foram renumeradas: 11 a 16); README do Studio e das baterias conferidos | `fa167e09` (sem mudança de código) |
| 24 | Antes do F8 | [Aprovado] F5, F6, F7 e a consolidação do F7 aprovados pelo usuário; F8 autorizado (seção 11.1) | `fa167e09` |
| 25 | F8 | Conversa e histórico (seção 11), aguardando avaliação | `dbc72629` |
| 26 | Validação do F8 | Correções antes do relatório: leitura da política usada (ausência de `allow_edits` = não); contraste no tema claro; detalhe das execuções montado ao abrir; espera pela medição em `tela-perfil` (+2 linhas); execução completa refeita por porta ocupada (seção 11.10) | `dbc72629` |
| 27 | Consolidação do F8 | Este documento ganha a seção 11 (as seguintes foram renumeradas: 12 a 17); F5 a F7 marcados como aprovados; READMEs do Studio e das baterias atualizados | `dbc72629` (sem mudança de código) |
| 28 | Antes do F9 | [Aprovado] F8 e a consolidação do F8 aprovados; F9 autorizado com o escopo completo da especificação; T25 com três escolhas; aviso ao sair só para edições do perfil (seção 12.1) | `dbc72629` |
| 29 | F9 | Projeto, Régua e Acervo (seção 12), aguardando avaliação | `b6fcff72` |
| 30 | Validação do F9 | Correções antes do relatório: o Enter no campo do caminho também apertava "Salvar e abrir" (`preventDefault`); três expectativas antigas ajustadas depois da primeira execução completa (1262/3); comentário desatualizado sobre o F9 na página (seção 12.11) | `b6fcff72` |
| 31 | Auditoria e consolidação do F9 (13/11) | Auditoria contra a especificação, o código, os diffs e os logs: dois defeitos médios (M1, M2), requisitos parciais e decisões classificadas (seção 12.10); este documento ganha a seção 12 (as seguintes foram renumeradas: 13 a 18); F8 marcado como aprovado; READMEs do Studio e das baterias atualizados | `b6fcff72` (sem mudança de código) |
| 32 | Fechamento do F9 (13/11) | [Aprovado] decisões do prompt do fechamento e, na sessão, Contagens manuais → Inventário. M1, M2, carregamento independente, edição durante a abertura, textos de vazio, Gerar, `tela-centro` 1 exata, Cancelar do T25, reabrir sem perda, critérios do rascunho, linha de base pela escada; testes de regressão; consolidação (seção 12.13) | `693f69dd` |
| 33 | Antes do F10 (13/11) | [Aprovado] o F9, com o fechamento e as pendências 57, 66, 67 e 68, aprovado; escopo do F10: especificação 4.1 a 4.5 mais "Aprovar todos" com prévia; o esclarecimento do usuário separa Revisão individual, envio em lote e "Aprovar todos" (seção 13.1) | `693f69dd` |
| 34 | F10 (13/11) | Revisão parcial, lotes, "Aprovar todos" com prévia, guardas de `askFacts` e `askHotFacts`; bateria `tela-revisao` (158); `tela-perfil` e `tela-centro` com expectativas ajustadas (seção 13) | `b89f8d41` |
| 35 | Fechamento documental do F10 (13/11) | Este documento ganha a seção 13 (as seguintes foram renumeradas: 14 a 19); F9 marcado como aprovado; pendências 4, 12, 37, 57, 66, 67 e 68 resolvidas e 70 a 76 novas; README do Studio com a situação até o F10. Corrigidos dois defeitos de edição herdados do fechamento do F9: uma cópia solta de 15 trechos da tabela de pendências antes do título (os trechos continuam íntegros na tabela) e caracteres de controle no lugar das barras invertidas na linha do material do F9 (seção 18.4) | `b89f8d41` (sem mudança de código) |

---

## 15. Registro de decisões funcionais aprovadas

| Assunto | Decisão aprovada | Comportamento esperado | Etapa | Observação |
|---|---|---|---|---|
| Navegação por projeto | Navegação isolada por projeto, em `librarian.nav.v1:<key>` | Cada projeto recupera a própria área e vista; nunca herda a de outro | F1, F2 (correção) | [Implementado] |
| Abrir projeto | Abrir não grava navegação | Só movimentos da pessoa gravam | F2 (correção) | [Implementado] |
| Voltar | Só dentro do projeto aberto; trocar de projeto limpa a pilha | Voltar nunca abre projeto; voltar a outro exige abri-lo | Pendências do F3 | [Implementado]; na especificação, 5.3 e T18 |
| `librarian.tab` | Convertida uma vez pela tabela 1.3; removida só após gravar a nova navegação | Navegação guardada prevalece; valor sem destino fica | F2 | [Implementado] |
| Seleção em Explorar | T1: inspeção sobre a lista da vista atual, com ponto no Voltar | — | F3 | [Implementado] |
| Seleção fora de Explorar | T2: área não muda; "Selecionado: X [Inspecionar]" | Só Inspecionar muda a área | F3 | [Implementado] |
| Destino de Inspecionar | A vista guardada de Explorar; Estrutura só sem vista válida | — | F3 | [Implementado] |
| Voltar à lista | Fecha a inspeção e restaura a lista; não grava ponto no Voltar | — | F3 | [Implementado] |
| Preservação das listas | Consultas, filtros, dobras, resultados e rolagem preservados | Só na memória | F3 | [Implementado] |
| Projeto sem runs | Cidade vazia; nada do grafo, seleção, inspeção, fluxograma, envelope ou pergunta do anterior | Navegação, perfil e rascunhos de cada projeto preservados | Correção do F3 | [Implementado] |
| Cidade como base | A cidade é a base do centro e não fecha | Vistas abrem por cima | Especificação 1.4; F4 | [Implementado] |
| Fluxograma como vista | Uma instância; substitui com confirmação se há escolhas; fechar descarta (com confirmação se há escolhas); ir à Cidade só esconde | — | Especificação 1.4 e 2.3; F4 | [Implementado] |
| Troca de run | As vistas de leitura da run anterior fecham (T14) | O fluxograma fecha | Especificação T14; F4 | [Implementado]; sem confirmação (ver pendência 2) |
| Restauração da vista central | Só "quando dá para refazer" (T17); o fluxograma não é refeito ao recarregar (2.3) | Cidade | Especificação; F4 | [Implementado] |
| Builder | Vista central da especificação; destino único de montagem; fechar só esconde (1.4, 2.4) | — | Especificação; F5 | [Implementado] [Aprovado] com a etapa (12/11); as decisões de 8.7, aprovadas com a etapa |
| Rascunho no navegador | Um por projeto e run; nunca transferido; incompatíveis não aplicados nem sobrescritos; duas abas sem mesclar (5.4 a 5.8) | Restaura ao recarregar e ao voltar à run | Especificação; F6 | [Implementado] [Aprovado] com a etapa; as decisões de 9.8, aprovadas com a etapa |
| Critérios do rascunho | Não guardados nem restaurados no F6; feitos no F7, só no compositor | Guardados no rascunho enquanto diferem do padrão; Preparar bloqueado com três opções; "Salvar como padrão" grava só `criteria` (seção 10.1, decisão 3) | F7 (decisões antes do F6 e do F7) | [Implementado] [Aprovado] com a etapa |
| Run ativa ao recarregar | O F6 não muda a escolha: abre a run mais recente; os rascunhos das outras runs voltam ao escolhê-las | Restaurar a última run ativa (2.1, T17, T25) | F9 (decisão antes do F6) | [Implementado] no F9 (seção 12.2); [Aprovado] com o F9, no início da sessão do F10 |
| Revisão, Comparação | Vistas centrais da especificação | Revisão no F10 (seção 13); Comparação no F12 | F10, F12 | Revisão [Implementado] [Verificado]; Comparação [Pendente] |
| Conversa (4.7, 4.8) | Linha do tempo da run; estados do registro; três camadas; critérios e política usados; Copiar diff (especificação) | Resposta de outra run ou projeto nunca aparece como desta; pergunta ativa guardada por run | Especificação; F8 | [Implementado] [Aprovado] com a etapa (12/11); as decisões de 11.8, aprovadas com a etapa |
| Modo e perfil | Salvar e Alterar a política como operações distintas; política do projeto como fonte de verdade | Nenhuma outra operação grava o perfil; T26 nas ações que exigem um modo | Especificação 3.13; F7 | [Implementado] [Aprovado] com a etapa; as decisões de 10.8, aprovadas com a etapa |
| Gravações implícitas fora do compositor | O F7 remove também as de Gerar, Exportar, Sugerir e Aplicar léxico, com as escolhas da especificação | — | F7 (decisão antes do F7, seção 10.1) | [Implementado]; o restante do F9 e do F11 continua com eles |
| Não salvo | Marcado em cada lugar e no cabeçalho | Confirmar ao trocar de projeto (T25) e o aviso ao recarregar ficam para o F9 | F7 (decisão antes do F7) | [Implementado] a marca (F7) e as confirmações (F9, seções 12.3 e 12.4) |
| Escopo do F9 | Tudo o que a especificação atribui ao F9: resumo, Régua, Acervo, T13, T25, T17 e a run guardada | — | Antes do F9 (seção 12.1) | [Implementado] |
| T25 | "Abrir sem salvar", "Salvar e abrir" e Cancelar; o mesmo em Limpar | Troca só depois de um Salvar que gravou tudo; Cancelar não muda nada | Antes do F9 (seção 12.1) | [Implementado] [Verificado] |
| Aviso ao sair (T17) | Edições não salvas do perfil, além dos rascunhos só na página; a resposta colada não conferida não avisa | O aviso é o do navegador e não guarda nada | Antes do F9 (seção 12.1) | [Implementado] [Verificado] |
| Linha de base em vigor | Critério da escada (2.5): qualquer versão do comando | A Régua diz quando a linha de base é de outra versão | Fechamento do F9 (12.13.1) | [Implementado] [Verificado] |
| Aprovação de outra versão | Não deduzir das execuções; "não aprovado nesta versão"; parcial até o servidor expor o histórico | — | Fechamento do F9 | [Limitação] |
| Destinos do resumo | Run → Régua sempre; Contagens manuais → Explorar › Inventário (decidido na sessão, no lugar do Acervo do prompt) | Sem tela nem recurso inventado | Fechamento do F9 | [Implementado] [Verificado] |
| Critérios do rascunho | Não provocam T25 nem aviso ao sair; ditos na linha do rascunho, separados de "Alterações não salvas" | Nenhuma gravação implícita | Fechamento do F9 | [Implementado] [Verificado] |
| Links sem voar | Régua, Acervo e resumo ativam a pergunta sem mover a câmera | — | Fechamento do F9 | [Implementado] (decisão 7 confirmada) |
| Cancelar o T25 | O campo do caminho volta ao do projeto aberto | — | Fechamento do F9 | [Implementado] [Verificado] |
| Reabrir o mesmo projeto | Sem confirmação quando nada é descartado | Releitura do registro; a página fica como estava | Fechamento do F9 | [Implementado] [Verificado] |
| Python com medição; provedor de teste | Limitação da versão atual; provedor falso só nos testes, nada na interface | — | Fechamento do F9 | [Limitação] |
| Aprovação do F9 | O F9, com o fechamento e as decisões das pendências 57, 66, 67 e 68, aprovado | — | Início da sessão do F10 (13.1) | [Aprovado] |
| Escopo do F10 | Especificação 4.1 a 4.5 mais o "Aprovar todos" existente, com prévia conferível e resultado por FACT; nenhuma recusa coletiva | — | Antes do F10 (13.1) | [Implementado] [Verificado] |
| Três operações distintas | Revisão individual (antes do envio; Enviar explícito), envio em lote ("Enviar as marcadas" inicia envios, não aprova) e "Aprovar todos" (depois das respostas, separado do lote); as decisões por claim continuam na conversa (F8) | Nenhuma se confunde com outra; nenhuma contorna a aprovação individual | Esclarecimento do usuário durante o F10 (13.1) | [Implementado] [Verificado] (`tela-revisao` A, B e C) |

---

## 16. Pendências e decisões em aberto

| # | Pendência | Origem | Situação atual | Etapa | Decisão humana? |
|---|---|---|---|---|---|
| 1 | Mudar a vista central deve criar ponto no Voltar? | F4 | Não cria: a vista central é independente do Voltar. [Recomendação] manter assim | — | **Sim, pendente** |
| 2 | Trocar de run com escolhas de homônimos no fluxograma deve pedir confirmação? | F4 | Fecha sem confirmação, conforme T14 (que não prevê confirmação) | F4 ou posterior | **Sim, pendente** |
| 3 | Restaurar a última run selecionada ao recarregar | F0, F1, F2 | **Resolvida no F9** (seção 12.2): a run escolhida pela pessoa fica em `nav.run` e volta ao recarregar e ao reabrir o projeto | F9 | Não (resolvida) |
| 4 | Guardas `sameRun` em `askFacts`, `askHotFacts` (lotes) e `continueTurn` (cadeia) | F0; 8.4 item 8 | `continueTurn` **resolvida no F8** (seção 11.2). `askFacts`, `askHotFacts` e `approveHotFacts` **resolvidas no F10** (seção 13.5): o lote pertence à run e ao projeto em que foi criado; trocar de run não cria nem envia o que falta; "Aprovar todos" só age na run do lote | F10 | Não (resolvida) |
| 5 | Gravação implícita do perfil (14 casos), sincronização do modo, T25, T26 | F0 | **Resolvida no F7** (seções 10.2 e 10.3): só Salvar e Alterar a política gravam; modo sincronizado; T26. **Fica para o F9** a confirmação do T25 com edições não salvas (pendência 35) | F7 | Não (decidido na especificação) |
| 6 | Léxico JSON inválido bloqueia `askModel`, escada, Gerar e lotes | F0 (lido no código) | **Resolvida no F7**: só o Salvar recusa o JSON inválido; perguntar e preparar não são mais bloqueados (`tela-perfil`, caso 4) | F7 | Não (resolvida) |
| 7 | Chave `librarian.modelfile`, gravada e nunca lida | F0 | Sem mudança | F11 | Sim (com a Aberta 3 da especificação, sobre `librarian.mode`) |
| 8 | Ambiguidade de nomes: D1–D11 (dependências, especificação) × D1–D14 (defeitos corrigidos, PLANO) | F0 | Sem mudança documental | — | Sim (prefixo ou outra forma) |
| 9 | Validação em Ubuntu e com CRLF | Todas | Só Windows | F13 | Não |
| 10 | Dependência do CDN jsDelivr (Three.js, elkjs) | F0 | A cidade, o fluxograma e as baterias precisam de rede | — | Não |
| 11 | Teste de isolamento com dois projetos reais com runs | Correções do F3 | Validado com projetos fictícios (respostas simuladas); no F9, a `tela-projeto` usa o mesmo recurso para a run guardada por projeto e o T25 | — | Não |
| 12 | `#centerBar` ainda não tem lugar para Builder, Revisão e Comparação | F4 | Builder acrescentado no F5; **Revisão acrescentada no F10** (seção 13.2); a Comparação continua fora, de propósito | F12 | Não |
| 13 | Cabeçalho sem "Rascunho" e "Não salvo" | F2 | "Rascunho" feito no F5 e completado no F6 ("· guardado neste navegador" ou "· só nesta página"); "Não salvo" **feito no F7** (seção 10.6) | F7 | Não (resolvida) |
| 14 | Tema "sistema" | F0, F2 | Só alterna e não guarda; nenhum passo o atribui | — | **Sim:** atribuição |
| 15 | Contagem de "pedidos pendentes" do Léxico (1.3) | F2 | Definição ambígua; só "vazio" implementado | — | Sim |
| 16 | Texto do servidor ainda diz "Investigar" (`questions.rs`) | F2 | Backend, fora do escopo | — | Não |
| 17 | Scripts de vídeo com seletores antigos (`#tabs`, `data-tab`) | F2 | Não ajustados | — | Não |
| 18 | A captura do F0 (`captura.js`) registra 1 erro em `file_mode` | F2 em diante | Esperado: procura `#tabs`, que só existia na página do F0; o modo arquivo é coberto pelas baterias | — | Não |
| 19 | O README do Studio dizia que o novo frontend estava "ainda não implementado" | Esta consolidação | **Resolvida** nos ajustes da consolidação: o README diz que a reformulação está em andamento, o que foi feito até o F4 e que Builder, revisão e comparação são de etapas posteriores. Atualizado de novo na consolidação do F5 (situação até o F5) | — | Não (resolvida) |
| 20 | `STUDIO.md` descreve a página anterior à reformulação | — | Não atualizado | F13 | Não |
| 21 | `tela-evidencia.js` (25 verificações) não aparecia na tabela do README das baterias | F1 (achado) | **Resolvida** nos ajustes da consolidação: linha acrescentada, com 25 verificações | — | Não (resolvida) |
| 22 | As decisões de implementação do F5 (8.7) | F5 | **Resolvida:** aprovadas com a etapa (12/11) | F5 | Não (resolvida) |
| 23 | 9.2.7 com "Pedir esta mudança" sobre rascunho não vazio | F5 | Mesmo caminho de Otimizar (`restartDraft`), **não testado** nesse caso | F5 ou F13 | Não |
| 24 | 9.5.2: H3 e manifesto do histórico contra `linha-base-etapa4-20261107` | F5 | **Não verificado separadamente** (nenhum código Rust mudou) | F13 ou quando autorizado | Não |
| 25 | `setMode` ainda chamado por Otimizar, pelo cartão e pelo achado; o modo necessário só é mostrado | F5 (8.7, decisão 1) | **Resolvida no F7**: Otimizar, o cartão e o achado só registram o modo necessário; T26 ao agir | F7 | Não (resolvida) |
| 26 | O cartão "Pedir esta mudança" passou de 1 para 2 `PUT /api/project` implícitos (agora pela escada do compositor) | F5 | **Resolvida no F7**: o clique não grava; a escada pede o T26 e grava só a política (uma vez) | F7 | Não (resolvida) |
| 27 | Rascunho só na memória: recarregar e trocar de run o perdem; rótulo "só nesta página" coincide com o de armazenamento indisponível (5.8) | F5 (8.6) | **Resolvida no F6** (seções 9.2 a 9.7) | F6 | Não (resolvida) |
| 28 | Recusa acima do orçamento (o [Validar] da 3.4) | F5 | Não validado | — | Não |
| 29 | Juntar a declaração de um fato pelo Acervo (2.4.1, 2.6) | F5 | **Resolvida no fechamento do F9** (12.13.3): Rust e Python, com decoradores | F9 | Não (resolvida) |
| 30 | Critérios do rascunho: a parte de critérios da 5.4, o passo 7 da 5.5 e a relação com o bloqueio da 3.12 | Decisão antes do F6 | **Resolvida no F7** (seção 10.4), inclusive o passo 7 da 5.5. Mostrar na pergunta os critérios usados (9.3.4) é do F8 (pendência 36) | F7 | Não (resolvida) |
| 31 | As decisões de implementação do F6 (9.8) | F6 | **Resolvida:** aprovadas com a etapa (12/11) | F6 | Não (resolvida) |
| 32 | Rastreabilidade de `tela-builder.js` e das demais baterias novas, não rastreadas pelo Git | Consolidação do F6 (9.11) | A versão do F5 de `tela-builder.js` existe só no material do F5 (`f5.diff`, blob `5c54b06`), em scratchpad temporário | Commit, quando autorizado | Não |
| 33 | O relatório do F6 dizia "9 expectativas" e que a versão anterior não fora guardada | Consolidação do F6 | **Corrigido aqui:** 8 verificações (9 substituições) e a versão anterior recuperável (seção 9.11) | — | Não (resolvida) |
| 34 | As decisões de implementação do F7 (10.8) | F7 | **Resolvida:** aprovadas com a etapa (12/11) | F7 | Não (resolvida) |
| 35 | Edições não salvas: confirmar ao trocar de projeto (T25) e aviso do navegador ao recarregar (T17) | F7 (decisão 2 de 10.1) | **Resolvida no F9** (seções 12.3 e 12.4) | F9 | Não (resolvida) |
| 36 | 9.3.4: a pergunta mostra os critérios e a política usados, separados do padrão atual | Especificação | **Resolvida no F8** (seções 11.5 e 11.9) | F8 | Não (resolvida) |
| 37 | Rótulo "Preparar" nas rotas 3 e 4 da 3.11 e a coluna "Revisão na v1" | F7 (10.8, decisão 1) | **Resolvida no F10** (seção 13.2): "Preparar" e a linha "Preparar abre a Revisão (parcial)…" no caminho previsto | F10 | Não (resolvida) |
| 38 | Língua explícita do léxico (D8) e campo de critérios padrão em Configurações (2.10) | Especificação; F7 (decisão 3 de 10.1) | Não feitos | F11 | Não |
| 39 | Critérios valendo só para uma pergunta (D10) e gravação do perfil sem corrida (D11) | Especificação | Limitações aceitas: bloqueio da 3.12; releitura antes de gravar, com a janela da 3.13.5 | Backend (B2) | Sim (Aberta 1 da especificação) |
| 40 | 9.3.1 com critérios editados conferido no perfil simulado da `tela-perfil` e nos corpos dos `PUT`, não nos bytes do `project.json` real; `librarian.json` não exercitado | F7 (10.10) | Sem verificação em bytes nesse caso | F13 ou quando autorizado | Não |
| 41 | O nome da verificação "1 5.5 passo 8" da `tela-rascunho` ainda diz "modo da página" (a expectativa já confere "a política do projeto") | Consolidação do F7 | Só o texto do nome; não alterado nesta consolidação (sem mudança de teste) | Próxima mudança autorizada nas baterias | Não |
| 42 | O README do Studio não tem cópia do fim do F6: a mudança do F7 nele só aparece no diff acumulado contra o HEAD | Consolidação do F7 | Registrado no material do F7 (10.11). Desde o F8 há cópia dos READMEs antes de cada consolidação (11.11) | — | Não |
| 43 | Estado "Aguardando resposta colada" (4.7: "Envio manual sem texto") | F8 | **Não implementado.** Esperado: o estado derivado do registro de um envio manual ainda sem texto. Feito: a caixa de colar e "Resposta colada não conferida" (memória da página). Motivo: com "Nenhum", a página não chama `/api/send`, e o servidor recusa `manual` nessa rota (`main.rs`, `implemented`); nada é gravado até `/api/paste`, então não há registro de onde derivar o estado. Resolver exige gravar o envio manual (backend ou contrato novo), dependência que não está entre D1 e D11 | **Pendência de classificação:** a especificação não atribui etapa nem dependência | Sim (classificação) |
| 44 | Copiar diff no Windows: a página entrega o texto com LF, igual a `check.diffs[claim]`; lido de volta da área de transferência neste Windows, vem com CRLF | F8 (11.6) | 9.9.1 parcialmente atendido. Conversão fora da página (navegador ou sistema; hipótese). Impacto em `git apply` não testado | **Pendência de classificação** (Q12 atribui Copiar diff ao F8 e ao F12; a validação em Windows e Ubuntu é do F13, pendência 9) | Sim (classificação e se há correção) |
| 45 | Desempenho da conversa em runs grandes: ~260 ms por redesenho com 668 perguntas (F7: ~170 ms), medido em uma amostra | F8 (11.12) | Registrado; nenhuma otimização além do detalhe das execuções montado ao abrir | **Pendência de classificação** | Sim (se e quando tratar) |
| 46 | As decisões de implementação do F8 (11.8) | F8 | **Resolvida:** aprovadas com a etapa (12/11, início da sessão do F9) | F8 | Não (resolvida) |
| 47 | Colada que falha depois da troca de run mostra o aviso de envio ("o envio da pergunta #N … falhou") | Consolidação do F8 (auditoria) | Só o texto do aviso; não alterado | **Pendência de classificação** (texto da conversa; próxima mudança autorizada na página) | Não |
| 48 | A verificação "6a nada é enviado dali" da `tela-conversa` depende de o seletor ter Claude CLI (nesta máquina tem); a contraprova contra o F7 só aborta na preparação | Consolidação do F8 (auditoria) | Registrado; bateria não alterada | Próxima mudança autorizada nas baterias | Não |
| 49 | Diferença do fluxo `pedir_esta_mudanca_cartao` contra o F7: só o texto fotografado de `#llmNote` | F8 (11.11) | Hipótese de tempo (11.12), não verificada. Na captura do F9 o fluxo saiu igual ao do F8, o que é compatível com a hipótese, sem prová-la | — | Não |
| 50 | 9.11.1: seleções de formato 1 e 2 e o fixture `tests/evidence-v1` não exercitados pela `tela-conversa` | F8 (11.9) | Cobertos em parte pela `tela-evidencia` (pacote); a conversa com eles não foi conferida | F13 | Não |
| 51 | M1: com `/api/graph` falhando, cinco itens do resumo (e a conversa) ficam em "Carregando…" indefinidamente | Auditoria do F9 (12.10) | **Resolvida no fechamento do F9** (12.13.2), com bateria (`tela-projeto` 9a) | F9 | Não (resolvida) |
| 52 | M2: Juntar a declaração não acha funções Python decoradas (o grafo começa no decorador; o fato guarda a linha do `def`) e diz que a declaração "não está no grafo" | Auditoria do F9 (12.10) | **Resolvida no fechamento do F9** (12.13.3), com bateria (9c). O lado do servidor é a pendência 65 | F9 | Não (resolvida) |
| 53 | O resumo não carrega cada item de forma independente (2.1, "cada item carrega à parte") | Auditoria do F9 (12.10, B1) | **Resolvida no fechamento do F9** (12.13.4, 9b) | F9 | Não (resolvida) |
| 54 | "Aprovação de outra versão" (2.5) não é mostrada; a API não lista aprovações; as execuções provam só parte | Auditoria do F9 (12.10, investigação 1) | **Parcial, por decisão** (12.13.6): nada deduzido das execuções; texto "não aprovado nesta versão" mantido | Backend (histórico de aprovações), sem etapa | Não (decidida) |
| 55 | Linha de base em vigor com o critério da comparação (mesma versão), não o do portão da escada (qualquer versão), como diz a 2.5 | F9 (12.8, decisão 10) | **Resolvida no fechamento do F9**: critério da escada (2.5), com teste de versão trocada (9h) | F9 | Não (resolvida) |
| 56 | Itens do resumo sem destino: "Run" (sem alterações), "Contagens manuais: Nenhuma"; "Perguntas" sem pendentes leva à coluna Pergunta | F9 (12.8, decisão 8) | **Resolvida no fechamento do F9**: Run → Régua sempre; Contagens manuais → Inventário em todos os estados (decisão na sessão); "Perguntas" sem pendentes continua indo à coluna Pergunta | F9 | Não (resolvida) |
| 57 | Decisões de implementação do F9 (12.8), em especial 4 a 8 e 10 | F9 | **Resolvida:** decididas em parte no fechamento e aprovadas com o F9 no início da sessão do F10 (13.1) | F9 | Não (resolvida) |
| 58 | Gerar grava a run nova como escolha (`keepRun`), sem teste | Auditoria do F9 (12.10, B4) | **Resolvida no fechamento do F9** (9k, com resposta simulada de `/api/generate`) | F9 | Não (resolvida) |
| 59 | Edição feita durante o pedido de abertura, depois da confirmação do T25, se perde sem pergunta | Auditoria do F9 (12.10, B2) | **Resolvida no fechamento do F9**: campos do perfil desabilitados durante a abertura (9e) | F9 | Não (resolvida) |
| 60 | Texto do Acervo vazio diferente do da 2.6 ("Nenhum fato aprovado neste projeto"); o mesmo para a nota da run sem runs (2.1, "Sem run: gere a primeira") | Auditoria do F9 (12.9, 12.10 B3) | **Resolvida no fechamento do F9** (Acervo vazio e nota da run sem runs) | F9 | Não (resolvida) |
| 61 | Python com medição gravada (9.10.3) não é possível: o servidor só instrumenta fontes Rust | Auditoria do F9 (12.10, investigação 4) | **Aceita como limitação da versão atual** no fechamento do F9 (12.13.6) | — | Não (decidida) |
| 62 | Juntar a declaração durante investigação, entrada que recomeça ou conflito entre abas: conferido só por sonda; declaração ambígua sem caso real (0 em 30 grafos) | Auditoria do F9 (12.10, investigações 5 e 6) | **Resolvida no fechamento do F9**: investigação e conflito com bateria (9d); ambiguidade com caso sintético declarado (9c), porque não há caso real | F9 | Não (resolvida) |
| 63 | A contraprova da `tela-projeto` contra o F8 só aborta no caso 1; a verificação 1 da `tela-centro` passou a aceitar dois resultados | Auditoria do F9 (12.10) | **Resolvida em parte no fechamento**: a verificação 1 da `tela-centro` voltou a ser exata (duas verificações). A contraprova da `tela-projeto` contra a página anterior está em 12.13.7 | F9 | Não |
| 64 | 9.10.1 completo: escada com "provedor de teste" (não existe no Studio) e Comparação (F12) | Auditoria do F9 (12.10, investigação 3) | **Parcial, por decisão** (12.13.6): sem provedor de teste na interface; a escada continua abortada ou simulada nas baterias | F12, F13 | Não (decidida) |
| 65 | O servidor compara a linha do fato com o início do trecho (`store.rs`, `target_of`): fatos ligados na sugestão e `reviewed_about` não acham funções Python decoradas | Fechamento do F9 (12.13.3) | Achado, não corrigido (backend fora do escopo) | Backend, sem etapa | Sim (se e quando corrigir) |
| 66 | Juntar a declaração de um fato desatualizado é recusado com o motivo do servidor (antes dizia "não está no grafo", ou juntava se a linha coincidisse com o início) | Fechamento do F9 (12.13.3) | **Resolvida:** aprovada com o F9 (13.1) | F9 | Não (resolvida) |
| 67 | O texto do compositor para critérios do rascunho difere do citado na 3.12 ("não salvos como padrão; ficam no rascunho") | Fechamento do F9 (12.13.5) | **Resolvida:** aceito com o F9 (13.1) | — | Não (resolvida) |
| 68 | Na troca de run, a cidade da run anterior sai da cena antes de o grafo novo chegar | Fechamento do F9 (12.13.5) | **Resolvida:** aprovada com o F9 (13.1) | F9 | Não (resolvida) |
| 69 | A pasta `C:\Users\beatl\Videos\librarian-ferramentas` (com o `playwright-core` das baterias) foi apagada durante a primeira execução completa desta sessão, fora dela. As baterias passaram a usar o `playwright-core` 1.48.2 instalado do cache local do npm no scratchpad da sessão | Fechamento do F9 | Registrado; a versão anterior não é conhecida. O F10 usou o mesmo `playwright-core` 1.48.2, do scratchpad do fechamento do F9 (sessão `6e1931b3`) | — | Sim (onde manter o Playwright) |
| 70 | As decisões de implementação do F10 (13.7) | F10 | Registradas a pedido do usuário no fechamento documental; **sem aprovação específica** | F10 | **Sim** (aprovação) |
| 71 | 9.7.4 testado só com falha de provedor simulada (502 da bateria), não com um pacote adulterado numa cópia | F10 (13.8) | [Limitação] de teste | F13 ou quando autorizado | Não |
| 72 | 9.7.1 e 9.7.2 conferidos nos corpos dos pedidos de criação, não nos `evidence.json` gravados pelo servidor (as perguntas do lote são respondidas pela bateria) | F10 (13.8) | Parcial | F13 ou quando autorizado | Não |
| 73 | A contraprova da `tela-revisao` contra a página do fim do F9 é grosseira (4 PASSA, 12 FALHA; cada bloco aborta ao esperar a Revisão) | F10 (13.9) | Registrada | — | Não |
| 74 | O ponteiro da Revisão guardada (`{view: 'review', turn}`, sem a run) pode reabrir a pergunta de mesmo número em outra run se outra aba mudar a run guardada do projeto | F10 (13.7, decisão 4) | [Limitação]; resolver exige guardar a run com o ponteiro (mudança no formato da 5.3 e no módulo do F1) | — | Sim (se e quando tratar) |
| 75 | 4.1 "Só você vê": comportamento e ambiguidades não aparecem como itens próprios; 9.6.4 conferido por busca na Revisão e nas chaves do F10, não na página inteira; 9.12 em parte (diálogos e "Aprovar todos" pelo teclado não conferidos à parte) | F10 (13.8) | Parcial | F13 | Não |
| 76 | O estado consolidado e o README do Studio não descreviam o F10 | Fim do F10 | **Resolvida** neste fechamento documental (seções 2, 13 a 19; README do Studio) | — | Não (resolvida) |

As questões **Aberta 1, 2 e 3** da especificação (seção 11) continuam abertas e não foram tocadas.

---

## 17. Contratos e invariantes protegidos

[Verificado no F4 e de novo no F5, no F6, no F7, no F8 e no F9, pelas baterias] continuam funcionando:

- modo arquivo (`file://`, `loadFiles`, `body:not(.studio)`);
- `#overlayCard` (`attachSearch`);
- `/api/ask` (`askModel`);
- Léxico (ferramenta);
- `askPath` (`/api/path-questions`), com o mesmo corpo;
- colar resposta (`pasteView`, `/api/paste`, só com LLM "Nenhum" e `answer_format === 'claims'`);
- cartão de desempenho (`/api/performance-question`, só leitura);
- `LADDER_MODELS` (Claude CLI, `performance-claude`, Haiku) e `FACT_MODEL` (Ollama, `verificavel-qwen3coder`);
- `retries: 0` e `lacuna_rounds: 0` na escada;
- preferências `librarian.llm.<tipo>`;
- `window.viewerDebug`, com a página aberta em `#debug`.

**Corpos de requisição:** [Verificado] os **21 corpos** do F0 continuam **iguais byte a byte** depois do F4, do F5, do F6, do F7, do F8 e do F9 (comparação com `captura/corpos/` do arquivo do F0). No F6, a `tela-rascunho` confere também que o corpo de `/api/gathered-question` depois de recarregar é byte a byte o de antes. No F5, a captura clica "Perguntar" depois do cartão "Pedir esta mudança", que agora só monta o rascunho (seção 8.9).

**Fluxos, modos e sondas:** [Verificado] os **18 casos de fluxo** que a captura registra (14 que gravam o perfil e 4 que não gravam), os casos de modo **M1–M7**, as **16 sondas de escrita** e o laço de Separar deram **o mesmo resultado do F0** em F1, F2, F3, nas correções e no F4. Isso inclui os defeitos de gravação do perfil, que ficam para o F7. No F5, o resultado foi o mesmo, **menos o fluxo `pedir_esta_mudanca_cartao`**, que passou de 1 para 2 gravações implícitas (pendência 26); M1–M7, as sondas e o laço de Separar ficaram iguais. No F6, todos ficaram iguais aos do F5 (a mesma diferença no cartão). **No F7, os fluxos e os casos de modo mudaram de propósito** (seção 10.11): só Salvar configurações, Salvar léxico e Aplicar léxico gravam (explícitos, com a lista), e a cadeia e o cartão gravam só `allow_hypothesis` e `allow_patch` depois do T26; nenhuma edição não salva é gravada por baixo; a tela mostra a política de cada projeto. As sondas e o laço de Separar continuam iguais ao F0. A captura do F7 foi adaptada às confirmações novas. **No F8**, contra a captura do F7: modos, sondas, laço de Separar e restauração iguais; um fluxo (`pedir_esta_mudanca_cartao`) difere só no texto fotografado de `#llmNote` (pendência 49). **No F9**, contra a captura do F8: fluxos, modos, sondas, laço de Separar e restauração das raízes **iguais** (seção 12.11).

**Navegador:** o F7 deixa de ler e de gravar `librarian.mode` (a chave não é apagada) e acrescenta ao rascunho só os critérios do rascunho (`composer.criteria`), nunca a política. O F6 grava o rascunho em `librarian.draft.v1:<key>:<run>` (chave já prevista no F1), nunca com a política do modo; um rascunho vazio não deixa chave, e abrir, trocar de run e recarregar sem rascunho continuam sem gravar nada (`tela-estado` B1).

**Servidor:** nenhuma rota, corpo ou código Rust foi alterado em F0–F9 nem no fechamento do F9 (conferido: `git diff` sem arquivos `.rs` nem `.toml`; o diff da página não acrescenta nenhuma chamada de escrita). Desde o F8, os pedidos sobre uma pergunta levam a run dela, que é a mesma do momento do clique; os corpos têm as mesmas chaves e valores. **Fechamento do F9:** as leituras da run (grafo, perguntas, fatos, comandos) passaram a ser pedidas juntas, não uma depois da outra; nenhum pedido novo, nenhum corpo mudou.

**Navegação (F8):** `active_turn` de `librarian.nav.v1:<key>` passa a ser gravado, só por escolha da pessoa (seção 11.4); abrir, trocar de run e recarregar continuam sem gravar.

**Navegação (F9):** `run` de `librarian.nav.v1:<key>` passa a ser gravado, só por escolha da pessoa: o seletor de run, Gerar e "Ir à origem" em outra run (seção 12.2). **Trocar de run pelo seletor grava agora só a `run`**; abrir e recarregar continuam sem gravar. Os links da Régua, do Acervo e do resumo gravam `active_turn` (escolha da pessoa).

**Fechamento do F9:** reabrir o mesmo projeto não grava nada nem troca a run (a não ser que ela tenha saído do registro); Cancelar o T25 não grava nada. Os 21 corpos continuam iguais ao F0, e fluxos, modos, sondas, laço de Separar e restauração, iguais aos do fim do F9 (12.13.7).

**Perfil:** desde o F7, `PUT /api/project` só é chamado por Salvar e por Alterar a política (a `tela-perfil` confere os dois pontos de chamada na página). No F9, "Salvar e abrir" e "Salvar e limpar" usam o mesmo Salvar. O F10 não acrescenta nenhuma gravação do perfil.

**F10:** os 21 corpos continuam iguais ao F0 (os de `/api/gathered-question` e `/api/ladder`, que o F10 não muda; o corpo de `POST /api/send` tem as chaves de sempre, conferidas pela `tela-revisao`). Isso não quer dizer que nada mudou: contra a captura do fechamento do F9, modos, sondas e restauração das raízes são iguais; os fluxos de Separar e de Conferir fatos diferem no texto da nota (as perguntas ficam preparadas, não são enviadas), e o laço de Separar com uma falha no meio pede as 3 criações (no F9 parava na segunda). **Preparar pelo Builder ou por um achado não envia mais**: o envio é o clique em Enviar, na Revisão. Nenhuma rota nova, nenhum `.rs` nem `.toml` alterado. Navegação: `center` passa a guardar `{view: 'review', turn}` quando a pessoa abre a Revisão de uma pergunta, e a troca de run no mesmo projeto, com essa vista guardada, grava `{view: 'city'}` (13.7, decisão 4).

---

## 18. Histórico de testes

### 18.1 Verificações de tela (`node crates/librarian-studio/tests/tela/rodar.js`)

Os totais são **acumulados**: cada marco roda todas as baterias existentes naquele momento. **Não some os números entre marcos.**

| Marco | Total | O que mudou na contagem | Página |
|---|---:|---|---|
| F0 | **310** | As 10 baterias anteriores à reformulação (Etapas 2 a 4 do Evidence Builder) | `adc84d54` |
| F1 | **368** | + `tela-estado.js` (58: 47 isoladas + 11 com servidor) | `340a5c47` |
| F2 inicial | **456** | + `tela-navegacao.js` (88); `tela-estado` B1 alterado | `9cde9b73` |
| Correções do F2 | **472** | `tela-navegacao` 88 → 103; `tela-estado` 58 → 59 (B1 reescrito: abrir não grava navegação) | `312a970e` |
| F3 inicial | **569** | + `tela-inspecao.js` (98); `tela-navegacao` 103 → 102 (três expectativas do comportamento provisório substituídas) | `a03a6252` |
| Correção do Voltar entre projetos | **586** | `tela-navegacao` 102 → 119 (cenários A a D, com e sem armazenamento) | `5989913a` |
| Correção do isolamento do grafo | **606** | `tela-navegacao` 119 → 139 (cenários de grafo A a D) | `8a7592f2` |
| F4 | **699** | + `tela-centro.js` (93) | `a0d5140f` |
| F5 | **802** | + `tela-builder.js` (102); `tela-4b` 55 → 56; `tela-centro` item 1 trocado (mesma contagem) | `0f47bab4` |
| F6 | **875** | + `tela-rascunho.js` (73); `tela-builder` com 8 expectativas atualizadas (mesma contagem) | `a5c0023b` |
| F7 | **1025** | + `tela-perfil.js` (150); `etapa2c`, `tela-builder`, `tela-rascunho` e `tela-passo6` adaptados (mesmas contagens; seção 10.11) | `fa167e09` |
| F8 | **1127** | + `tela-conversa.js` (102); `tela-perfil` com +2 linhas de espera (mesma contagem; seção 11.10) | `dbc72629` |
| F9 | **1265** | + `tela-projeto.js` (138); `tela-estado`, `tela-centro`, `tela-rascunho` e `tela-navegacao` ajustados ao F9 (mesmas contagens; seção 12.11). Uma execução anterior deu 1262/3, nas expectativas ajustadas depois | `b6fcff72` |
| Fechamento do F9 | **1347** | `tela-projeto` 138 → 219 (seção 9); `tela-centro` 93 → 94 (verificação 1 em duas); `tela-builder` e `tela-perfil` com as mesmas contagens (esperas e textos, 12.13.7) | `693f69dd` |
| F10 | **1505** | + `tela-revisao.js` (158); `tela-perfil` (2 textos) e `tela-centro` (verificação 1) ajustadas ao F10, com as mesmas contagens (seção 13.9) | `b89f8d41` |

**Contagem por bateria na última execução** (F10, 13/11, página `b89f8d41`, que é a página atual): cartão 37, zero 29, origem 32, nomes 12, passo2 9, 4a 30, 4b 56, passo6 36, etapa2c 45, evidência 25, estado 59, navegação 139, inspeção 98, centro 94, builder 102, rascunho 73, perfil 150, conversa 102, projeto 219, revisão 158.

**Divergência registrada (consolidação do F4):** o prompt daquela consolidação citou `tela-navegacao.js` com 119 verificações. Esse era o número depois da correção do Voltar; a correção do grafo o levou a **139**, que é o valor da última execução e o que já estava no README.

### 18.2 Outros testes (iguais em todos os marcos)

| Verificação | Resultado |
|---|---|
| `cargo test --locked -p librarian-studio` | 185 aprovados (reproduzido no F10, 13/11) |
| `cargo test --locked --workspace` | 432 aprovados, 2 ignorados (reproduzido no F10, 13/11) |
| `cargo build --release --locked -p librarian-studio` | sucesso (no F10, `cargo build --release --locked` do workspace) |
| Corpos 9.5 | 21/21 iguais ao F0 (no fechamento do F9, com a espera da captura aumentada, 12.13.7; de novo no F10, 13.9) |
| `git diff --check` | sem problemas (no F10 e neste fechamento documental) |
| `send.json` / `question.json` nas raízes de teste | 844 → 844 / 1420 → 1428 (as 8 perguntas criadas por `etapa2c`, como documentado) |
| `project.json` das raízes | iguais em bytes antes e depois (no F7, a `tela-perfil` grava uma vez e restaura, e o `etapa2c` muda a política e a devolve pelo seletor; a `tela-conversa` e a `tela-projeto` não gravam) |

O hash do binário de release muda entre marcos, embora o código Rust seja o mesmo; a hipótese é o `build.rs`, que grava o estado do Git no binário.

### 18.3 Como as baterias rodam

- **Nenhuma chamada real a LLM:** as baterias abortam `/api/send`, as execuções e `/api/generate`, e simulam ou abortam `/api/ladder`. As novas (estado, navegação, inspeção, centro, builder, rascunho, perfil) abortam toda escrita e contam as tentativas; as do Builder e do rascunho respondem elas mesmas `PUT /api/project` (desde o F7, devolvendo o corpo enviado) e `POST /api/gathered-question`, sem servidor; a do perfil responde o perfil inteiro com a validação do servidor, e só um caso grava de verdade, restaurando depois.
- **Sempre em cópias das raízes de teste**, nunca na raiz do repositório (o `rodar.js` recusa). Cada execução completa recopia as raízes intactas da seção 3.6.
- **Projetos fictícios:** B e C das baterias de navegação, centro, builder, rascunho e perfil, o projeto fictício da `tela-conversa` e o B da `tela-projeto` são simulados (seção 6.6 b).
- Variáveis e requisitos: [README das baterias](../crates/librarian-studio/tests/tela/README.md).

### 18.4 Onde está o material de cada etapa

**[Limitação]** Fora do arquivo do F0, todo o material fica em scratchpads temporários, que podem desaparecer. Os relatórios completos das etapas estão nas transcrições das sessões (`C:\Users\beatl\.claude\projects\c--Users-beatl-Documents-Projetos-librarian\<sessão>.jsonl`).

| Etapa | Sessão | Material (`C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\<sessão>\scratchpad\…`) |
|---|---|---|
| F0 | `a0aa6f80-26d4-49aa-b2a9-25e2b99e2bcb` | `f0\` (cópia estável em `.studio/arquivo-copias/linha-base-f0-20261111/`) |
| F1 | `db624789-6a2c-4a0a-afbb-b8961877f73a` | `f1\` (`f1.diff`, logs) |
| F2 | `a72dc6fc-7c4e-460c-880b-a15b9d348d21` | `f2\` (`f2-somente.diff`, `f2-completo.diff`, `log\final.log`) |
| Correções do F2 | `db624789-…` | `f2fix\` (`correcao-index.diff`, `f2-completo-atualizado.diff`) |
| F3 e correções | `af36dd42-edd6-47bb-b9d8-1b759b082d8b` | `f3\` (`f3-so.diff`, `f1-f2-f3-completo.diff`), `fix\` e `fix2\` (`correcao.diff` e o acumulado de cada uma) |
| F4 | `464af051-9ee4-49bf-9b9b-cba66ea7a254` | `f4\` (`f4-completo.diff`, `final.sh`, `log\final.log`) |
| F5 | `efa1aaa8-d3a1-44eb-abdb-d99e44f8212b` | `f5\` (`f5.diff`, que contém `tela-builder.js` inteiro na versão do F5; `base-f4\`, `final.sh`, `captura.js` adaptada, `log\final.log`) |
| F6 e consolidação | `6179a9b1-b16f-404f-9d6a-632b4398d997` | `f6\` (`f6-index-e-docs.diff`, `base-f5\index.html`, `tela-builder-f5-do-diff.js`, `tela-builder-f5-para-f6.diff`, `final.sh`, `uma.sh`, `log\final.log`, `log\rodar.log`) |
| F7 e consolidação | `652035f0-ab60-4a0a-82f3-d832e119ea0b` | `f7\` (`f7-index.diff`, `f7-baterias.diff`, `f7-tela-perfil.diff`, `f7-rastreados.diff`, `base-f6\` com a página e as baterias do fim do F6, `final.sh`, `uma.sh`, `contra-f6.sh`, `captura.js` adaptada, `log\final.log`, `log\rodar.log`, `log\contra-f6.log`, logs do cargo) |
| F8 e consolidação | `1b1a4762-b094-447e-ab02-84372fb6c170` | `f8\` (`f8-index.diff`, `f8-baterias.diff`, `tela-conversa.js`, `base-f7\` com a página, as baterias e os documentos do fim do F7, `final.sh`, `uma.sh`, `contra-f7.sh`, `captura.js`, `mede.js`, `mede2.js`, `log\final.log`, `log\final-1.log`, `log\final-2.log`, `log\rodar.log`, `log\contra-f7.log`, `log\fluxos-contra-f7.log`, `log\cargo.log`, `consolidacao\` com os documentos antes desta consolidação) |
| F9 e auditoria | `8f80f82a-52dd-4d24-b9bf-1981f62091b4` | `f9\` (`f9-index.diff`, `f9-baterias.diff`, `base-f8\` com a página, as baterias e os documentos do fim do F8, `final.sh`, `so.sh`, `sobe.sh`, `desce.sh`, `so-base.sh`, `captura.js`, `compara.js`, `compara-fluxos.js`, `log\final.log`, `log\final-1.log`, `log\rodar.log`, `log\rodar-1.log`, `log\captura.log`, `log\contra-f8.log`, `log\fluxos-contra-f8.log`, `log\cargo.log`, `auditoria\` com a sonda, o log dela e o cargo da auditoria, `consolidacao\` com os documentos antes daquela consolidação) |
| Fechamento do F9 | `6e1931b3-8608-4155-8c9d-3699327ae1e0` | `f9fix` (12.13.8) |
| F10 e fechamento documental | `7c8c75ba-7413-4e03-b80e-5fe43ddf38c1` | `f10\` (13.11) |

---

## 19. Próxima etapa: F11

- **F0 a F10 estão concluídos.** F0 a F9 foram aprovados; o F10 (seção 13) foi implementado, verificado e registrado como concluído no fechamento documental de 13/11, com as decisões de 13.7 sem aprovação específica (pendência 70).
- **F11 — Ferramentas** não foi iniciado. Segundo o usuário, é a **próxima etapa autorizada**, a começar em sessão própria, com prompt próprio. A definição oficial está na seção 10 da [especificação](ESPECIFICACAO-FUNCIONAL-STUDIO.md#10-plano-de-implementação-f0-a-f13): Léxico com língua explícita (D8) e Aplicar pelo Salvar; Configurações com a lista do que muda; Exportar sem Salvar implícito; Modelfiles; Abrir arquivos. Este documento não define escopo nem decisões do F11.
- Pendências que a especificação ou este documento ligam ao F11: 7 (`librarian.modelfile`) e 38 (D8 e os critérios padrão em Configurações).
- Continuam com as outras etapas: **F12** ("Comparar", Copiar diff na Comparação e a Comparação do 9.10.1), **F13** (pendências 9, 20, 24, 40, 50, 71, 72 e 75, e 58, 62 e 64 na aceitação) e o backend (pendência 39; a 54, se for por backend; a 65).
- **À espera de decisão do usuário:** as decisões do F10 (70); o achado do servidor (65); o ponteiro da Revisão guardada (74); onde manter o Playwright das baterias (69).
- **Pendências de classificação** (sem etapa na especificação): 43, 44, 45 e 47 (do F8).
- **Commit:** nada de F0 a F10 foi commitado. A consolidação do commit depende de autorização do usuário.
