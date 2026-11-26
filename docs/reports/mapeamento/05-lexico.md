# Mapeamento do repositório — parte 5: o léxico e a busca

**Estado:** validado pelo usuário em 26/11/2026 (Paradas 1 e 2). MAP-Q16 a MAP-Q19 respondidas.

**Data:** 26/11/2026. **Base:** `main` em `10b0f2e` (worktree limpa no início). **Só leitura**: nenhum código, documento, `.studio`, Healthchecks ou ray tracer foi alterado. Nenhum LLM foi chamado (nem Ollama, nem Claude). A execução foi feita só sobre uma **cópia** da `.studio`, no scratchpad da sessão, e só nas rotas `/api/lexicon/suggest` (com o perfil `manual`, que não chama ninguém) e `/api/lexicon/check`. No fim, o original foi conferido e a cópia, apagada (seção "Execução").

**Escopo.** Do léxico: de onde vêm as palavras do pedido, o que vai ao LLM e o que volta, o dicionário e a conferência. Ids novos continuam a numeração da [parte 4](04-gabarito-2.4e.md): `MAP-O43` em diante e `MAP-Q16` em diante.

**Fontes.** `.rs`, `.js`, `crates/librarian-studio/web/index.html` (só o que a página manda e mostra), `modelfiles/catalog.json`, `modelfiles/system/lexico.*.txt` e os JSON da cópia da `.studio`. Nenhum `.md` foi aberto além dos relatórios 01, 02 e das seções A e C do 03; do 04 e do ESTADO-ATUAL, só o cabeçalho e as regras de trabalho. Comentários no código e textos da página contam como **"diz ser"**.

**Regras de evidência:** as mesmas das partes 1 a 4. Cada afirmação traz `ARQUIVO:linha` ou o comando. **Verificado** quer dizer por execução ou pela leitura do código que faz a coisa. **Leitura minha** marca interpretação.

Caminhos abreviados: `lexicon.rs`, `store.rs`, `main.rs`, `llm.rs` são de `crates/librarian-studio/src/`; `search.rs` é `crates/librarian-ingest/src/search.rs`; `index.html` é `crates/librarian-studio/web/index.html`; `models.rs` e `transport.rs` são de `crates/librarian-llm-engine/src/`.

**Casos reais usados** (os dois projetos da `.studio`, MAP-O1):
- ray tracer: `renderer-9b78f3a9`, run `20261118-020654`, 819 símbolos (214 marcados como teste); léxico atual com 58 grupos e 60 stop words, importado do `librarian.json` do projeto (`imported_from` no `project.json`);
- Healthchecks: `healthchecks-25597e41`, run `20261118-061046`, 4.718 símbolos; léxico atual **vazio** (0 grupos, 0 stop words); 14 perguntas gravadas, todas do tipo "reunida" (`chain-step-v1`, `status: "gathered"`), sem busca pelo léxico.

---

## Parada 1: o léxico

### A. De onde vêm as palavras

**Quem escolhe:** `lexicon::request` (`lexicon.rs:19-68`), chamado por `Store::suggest_lexicon` (`store.rs:1182`). As palavras são de **três fontes**, todas mecânicas, sem LLM (verificado, por leitura):

| Fonte | Função | O que entra |
|---|---|---|
| Termos do código (`code_terms`) | `search::vocabulary` (`search.rs:55-68`), cortado em `TERMS = 80` (`lexicon.rs:14`, `:20-24`) | Os 80 tokens mais frequentes **dos nomes declarados**, cada um com quantos símbolos o usam |
| O léxico atual (`current_lexicon`) | `settings.lexicon` (`store.rs:1182`) | Os grupos e as stop words já salvos, inteiros |
| Termos sem resultado (`missed_terms`) | `Store::missed_terms` (`store.rs:393-418`) | Palavras de perguntas anteriores que não tocaram nenhum hit, com quantas perguntas as usaram |

**Como `vocabulary` conta** (verificado, `search.rs:55-68`):
- **De onde:** só `symbol.name` de **todos** os símbolos da edition (`:57-58`). Não entram o nome qualificado, o caminho do arquivo, o corpo do código nem os comentários.
- **Tokens:** `search::tokens` (`:23-51`): divide em maiúscula depois de minúscula ou número, e em sigla seguida de minúscula (`:27-35`); passa para minúscula; tira o acento do português (`:37-44`) e as marcas combinantes (`:45-47`); todo caractere que não é letra nem número vira separador (`:48`). Ex.: `the_pixel_size_for_a_horizontal_canvas` → `the`, `pixel`, `size`, `for`, `a`, `horizontal`, `canvas`.
- **Frequência:** conta **símbolos**, não ocorrências: cada token conta uma vez por símbolo (`BTreeSet`, `:58`).
- **Filtro:** só tokens com 3 ou mais caracteres e que não sejam só números (`:60`). **Nada mais é filtrado:** nem os símbolos de teste, nem os blocos de navegação (`impl`, `module` no Rust; `class` no Python, `rust.rs:179-181`, `python.rs:109-111`), que a busca exclui (`search.rs:150-157`), nem as stop words embutidas (`BUILTIN_STOP_WORDS`, `search.rs:19-20`).
- **Ordem:** mais frequente primeiro; empate em ordem alfabética (`:66`). O corte em 80 cai no meio de um empate quando ele existe.

**Termos sem resultado** (verificado, `store.rs:393-418`): lê o `search.json` de todas as perguntas de todos os runs do projeto (`:401-405`). Um termo entra quando está em `terms` e não aparece no `reasons[].term` de nenhum hit daquela pergunta (`:406-413`). Ficam de fora as stop words embutidas e as palavras que o léxico já tem, em grupo ou como stop word (`:396-397`). Ordem: mais perguntas primeiro, depois alfabética (`:416`).

**Caso real** (execução na cópia, seção "Execução"; o pedido gravado em `lexicon/<id>/query.json`):

| | ray tracer | Healthchecks |
|---|---|---|
| Tamanho do pedido | 14.253 bytes | 7.130 bytes |
| `code_terms` | 80 (de 385 tokens distintos) | 80 (de 1.179) |
| Os 15 primeiros (`termo=símbolos`) | `impl=97 the=82 for=73 color=62 transform=51 default=42 material=38 ray=38 from=37 with=31 matrix=26 object=26 set=26 sphere=26 normal=24` | `test=2009 handles=415 case=219 migration=190 dependencies=187 operations=187 works=173 set=159 requires=151 shows=127 rejects=125 get=121 checks=112 access=111 add=111` |
| O corte no 80º | `between=5` entra; `caps`, `create`, `depth`, `eye` (também 5) ficam de fora | `report=27` entra; `validates=27` fica de fora |
| `current_lexicon` | 58 grupos, 60 stop words | vazio |
| `missed_terms` | 0 (não há perguntas no run) | 0 (as 14 perguntas não buscaram: `terms: []`) |

**De onde vêm os 80, no caso real.** Conta minha, com uma reimplementação de `tokens` e `vocabulary` em JS (no scratchpad, `scripts/vocab.js`). **Conferida:** os 80 termos e contagens que ela dá são iguais, um a um, aos do pedido que o studio montou, nos dois projetos.

| | ray tracer | Healthchecks |
|---|---|---|
| Termos que vêm **só** de símbolos de teste | 14 (ex.: `the`, `and`, `when`, `constructing`, `intersecting`, `shadow`) | 11 (`handles`, `works`, `requires`, `shows`, `rejects`, `does`, `missing`, `work`, `instructions`, `sets`, `non`) |
| Termos com a maioria dos símbolos de teste | 35 de 80 | 51 de 80 |
| Termos que vêm **só** de blocos que a busca exclui | 3: `impl` (97, o 1º; nomes como `impl Camera`), `tests`, `ops` | 0 |
| Stop words embutidas entre os 80 | `the`, `from`, `with` | `does`, `with` |

Ver MAP-O43 e MAP-O44.

### B. O que vai ao LLM e o que volta

**As rotas** (verificado):

| Rota | Onde | O que faz |
|---|---|---|
| `POST /api/lexicon/suggest` `{key, run, profile, claude_model?}` | `main.rs:255-263` | Lê a escolha (`choice`, `main.rs:490-499`; sem `profile`, vale `"manual"`, `:495`). Se não é `manual`, confere que o perfil existe (`:259-261`). Chama `Store::suggest_lexicon` |
| `POST /api/lexicon/check` `{key, run, id, text}` | `main.rs:264-268` | Confere uma resposta colada à mão, de um pedido `manual` anterior (`Store::check_lexicon`) |

Não há outra rota de léxico em `main.rs` (`grep -n lexicon crates/librarian-studio/src/main.rs`: só essas duas, mais o `mod lexicon`). O léxico também é gravado por `PUT /api/project` (seção C).

**O pedido montado** (`Store::suggest_lexicon`, `store.rs:1176-1196`, verificado):
1. Lê o idioma do projeto (`language_of`, `store.rs:1682-1688`: vazio ou `pt` é português, `en` é inglês, outro valor é erro), carrega a edition do run e calcula os termos sem resultado (`:1177-1180`).
2. Reserva uma pasta nova `projects/<key>/lexicon/<carimbo>` (`:1181`; `reserve`, `store.rs:1537-1549`, acrescenta `-2`, `-3`… se já existe).
3. Monta o pedido (`lexicon.rs:19-68`) e o grava em `query.json` (`store.rs:1183-1185`). A forma é a mesma das perguntas, `{question, evidence, instructions}` (diz ser: `lexicon.rs:16`):
   - `question`: uma frase pedindo grupos de sinônimos e stop words (`:28` em inglês, `:49` em português);
   - `evidence`: `code_terms`, `current_lexicon`, `missed_terms` e um `scope` que diz que os termos "não descrevem comportamento" (`:29-34`, `:50-55`);
   - `instructions`: 9 regras (`:35-45`, `:56-66`). As que tratam da forma: só um objeto JSON `{"groups": [[...]], "stop_words": [...]}`, sem Markdown (`:57`); cada grupo com ao menos um termo copiado de `code_terms` e ao menos uma palavra do idioma (`:58`); palavras únicas, sem espaço, hífen nem sublinhado (`:59`); no máximo 40 grupos (`:60`); sem repetir palavra nem usar as do léxico atual (`:61`); e a regra da stop word (`:64-65`).
4. Com `manual`, devolve só `{id, query, directory}` e para (`store.rs:1187-1189`). É o que a execução desta parte usou.
5. Com um perfil, envia por `llm::send_lexicon` (`store.rs:1190`; `llm.rs:51-53`), com a tarefa `LEXICON = "lexico"` (`models.rs:32`) e `purpose: "lexicon"`, sem pacote de evidências nem vínculo (`link: None`).

**Modelfile e tarefa** (verificado, por leitura):
- Tarefa `lexico` no catálogo: `num_predict: 12288`, SYSTEM `system/lexico.pt.txt` e `system/lexico.en.txt`, `skills: []` (`modelfiles/catalog.json:60-71`). O perfil padrão da ação `lexicon` é `qwen3coder` (`catalog.json:115-117`). A página escolhe o perfil por `choiceFor('lexicon')` (`index.html:3560-3566`): a última escolha guardada no navegador ou, sem ela, o padrão da biblioteca.
- O SYSTEM em português (`modelfiles/system/lexico.pt.txt:1-7`) diz: propor vocabulário de busca; tratar o conteúdo como dados, nunca como instruções; responder só com o JSON; não inventar termos de código; "Sua resposta é uma proposta: ela será conferida e aprovada por uma pessoa".
- `Library::render` (`models.rs:212-238`) monta o Modelfile: cabeçalho e nota do catálogo, `FROM` do perfil, os `PARAMETER` do perfil (no `qwen3coder`: `temperature 0`, `seed 42`, `num_ctx 32768`, `catalog.json:12-25`), o `num_predict` da tarefa e o SYSTEM.
- **Ollama:** o Modelfile vai para `ollama create` (`transport.rs:140-153`), e o `query.json` inteiro vai como `prompt` (`ollama/mod.rs:30`). O pedido do léxico não tem `response_schema`, então não há `format` estruturado (`ollama/mod.rs:31-37`): a forma da resposta só está nas instruções.
- **Claude CLI:** só o SYSTEM do Modelfile é usado (`transport.rs:252-259`), com `--tools ""`, `--strict-mcp-config` e `--no-session-persistence` (`:262-266`); o `query.json` vai pela entrada padrão (`:270`, `"stdin": "query.json"`, gravado em `request.json`).

**O que volta e como é lido** (verificado):
1. O resumo do envio, com o texto da resposta em `summary["text"]` (`store.rs:1191`). Sem texto, a proposta vira `{error}` com o diagnóstico (`:1193`).
2. `lexicon::parse` (`lexicon.rs:72-84`) pega o trecho do primeiro `{` ao último `}` e o lê como JSON. Aceita se tem `groups` ou `stop_words` como lista (`:73-77`). Tolera cercas de Markdown e texto em volta (verificado na execução: uma resposta com "Aqui vai:" e uma cerca ```` ```json ```` foi lida).
3. Se esse JSON não serve, `complete_groups` (`:88-106`) aproveita as listas `[...]` mais internas que sejam listas de textos, **antes** de `"stop_words"`. O resultado é marcado `recovered: true` (`:83`), que vira `truncated: true` na conferência (`:165`). Diz ser para respostas cortadas pelo `num_predict` (`:71`, `:216`). Verificado na execução: `{"groups": [["canal", "channel"], ["chave", "ke` deu 1 grupo aceito e `truncated: true`.
4. Nada aproveitável: erro "A resposta não tem um JSON com "groups" nem grupos completos para aproveitar" (`:81`). Verificado com `{"outra":1}`.
5. A conferência (`lexicon::check`, seção D) e a gravação em `proposal.json` (`store.rs:1210-1216`).

### C. O dicionário

**Formato** (verificado): `search::Lexicon` (`search.rs:9-16`), com `deny_unknown_fields` (`:10`):

```
{ "groups": [["palavra", "sinonimo", "termo_do_codigo"], ...], "stop_words": ["palavra", ...] }
```

- **`groups`**: grupos de sinônimos. Na busca, cada palavra do grupo vira um "alias" para o grupo inteiro (`search.rs:127-142`).
- **`stop_words`**: palavras tiradas de toda pergunta, **somadas** às embutidas (`BUILTIN_STOP_WORDS`, `search.rs:19-20`, 34 palavras em português e inglês, como `a o de que qual onde como the of how where`; `search.rs:143-146`).
- **Não há campo de homônimos, de termos nem de quem/quando.** Os dois campos são as únicas chaves aceitas. "Termos" aparecem só no pedido (`code_terms`), nunca no dicionário. Leitura minha: um homônimo (a mesma palavra para dois conceitos) não tem como ser escrito, porque uma palavra só pode estar num grupo. `SearchIndex::new` recusa palavra em dois grupos ("Overlapping alias group", `search.rs:138-139`), e `check` também (`lexicon.rs:137-140`).

**Onde fica** (verificado):

| Lugar | Quem grava | Quem lê |
|---|---|---|
| `.studio/projects/<key>/project.json`, campo `lexicon` do `Settings` (`store.rs:187`) | `PUT /api/project` → `save_settings` (`main.rs:124-128`; `store.rs:325-337`), que regrava o `project.json` inteiro (`:335`) | toda busca e todo pedido de léxico (`store.rs:464`, `:1177-1182`) |
| No projeto analisado: o arquivo apontado por `librarian.json` → `"lexicon"` (padrão `ai/librarian/lexicon.json`) | só **Exportar**, a pedido (`export_profile`, `store.rs:1241`, `:1263`) | só ao **abrir** o projeto pela primeira vez (`store.rs:1507-1508`); depois, as edições ficam na `.studio` (diz ser: `store.rs:199`, "later edits stay in `.studio`") |

Projeto novo sem perfil: léxico vazio (`store.rs:1479`). É o caso do Healthchecks.

**Quem edita** (verificado):
- **A pessoa**, à mão: a caixa de texto do painel Léxico (`index.html:538`) e o botão Salvar (`:8228-8230`). A página só confere se é JSON (`index.html:3239`) e mostra a lista do que muda num diálogo de confirmação (`saveProfile`, `:3300-3316`; o diálogo em `:3312`). O servidor não confere nada do léxico ao salvar (`save_settings`, `store.rs:325-337`; o comentário diz ser: "The lexicon is checked by the search itself, on the next question", `:334`). Só a desserialização recusa chave desconhecida (`search.rs:10`).
- **O LLM propõe**, nunca grava. A proposta conferida volta à página com o léxico já somado (`lexicon` = atual + aceitos, `lexicon.rs:118`, `:156`, `:202`). **Aplicar** copia esse léxico para a caixa de texto e abre o mesmo Salvar (`index.html:8079-8086`). Confirmar grava; cancelar deixa como alteração não salva (`:8084`, texto `lexicon.appliedUnsaved`, `:1888`). O texto da página diz ser: "nada entra no léxico sem você aplicar" (`index.html:919`).

**O que é gravado para auditoria** (verificado, por leitura e na execução):

| O quê | Onde | Observação |
|---|---|---|
| O pedido | `lexicon/<id>/query.json` (`store.rs:1185`) | um por pedido, numa pasta nova |
| O envio (só com perfil) | `lexicon/<id>/send/send.json` (`llm.rs:106`), com `provider`, `profile`, `task`, `language`, `created`, `elapsed_ms`, `semantic_evaluation: "pending"`, `skills`, `purpose: "lexicon"` (`llm.rs:87-101`); o Modelfile usado em `Modelfile.requested` (`transport.rs:140`, `:255`) | **quem propôs** fica aqui |
| A resposta colada (manual) | `lexicon/<id>/pasted-response.txt` (`store.rs:1206`) | **regravada** a cada conferência do mesmo `id`. Verificado: depois de duas conferências no mesmo pedido, o arquivo só tinha a segunda resposta |
| A proposta conferida | `lexicon/<id>/proposal.json` (`store.rs:1214`) | também regravada a cada conferência; o erro de gravação é ignorado (`let _ =`) |
| O léxico aplicado | `project.json` (`store.rs:335`) | só o estado novo. **Não fica gravado:** qual proposta (`id`) foi aplicada, quem confirmou, quando, nem o léxico anterior |
| O léxico usado por pergunta | `lexicon_sha256` no relatório da busca (`search.rs:98`, `:287`) | só o hash. Detalhe na Parada 2 |

Ver MAP-O47.

### D. A conferência do léxico

O código não usa a palavra "portão" para o léxico. A conferência é `lexicon::check` (`lexicon.rs:113-169`), com `check_stop_words` (`:174-207`). Ela roda nos dois caminhos: depois de um envio (`store.rs:1192`) e em `/api/lexicon/check` (`store.rs:1207`), sempre por `check_text` (`store.rs:1210-1216`).

**`/api/lexicon/check`, antes de conferir** (verificado, `store.rs:1199-1208`): o `id` passa por `safe_id` (`:1202`; `../x` é recusado, teste `store.rs:2615`), a pasta precisa ter um `query.json` (`:1203-1205`; verificado: "Pedido de léxico desconhecido: 99999999-999999"), e o texto é gravado antes de conferir (`:1206`).

**Cada grupo** (verificado, `lexicon.rs:120-160`), palavra por palavra, em ordem:
1. não é texto → ignorada, com nota (`:124-127`);
2. não dá **um** token → recusada: "não é uma palavra única" (`:128-132`);
3. repetida no grupo → ignorada sem nota (`:134-136`);
4. já está num grupo do léxico atual ou num grupo aceito antes nesta proposta → recusada: "já está em outro grupo" (`:137-140`);
5. é stop word (embutida ou do léxico atual) → recusada: "é stop word" (`:141-144`).

Depois, o grupo inteiro:
- menos de 2 palavras que restaram → `rejected`, "menos de duas palavras" (`:148-150`);
- **nenhuma** palavra está no vocabulário do código → `rejected`, "nenhuma palavra existe no código" (`:151-153`). É a conferência de "tem hit no código". O "código" aqui é o `vocabulary`: tokens de **nomes de símbolos**, com 3 ou mais caracteres (`:114`, `search.rs:55-68`);
- senão → `accepted`; as palavras entram em `taken` e o grupo é somado ao léxico (`:154-157`).

**Cada stop word** (verificado, `lexicon.rs:174-207`), em ordem:
- já é stop word → `rejected` (`:192`);
- está num grupo (atual ou aceito agora) → `rejected` (`:193`);
- é token de um **nome**, **nome qualificado** ou **caminho de arquivo** → `rejected`, "existe em nomes ou caminhos do código" (`:194`; conjuntos em `:177-184`);
- só aparece no **corpo** do código → `accepted`, com nota (`:195`);
- outra → `accepted`, sem nota (`:196`);
- mais de um token → `rejected` (`:197`).

**No fim:** o léxico resultante (atual + aceitos) precisa construir um `SearchIndex` (`lexicon.rs:162`); se não, a conferência inteira vira erro. A resposta traz `lexicon`, `report`, `accepted`, `rejected`, `truncated`, `stop_report`, `stop_accepted` e `stop_rejected` (`:166-168`). A página mostra cada grupo com a marca aceito/recusado e realça as palavras que estão no código (`index.html:8066-8070`).

**O que faz quando não há hit:** o grupo é recusado e fica no relatório com a nota; nada é somado. Não há uma segunda tentativa nem aviso ao LLM. A pessoa vê a recusa e decide (verificado, `index.html:8063-8100`).

**Caso real, com respostas escritas à mão** (sem LLM; cópia da `.studio`; o pedido `manual` de cada projeto). Resultado verificado pela rota:

| Projeto | Proposta | Resultado e nota do código |
|---|---|---|
| Healthchecks | `["verificacao","verificacoes","check","checks"]` | aceito; no código: `check`, `checks` |
| | `["canal","canais","channel","channels"]` | aceito; no código: `channel`, `channels` |
| | `["atraso","grace"]` | aceito; no código: `grace` |
| | `["channel","channels"]` (as duas do código) | **aceito**: a instrução pede uma palavra que não é do código (`lexicon.rs:58`), mas a conferência não exige isso (MAP-O48) |
| | `["identificador","id"]` | **recusado**, "nenhuma palavra existe no código", embora 54 nomes de símbolos tenham o token `id`: `id` tem 2 letras e o vocabulário só guarda 3 ou mais (MAP-O45) |
| | `["api","interface"]` | aceito; no código: `api` |
| | stop words `fica` / `ping` / `self` | `fica` aceita sem nota; `ping` e `self` recusadas, "existe em nomes ou caminhos do código" |
| ray tracer | `["raio","feixe","ray"]` | recusado: `raio` e `ray` já estão num grupo; sobrou `feixe`, "menos de duas palavras". A palavra nova não é somada ao grupo que já existe (MAP-O48) |
| | `["xyz","abc"]` | recusado, "nenhuma palavra existe no código" |
| | `["cor de fundo","color"]` | recusado: "cor de fundo" não é uma palavra única; `color` já está num grupo |
| | stop words `fica` / `unwrap` | aceitas, com a nota "aparece no corpo do código" |
| | stop word `zzz` | **aceita, sem nota**: nada confere que a palavra seja de pergunta (MAP-O49) |
| | stop words `color` / `sorteio` / `calcula` / `duas palavras` | recusadas: num grupo; em nomes do código; "já é stop word"; não é palavra única |

**O léxico atual do ray tracer não passaria inteiro pela conferência de hoje.** Na cópia, zerei o léxico do `project.json` e enviei os 58 grupos e as 60 stop words atuais como uma proposta a `/api/lexicon/check`. Depois, restaurei o `project.json` da cópia. Resultado, verificado pela rota:
- grupos: 56 aceitos e 2 recusados por "nenhuma palavra existe no código": `["forma","formas","shape","shapes"]` e `["coordenada","coordenadas","coordinate","coordinates"]`;
- stop words: 36 aceitas (3 com a nota "aparece no corpo do código": `use`, `without`, `result`). 14 recusadas por "já é stop word": são as embutidas repetidas (`como`, `qual`, `the`, `how`…). E **10 recusadas por "existe em nomes ou caminhos do código"**: `make`, `an`, `on`, `for`, `are`, `every`, `each`, `changing`, `line`, `call`.

A conferência só vale para **propostas**. O léxico salvo à mão, ou importado do `librarian.json`, não passa por ela (seção C). Ver MAP-O46.

---

## Execução (Parada 1)

Tudo no Git Bash. `S` é o scratchpad da sessão.

1. **Original antes**, pelo comando de `docs/reports/piloto-1b/dados/p1/original-before.sha256-agregado.txt`, para cada projeto (`cd .studio && find projects/<key> -type f -print0 | xargs -0 sha256sum | sed 's/ \*/  /' | LC_ALL=C sort -k2 | sha256sum`). Deu healthchecks `61cd25c1…5ca3d5` (932 arquivos) e renderer `612cdbf6…f3ecc` (42), os mesmos da Parada 0 da parte 3 (MAP-O1).
2. **Cópia:** `cp -r .studio/projects/* $S/lex/root/.studio/projects/`.
3. **Studio sobre a cópia**, como o portão faz (`tools/precisao/lib.js:46-51`, com `LIBRARIAN_ROOT`): `LIBRARIAN_ROOT=$S/lex/root target/release/librarian_studio.exe --port 8791`. O binário é de 26/11 04:49, depois do último commit em `crates/` (25/11 20:23). O log confirmou: "Dados: …\scratchpad\lex\root\.studio".
4. **Pedidos:** `POST /api/lexicon/suggest` com `{"key", "run", "profile": "manual"}`, um por projeto. Criou só `lexicon/20261126-110240/query.json` em cada projeto da cópia.
5. **Conferências:** `POST /api/lexicon/check`, com as respostas escritas à mão da seção D, a resposta cortada, `{"outra":1}`, um `id` inexistente e o léxico atual do ray tracer (com o léxico da cópia zerado e depois restaurado).
6. **Fim:** o studio da cópia foi encerrado (`Stop-Process` no processo da porta 8791). O hash do original foi refeito: os mesmos dois valores e contagens do passo 1; `ls .studio/projects/*/` não mostra pasta `lexicon`. A cópia foi apagada (`rm -rf $S/lex`). Ficaram no scratchpad só os dois scripts de contagem (`scripts/vocab.js`, `scripts/grupos.js`), que só leem.

---

## Observações novas (Parada 1)

- **MAP-O43.** O vocabulário que vai ao LLM não tira nada do que a busca tira (`search.rs:55-68` × `:143-157`). Entram os símbolos de teste, os blocos `impl`/`module`/`class` e as stop words embutidas. Nos casos reais:
  - no ray tracer, o 1º termo é `impl` (97), que vem só de nomes de blocos `impl` (excluídos da busca); 35 dos 80 vêm na maior parte de testes; `the`, `from` e `with` são stop words embutidas;
  - no Healthchecks, o 1º é `test` (2.009), e 51 dos 80 vêm na maior parte de testes.

  As instruções pedem ao LLM para ignorar nomes genéricos (`lexicon.rs:63`), mas o filtro não está no código.
- **MAP-O44.** O corte em 80 desempata em ordem alfabética (`search.rs:66`). Nos casos reais, ele parte um empate: no ray tracer, `between` (5) entra e `caps`, `create`, `depth`, `eye` (5) ficam de fora; no Healthchecks, `report` (27) entra e `validates` (27) fica de fora.
- **MAP-O45.** Homônimo: "existe no código" tem três sentidos no código.
  - Para um **grupo**, é ser token de um nome de símbolo, com 3 ou mais caracteres (`lexicon.rs:114`, `search.rs:60`).
  - Para uma **stop word**, é ser token de nome, nome qualificado ou caminho, de qualquer tamanho (`lexicon.rs:177-184`).
  - A **busca** pesa nome, escopo, caminho e corpo (`search.rs:164-169`).

  Caso real: `["identificador","id"]` foi recusado no Healthchecks, com 54 nomes de símbolos que têm o token `id`.
- **MAP-O46.** A conferência só vale para propostas do LLM ou coladas à mão em `/api/lexicon/check`. O léxico salvo pela caixa de texto (`PUT /api/project`) ou importado do `librarian.json` (`store.rs:1507-1508`) não é conferido contra o código. Só passa pela construção do `SearchIndex` na próxima pergunta (palavra única, sem palavra em dois grupos; `search.rs:131-140`). Caso real: o léxico atual do ray tracer tem 2 grupos sem termo do código e 10 stop words que nomeiam algo do código, que a conferência de hoje recusaria (seção D).
- **MAP-O47.** O registro do léxico guarda o pedido, o envio (quem propôs: provedor, perfil, tarefa, data) e a última proposta conferida de cada pedido. Não guarda:
  - qual proposta foi aplicada, quem confirmou, quando, nem o léxico anterior: o `project.json` é regravado inteiro (`store.rs:335`);
  - as conferências anteriores do mesmo pedido: `pasted-response.txt` e `proposal.json` são regravados a cada conferência (`store.rs:1206`, `:1214`; verificado na execução).

  A única ligação entre uma pergunta e o léxico que ela usou é o `lexicon_sha256` (`search.rs:287`).
- **MAP-O48.** Duas instruções do pedido não são conferidas pelo código:
  - "ao menos uma palavra em português" (`lexicon.rs:58`): `["channel","channels"]`, só com termos do código, foi aceito;
  - a palavra nova de um conceito que já tem grupo não é somada a esse grupo: o grupo proposto é recusado inteiro quando sobram menos de duas palavras (`lexicon.rs:137-150`). Caso: `["raio","feixe","ray"]` → `feixe` perdido.
- **MAP-O49.** A conferência da stop word só recusa; não confere que a palavra seja de pergunta. `zzz` foi aceita sem nota. Uma stop word que só aparece no corpo do código é aceita, com nota (`lexicon.rs:195`).
- **MAP-O50.** O pedido do léxico não tem `response_schema`. Por isso, no Ollama, não há `format` estruturado (`ollama/mod.rs:31-37`): a forma da resposta está só nas instruções, e `parse` tolera texto em volta e aproveita grupos de uma resposta cortada (`lexicon.rs:72-106`).
- **MAP-O51.** Nomes parecidos do léxico: a tarefa do catálogo é `lexico` (`catalog.json:60`, `models.rs:32`); a ação é `lexicon` (`catalog.json:115`); a pasta é `lexicon/` e o `purpose` é `"lexicon"` (`llm.rs:52`). "Conferir" aparece como `check` (a função, `lexicon.rs:113`), `check_lexicon` (a rota) e o botão "Conferir resposta" (`index.html:926`). Só o registro, sem juízo.

## Perguntas (Parada 1)

- **MAP-Q16.** O léxico atual do ray tracer foi importado do `librarian.json` do projeto (`imported_from`). Ele tem 10 stop words que a conferência de hoje recusaria (`line`, `call`, `changing`, `make`…) e 2 grupos sem termo do código (seção D). O código não registra se ele passou por alguma conferência. A regra das stop words existe desde `15eb294` (26/10) e a dos grupos desde `2e0837f` (20/10) (`git log -S`). Esse léxico foi feito à mão, ou antes dessas regras? Ele deve continuar como está nos experimentos?
- **MAP-Q17.** O vocabulário que vai ao LLM inclui testes, blocos `impl` e stop words embutidas (MAP-O43), e "existe no código" tem três sentidos (MAP-O45). Isso é para ficar registrado só como fato, ou entra numa lista de pontos a decidir na reestruturação?

---

## Parada 2: a busca até o nó de entrada

**Base:** `main` em `5802a41`. As mesmas regras da Parada 1. A execução foi feita sobre uma **nova cópia** da `.studio`, no scratchpad. Rotas usadas: `/api/ask` no studio da cópia e o comando `search` do `librarian_ingest` sobre a edition da cópia. Para variar `exclude_tests`, `hits` e `radius`, editei o `project.json` **da cópia** (seção "Execução (Parada 2)"). Nenhum LLM foi chamado.

Caminhos abreviados novos: `prepare.rs` e `graph.rs` são de `crates/librarian-ingest/src/`; `claims.rs` é de `crates/librarian-studio/src/`; `graph_engine.rs` é `crates/librarian-graph-engine/src/graph_engine.rs`.

### E. A pergunta, de `/api/ask` até `SearchIndex::search`

**O caminho** (verificado, por leitura):

```
POST /api/ask {key, run, question}          main.rs:184-188
 → Store::ask                               store.rs:461-473
   → SearchIndex::new(catalog, settings.lexicon)      store.rs:464   (search.rs:126-181)
   → .without_tests() se settings.exclude_tests        store.rs:465-467 (search.rs:185-198)
   → .search(question, settings.top, settings.depth)   store.rs:468   (search.rs:200-295)
   → request(settings, question, None, …)              store.rs:471   (store.rs:1378-1401)
   → new_turn → new_turn_with                          store.rs:472 → :874 → :883
       grava search.json (o da busca acima)            store.rs:909-911
       grava prepare-request.json                      store.rs:913
       → prepare::write + prepare::verify              store.rs:915
         → prepare::prepare, ramo sem targets          prepare.rs:118, :160-180
             SearchIndex::new + without_tests + search, DE NOVO, com o léxico, top, depth e
             exclude_tests do pedido                   prepare.rs:167-171
             recusa se hits.len() < request.hits       prepare.rs:172-178
             fichas E1..En dos primeiros `hits`        prepare.rs:189 (take(chosen))
```

A busca roda **duas vezes** com o mesmo pedido: uma no `Store::ask` (para o `search.json` da pergunta) e outra dentro do `prepare` (para o `prepared/search.json`). Leitura minha: as duas são iguais porque a busca é determinística, e o teste `lexical_search_expands_explicit_aliases_and_abstains` confere bytes iguais em duas buscas (`crates/librarian-ingest/tests/editions.rs:49-56`). **Verificado no caso real:** `cmp turns/01/search.json turns/01/prepared/search.json` deu arquivos iguais.

**Como a pergunta vira termos** (verificado, `search.rs:208-221`):
1. `tokens(question)`, a mesma função do léxico (Parada 1, A): minúsculas, sem acento, quebra de identificadores (`:210`).
2. Tira as stop words: as embutidas mais as do léxico, juntadas em `SearchIndex::new` (`:143-146`, `:212`).
3. Cada termo vira o seu grupo de sinônimos inteiro (o "alias"), ou só ele mesmo se não está em grupo nenhum (`:214-218`).
4. Termos do mesmo grupo contam uma vez: a chave é a primeira palavra do grupo em ordem alfabética (`:219`), e o primeiro termo escrito é o que fica como `term` (`:220`). O comentário diz ser contra o plural inflar a cobertura (`:208`).

**O que se busca** (verificado, `search.rs:147-172`): um "documento" por símbolo da edition, menos os de tipo de navegação do extrator (`impl` e `module` no Rust, `class` no Python; `:150-157`). Cada documento tem 4 campos, cada um com um peso:

| Campo | Peso | De onde |
|---|---:|---|
| `name` | 8 | tokens do nome |
| `scope` | 3 | tokens do nome qualificado |
| `path` | 1 | tokens do caminho do arquivo |
| `code` | 1 | tokens do trecho inteiro do símbolo (corpo, comentários, strings) |

**Pontuação e ordem** (verificado, `search.rs:222-280`):
- Para cada termo, o peso é o **maior** dos campos onde algum sinônimo aparece (`:226-233`). Cada termo que pega gera um `MatchReason` com o termo, os sinônimos e os campos (`:234-241`).
- **Corte por cobertura:** o símbolo só entra se pegar pelo menos 60% dos termos (`reasons*5 < groups*3` → fora, `:243`).
- **Pontos:** `100 × termos que pegaram + soma dos pesos − 20 se é teste` (`:248-249`).
- **Ordem:** pontos (maior primeiro); depois o que não é teste; depois o trecho menor; depois o id (`:252-261`).
- **Corte:** até `top` hits. Pula um símbolo cujo trecho se sobrepõe ao de um hit melhor do mesmo arquivo (`:270-278`). Cada trecho é cortado em 4.000 bytes, e o total dos trechos em 16.000 (`:263`, `:299-308`).
- **Situação:** `status` vale `candidates`, ou `no_lexical_evidence` sem hits (`:289`). `limits` traz os textos do perfil do extrator e, com `exclude_tests`, mais um (`:291-293`).

**Caso real** (cópia; `/api/ask`; as configurações gravadas: `top` 5, `depth` 2, `hits` 1, `radius` 0):

| Projeto · pergunta | `terms` | Hits (pontos) | Termos sem hit |
|---|---|---|---|
| ray tracer · "Como a câmera calcula a direção do raio para um pixel?" | `camera`, `direcao`, `pixel`, `raio` (`como`, `a`, `do`, `para`, `um` são embutidas; `calcula` é stop word do léxico) | 5: `impl Camera::ray_from_pixel` (420), três `tests_camera::constructing_a_ray_…` (398, 393, 393), `impl Camera::render` (305) | nenhum |
| Healthchecks · "Como o ping atualiza o status do check?" (léxico vazio) | `atualiza`, `check`, `ping`, `status` | 5, todos com 3 dos 4 termos: `Check.ping` (312), `ping_body` em dois arquivos (310), `status` (310), `check_urls` (310) | `atualiza` |
| ray tracer · "Onde fica o banco de dados de usuários?" | `banco`, `dados`, `fica`, `usuarios` | 0: `status: "no_lexical_evidence"`; `prepare_error: "Search returned 0 candidates; 1 requested"` | os 4 |

Conta dos pontos do 1º hit do ray tracer, a partir dos `reasons` gravados: 4 termos × 100, mais `camera` 3 (escopo e caminho), `direcao` 1 (código), `pixel` 8 (nome) e `raio` 8 (nome), dá 420. Os testes ficam 20 abaixo: `constructing_a_ray_when_the_camera_is_transformed` dá 4×100 + 8 + 1 + 1 + 8 − 20 = 398.

**O dicionário decide o caso do ray tracer** (CLI sobre a edition da cópia, `librarian_ingest search <edition> "<pergunta>" - 5 2`, sem léxico): `terms` = `calcula`, `camera`, `direcao`, `pixel`, `raio` e **0 hits**. Só `camera` e `pixel` existem no código: 2 de 5 termos ficam abaixo dos 60%.

**Acréscimo 1: por que as 14 perguntas do Healthchecks têm `terms: []`.** Elas **não passaram pela busca**. Foram feitas por outro caminho, o da pergunta "reunida" (P4). O léxico vazio não é a causa: nesse caminho o léxico não é usado. Verificado, por leitura e nos arquivos da cópia:
- **Os registros:** o `question.json` da pergunta 01 tem o campo `gathered` com os `nodes` escolhidos. O `prepare-request.json` tem `"gathered": true` e 5 `targets` (e `lexicon` vazio, que não é lido). O `search.json` das 14 tem `version: "chain-step-v1"`, `status: "gathered"` e `terms: []`.
- **O caminho:**

```
página: com declarações reunidas (ou uma investigação), manda /api/gathered-question;
        sem elas, /api/ask                                   index.html:4274-4279
POST /api/gathered-question                                  main.rs:155-163
 → Store::ask_gathered → ask_gathered_parts                  store.rs:503-506, :515-557
   → request(settings, question, Some(targets), …)           store.rs:554
     request.gathered = true                                 store.rs:555
   → new_turn_with(…, search: None, …)                       store.rs:557
     → prepare::prepare, ramo com targets                    prepare.rs:122-158
       origem StepOrigin::Gathered                           prepare.rs:128-133
       → chain_report (nada é buscado)                       prepare.rs:154 → search.rs:339-358
           terms: vec![], status "gathered", lexicon_sha256: ""   search.rs:345-346, :355-357
     → sem search próprio, copia prepared/search.json para a pergunta   store.rs:920-922
```

- O próprio pedido diz que, num passo com alvos, "`top`, `hits` and `lexicon` are then unused" (diz ser: `prepare.rs:39-40`). O texto de limite gravado diz: "Gathered step: declarations the person gathered one by one; nothing was searched." (`search.rs:346`).
- **Homônimo na página:** para `terms: []`, a página mostra "nenhum termo após as stop words" (`index.html:5443`, texto `turn.noTerms`, `:1343`). Isso vale também para as perguntas reunidas, em que nenhuma stop word foi tirada porque nada foi buscado. Ver MAP-O52.

### F. As configurações da busca

**Onde moram:** todas no `Settings` do `project.json` (`store.rs:187-222`), editadas na página em Configurações (`index.html:502-509`) e gravadas por `PUT /api/project` (`save_settings`, `store.rs:325-337`). Os padrões de um projeto novo estão em `store.rs:1479-1496`. O `librarian.json` do projeto pode trazer `search.top`, `search.depth`, `search.hits`, `search.exclude_tests`, `prepare.radius` e `prepare.numbered_lines`, lidos só ao abrir o projeto (`store.rs:1517-1522`). Toda pergunta copia as configurações para o pedido do `prepare` (`request`, `store.rs:1378-1401`), gravado em `prepare-request.json` e em `prepared/request.json`.

**As quatro que o usuário lembra**, mais as outras do código (verificado, por leitura; efeitos conferidos na cópia):

| Configuração | Padrão | Limites e onde são conferidos | Muda o que a busca acha? | Quem lê |
|---|---|---|---|---|
| **`top`**: quantos resultados a busca devolve | 5 (`store.rs:1481`) | 1 a 10: `search.rs:203` (recusa a pergunta: "top 1..10"); página `min=1 max=10` (`index.html:503`) | **Corta** a lista: quais nós entram como hits, até `top`. Não muda quem passa nos 60% nem a ordem. Caso: "Como a esfera calcula a normal?" deu 1, 5 e 10 hits com `top` 1, 5 e 10 (CLI); `top` 11 → recusa | `search.json` (a lista da página, `#1…#n`, `index.html:2687`, `:5394-5397`). Ao pacote, só os primeiros `hits` vão como fichas; os outros ficam só no `search.json` (diz ser: `prepare.rs:25`), cujo hash entra no pacote (`search_sha256`, `evidence.rs:872`) |
| **`hits`**: quantos dos primeiros viram fichas E1…En | 1 (`store.rs:1483`) | 1 a `top`: `save_settings` (`store.rs:331`) e `prepare` (`prepare.rs:161-162`); página `min=1 max=10` (`index.html:505`) | **Não.** Escolhe quantos dos hits já achados vão ao pacote (`prepare.rs:189`). Se a busca achou menos que `hits`, o `prepare` recusa ("Search returned N candidates; M requested", `prepare.rs:172-178`), a recusa fica no `question.json` e não há pacote (`store.rs:915-918`) | **A resposta para fora**: as fichas da consulta. Na página, a marca `E1…En` × `#n` (`index.html:2687`, `:5390`, `:5396`). Num passo com alvos, não é usado (`prepare.rs:39-40`) |
| **`depth`**: quanto dos vizinhos acende no mapa | 2 (`store.rs:1482`) | 0 a 3: `search.rs:204` e `graph.rs:163`; página `min=0 max=3` (`index.html:504`). Tetos fixos: 48 nós e 96 arestas (`search.rs:284`) | **Não.** Só a vizinhança do **primeiro** hit (`search.rs:281-285`). Caso (CLI, ray tracer): `depth` 0 → 1 nó, 1 → 36 nós e 35 arestas, 2 → 48 e 52 (cortada), 3 → 48 e 61 (cortada); os hits e os pontos não mudam | **Só a página**: acende os nós e as arestas (`attachSearch`, `index.html:2690-2698`). A vizinhança fica no `search.json` (`graph`), mas **não vai na consulta**: no caso real, `claims/query.json` só tem `evidence.items`, `limits` e `line_map`. Vai só o texto fixo "Graph expands only the first hit… Depth bounded to 0..3, 48 nodes, 96 edges" nos limites. Também é usado num passo com alvos (`prepare.rs:154`) |
| **`radius`**: linhas a mais acima e abaixo da ficha | 0 (`store.rs:1484`) | ≥ 0, sem teto. Só é cortado no começo e no fim do arquivo (`graph_engine.rs:217-218`); página `min=0` (`index.html:506`) | **Não.** Só o recorte de linhas montado depois (`prepare.rs:427` → `prepare_query_formatted`). Caso (cópia, `radius` 3): fichas 52–54 → 49–57, 102–111 → 99–114, 47–178 → 44–178 | **A resposta para fora**: as linhas de cada ficha na consulta. Também registrado em `composition.radius` (`prepare.rs:436`) |
| `exclude_tests` | `false` (`store.rs:1496`) | sim/não; só para a busca lexical (`store.rs:1391`: `settings.exclude_tests && targets.is_none()`) | **Sim**: tira os símbolos de teste e o que está dentro deles (`search.rs:185-198`). Caso: "Como a esfera calcula a normal?" deu 3 hits, nenhum teste (com `top` 5); sem ele, 5 hits, 2 de teste no top 5 e 7 no top 10 | **A resposta para fora**: um limite a mais, "Test symbols marked by the extractor … were left out" (`search.rs:292`), que vai aos `unknowns` (`prepare.rs:457`) e aparece nos `limits` da consulta. E o `search.json` |
| `lexicon` | vazio (`store.rs:1479`) | palavra única e sem palavra em dois grupos, conferidos ao construir o índice (`search.rs:131-140`) | **Sim**: stop words e sinônimos mudam os termos e os acertos (caso: 5 hits com o léxico, 0 sem) | O pedido do `prepare` (com o léxico inteiro, `prepare.rs:22`) e o `search.json` (`lexicon_sha256`). A consulta não leva o léxico |
| `numbered_lines` | `true` num projeto novo (`store.rs:1485`); `false` se o `librarian.json` não diz (`store.rs:1522`) | sim/não | **Não**: a forma das linhas (`prepare.rs:418-422`) | A resposta para fora |
| `accept_diagnostics` | `false` | sim/não | **Não**: o `prepare` recusa uma edition com diagnósticos (`prepare.rs:119-121`) | O pacote (recusa ou não) |

**Fixos no código, sem configuração** (verificado):
- cobertura mínima de 60% (`search.rs:243`);
- pesos 8, 3, 1 e 1 (`:165-168`);
- desconto de 20 para teste (`:249`);
- pergunta de até 8.192 bytes (`:202`);
- trecho de até 4.000 bytes por hit e 16.000 no total (`:263`, `:304`);
- vizinhança com até 48 nós e 96 arestas (`:284`), e rótulos de até 256 bytes (`graph.rs:208-215`, `:228`).

Diferença entre caminhos: a CLI usa `top` 5 e `depth` **1** por padrão (`cli.rs:13-14`), e o studio usa `depth` 2; a CLI não tem `exclude_tests`.

**A palavra "hits" tem dois sentidos** (homônimo). No relatório da busca, `hits` é a lista de resultados, até `top` (`search.rs:102`). Nas configurações e no pedido, `hits` é quantos desses viram fichas (`prepare.rs:25-26`). A página diz o segundo: "Quantos primeiros resultados viram fichas E1…En" (`index.html:953`). Ver MAP-O53.

### G. O nó de entrada

**Qual é** (verificado): só o **primeiro hit**. A vizinhança é `graph.neighborhood(&hits[0].symbol_id, depth, 48, 96)` (`search.rs:281-285`). Os outros hits não são expandidos (o limite diz isso: "Graph expands only the first hit", `extractors/mod.rs:143`).

**Como se percorre** (verificado, `graph.rs:155-232`):
- É uma busca em largura a partir do nó do primeiro hit, sobre o grafo sintático (`Graph::build`, `search.rs:281`), nos **dois sentidos** de cada aresta (`graph.rs:171-175`, `:186-190`), até `depth` níveis.
- Um nó `source` é folha (`:181`).
- Ao bater no teto de nós ou de arestas, para de crescer e marca `truncated: true` (`:191-195`).
- As arestas seguidas são de **todos** os tipos do grafo sintático: `declares`, `contains`, as de observação (`call_observed`, `attribute_access`, `receiver_type`…) e também `name_candidate`, a aresta de nome para cada homônimo (parte 3, A).

**O que acende na página** (verificado, `attachSearch`, `index.html:2675-2719`):
- os hits que têm nó no grafo v2 da página, com rótulo `E1…En` (fichas) ou `#n` (`:2687`, `:2707-2711`);
- os nós da vizinhança, casados **por id** com o grafo v2 (`:2690-2693`);
- as arestas, casadas por `from|to|bytes da evidência` (`:2694-2698`). O grafo v2 renomeia as de observação para `observes` (parte 3, A), mas a chave é a mesma;
- uma nota: "Vizinhança: {n} pontos e {m} ligações do primeiro resultado, associados por ID" (`:2715`, texto `:1023`) e, se algum hit não tem nó, "{n} resultado(s) sem nó correspondente no grafo" (`:2698`, `:2716`).

**Caso real** (cópia; contagem com `node` sobre o `search.json` e o `graph-v2.json`):

| | ray tracer, pergunta 01 | Healthchecks, pergunta 15 |
|---|---|---|
| Nó de entrada | `impl Camera::ray_from_pixel` (`src/camera.rs:81-93`) | `Check.ping` (`hc/api/models.py:493-573`) |
| Vizinhança (`depth` 2) | 48 nós (o teto), 52 arestas, `truncated: true` | 48 nós (o teto), 48 arestas, `truncated: true` |
| Casam com o grafo v2 | 48 de 48 nós, 52 de 52 arestas | 48 de 48, 48 de 48 |
| Arestas `name_candidate` acesas | 13 | 5 |
| Hits com nó no v2 | 5 de 5 | 5 de 5 |

**Acréscimo 3: o que fica de fora, e se é declarado** (verificado, por leitura e nos arquivos da cópia):

| O que fica de fora | Gravado? | Declarado na consulta (o que vai para fora)? | Na página? |
|---|---|---|---|
| Hits depois dos primeiros `hits` (no caso: 4 de 5) | sim, em `search.json`, com pontos e `reasons` | **não**. A consulta só tem as fichas (no caso, `items` = só `E1`) e os limites fixos. Nada diz que houve outros candidatos | sim, na lista, como `#2…#5` (`index.html:5394-5397`) |
| Candidatos depois de `top` | **não**: a lista ordenada é descartada depois do corte (`search.rs:264-280`) | não | não |
| Candidatos abaixo dos 60% de cobertura | não (`search.rs:243`) | só o texto fixo "Minimum coverage: 60% of query term groups" | não |
| Símbolos pulados por sobreposição com um hit melhor | não (`search.rs:270-278`) | não | não |
| Trecho cortado pelo limite de bytes | sim: `excerpt_truncated` por hit (`search.rs:321`) | só o texto fixo "Excerpts limited to 4000 UTF-8 bytes each and 16000 total" | — |
| Termos sem hit (no caso do Healthchecks: `atualiza`) | sim, de forma indireta: `terms` e os `reasons[].term` estão no `search.json`; a diferença não é um campo | **não** | sim: cada termo vira uma etiqueta "tocou" ou "não tocou nenhum resultado" (`index.html:5439-5442`, textos `:1341-1342`). Também alimenta os `missed_terms` do léxico (Parada 1, A) |
| Vizinhança cortada pelo teto | sim: `graph.truncated` no `search.json` (`graph.rs:229`) | não (a vizinhança não vai) | **não**: `attachSearch` não lê `truncated` (nenhum texto de `truncated` da página é do overlay: `grep -n truncated index.html`) |
| Os outros hits como pontos de entrada | não se aplica: não são expandidos | só o texto fixo "Graph expands only the first hit" | os rótulos `#n` aparecem; a vizinhança deles, não |

Os textos fixos chegam à consulta assim: `search.limits` → `report_limits` (`prepare.rs:445-461`) → `unknowns` do dossiê (`prepare.rs:408-411`) → `evidence_unknowns` de cada seleção → os `limits` da consulta de claims (`claims.rs:64-68`). Verificado no caso real: os 5 `limits` de `claims/query.json` são os 4 do perfil mais "Fichas descrevem onde estão declarações; não respondem à pergunta nem afirmam comportamento" (`prepare.rs:349-354`).

**Acréscimo 4: a busca confere "teve hit no código"?** Além da conferência do léxico (Parada 1, D), há quatro pontos no código (verificado). Nenhum deles confere termo a termo:
1. **Por candidato:** o corte de 60% (`search.rs:243`). Um símbolo com menos termos não é hit.
2. **Pela busca inteira:** `status: "no_lexical_evidence"` quando não há hit (`search.rs:289`). A busca não falha; o relatório é gravado (caso real: pergunta 02 do ray tracer).
3. **No `prepare`:** recusa quando há menos hits que `hits` (`prepare.rs:172-178`). O erro fica em `prepare_error` no `question.json` (`store.rs:915-918`), a pergunta não ganha pacote e não pode ser enviada (diz ser: `store.rs:870-872`). Caso real: "Search returned 0 candidates; 1 requested". A página mostra "Nenhum resultado: o léxico não alcançou símbolos do projeto" (`index.html:5447`, texto `:1344`).
4. **Na CLI:** `librarian_ingest search` sai com código 2 sem hits (`cli.rs:17`).

**Não há** conferência de que **cada termo** da pergunta tem hit: um termo que não toca nada (como `atualiza`) passa sem aviso na consulta. Só a página o marca. Ver MAP-O54.

### H. Os testes do léxico e da busca

Achados com `git grep -lE "SearchIndex|search::|vocabulary\(|tokens\(|\.search\(|lexicon"` em `crates/*/tests/*.rs` e `crates/*/src/*.rs`, e lidos um a um. **O que o método não pega:** testes que exercitam a busca só por dentro de outra rota sem citar esses nomes (por exemplo, testes de `ask_path` ou da escada que chamam `Store::ask`), e as baterias de tela (`.js`), que ficam fora do cargo.

| Teste | Arquivo:linha | O que verifica |
|---|---|---|
| `lexical_tokens_handle_portuguese_and_identifier_boundaries` | `crates/librarian-ingest/tests/editions.rs:11` | `tokens` tira acentos (também os combinantes) e quebra `ray_from_pixel` e `HTTPServer` |
| `lexical_search_expands_explicit_aliases_and_abstains` | `editions.rs:21` | Os sinônimos levam a `ray_from_pixel` com 3 razões; o trecho traz o corpo; uma pergunta sem acerto dá `no_lexical_evidence`; só stop words dão 0 hits; pergunta vazia, `top` 0 e `depth` 4 são recusados; duas buscas iguais dão os mesmos bytes |
| `python_methods_are_searched_and_prepared_like_rust` | `crates/librarian-ingest/tests/python.rs:152` | Em Python, a classe é navegação e não é hit; o método é. O `prepare` monta a ficha e as linhas numeradas |
| `vocabulary_lists_name_tokens_by_frequency` | `python.rs:202` | `vocabulary` conta por símbolo, deixa de fora tokens curtos e ordena por frequência e depois alfabeticamente |
| `a_search_without_tests_leaves_out_test_functions_and_what_is_written_inside_them` | `crates/librarian-ingest/tests/calls.rs:166` | `without_tests` tira a função de teste e o auxiliar escrito dentro dela, mantém o resto e acrescenta o limite |
| `search_matches_frozen_v1_goldens` | `crates/librarian-ingest/tests/v1_regression.rs:173` | Os bytes da busca são iguais aos congelados em `tests/fixtures/v1/` (`search-camera.json`, `search-abstain.json`, `search-test-penalty.json`, com o `lexicon.json` de lá), inclusive o desconto de teste |
| `cut_off_answers_keep_only_complete_groups` | `crates/librarian-studio/src/lexicon.rs:215` | `parse` aproveita só os grupos inteiros de uma resposta cortada e marca `recovered`; uma resposta inteira é lida sem a marca; sem JSON aproveitável, erro |
| `open_generate_and_ask_light_the_graph_and_prepare_a_numbered_query` | `crates/librarian-studio/src/store.rs:2041` | `Store::ask` com léxico: o 1º hit é `ray_from_pixel`, a consulta tem linhas numeradas, o envio fica ligado ao pacote conferido; uma pergunta sem acerto grava a busca com a recusa do `prepare`; numeração das perguntas |
| `lexicon_proposals_are_checked_and_profiles_exported_on_request` | `store.rs:2554` | `missed_terms` (`fica`, `hoje`); o pedido `manual`; o pedido em inglês com os mesmos termos; idioma desconhecido recusado; a conferência de grupos e de stop words (Parada 1, D); resposta cortada; `id` com `../` recusado; exportação do perfil |
| `routes_that_make_no_question_make_no_package` | `store.rs:7136` | Entre outras rotas, `/api/lexicon/suggest` não cria pergunta nem pacote |
| `routes_typed_questions_by_words_and_lexicon` | `crates/librarian-studio/src/questions.rs:503` | A pergunta de performance usa os sinônimos do léxico para reconhecer as palavras da regra |

**Execução** (só esses):
- `cargo test --locked -p librarian-ingest --test editions --test python --test calls --test v1_regression -- --exact <os 6 nomes>`: **exit 0, 6 aprovados**, 0 falhas;
- `cargo test --locked -p librarian-studio --bin librarian_studio -- --exact <os 5 nomes com o módulo>`: **exit 0, 5 aprovados**, 0 falhas (177 filtrados).

Logs no scratchpad (`testes-ingest.log`, `testes-studio.log`). `git status --porcelain` depois: vazio.

---

## Execução (Parada 2)

1. **Original antes:** o mesmo comando da Parada 1 deu os mesmos dois hashes e contagens (healthchecks `61cd25c1…5ca3d5`, 932; renderer `612cdbf6…f3ecc`, 42).
2. **Cópia nova:** `cp -r .studio/projects/* $S/busca/root/.studio/projects/`. Studio sobre ela: `LIBRARIAN_ROOT=$S/busca/root target/release/librarian_studio.exe --port 8792`; o log confirmou a pasta de dados da cópia.
3. **Perguntas** (`POST /api/ask`), nesta ordem: ray tracer "Como a câmera calcula a direção do raio para um pixel?" (virou a pergunta 01 do run); Healthchecks "Como o ping atualiza o status do check?" (15); ray tracer "Onde fica o banco de dados de usuários?" (02).
4. **CLI:** `target/release/librarian_ingest.exe search <edition da cópia> "<pergunta>" <léxico extraído da cópia | -> <top> <depth>`, com `top` 1, 5, 10 e 11 e `depth` 0 a 4.
5. **Configurações na cópia:** no `project.json` do ray tracer **da cópia**, `exclude_tests: true`, `hits: 3`, `radius: 3`. Depois, `/api/ask` "Como a esfera calcula a normal?" (virou a pergunta 03).
6. **Testes:** seção H.
7. **Fim:** o studio da cópia foi encerrado (`Stop-Process` no processo da porta 8792). O hash do original foi refeito: os mesmos dois valores e contagens do passo 1; `ls .studio/projects/renderer-9b78f3a9/runs/*/` não mostra `turns`. A cópia foi apagada (`rm -rf $S/busca`).

---

## Em palavras simples

*Esta seção está à parte das seções E a H: sem números nem nomes de arquivo. Foi escrita com a ideia da skill "elementary school teacher" (nível newcomer).*

Pense na biblioteca de novo. A pessoa chega ao balcão com uma pergunta escrita.

**Primeiro, o bibliotecário limpa a pergunta.** Ele tira as palavras de ligação ("como", "o", "de") e troca cada palavra pelo seu grupo de sinônimos do dicionário. "Raio" vira "raio ou ray". Se o dicionário estiver vazio, uma pergunta em português não encontra nomes em inglês.

**Depois, ele procura nas fichas do catálogo.** Cada ficha é uma parte do código. Uma palavra que aparece no nome vale muito, no caminho da estante vale pouco, e dentro do texto vale pouco também. Uma ficha só entra na lista se combinar com a maior parte das palavras da pergunta. As fichas de teste perdem um pouco de pontos.

**Ele separa as melhores.** Fica com as primeiras da lista, até um número combinado. Dessas, só as primeiras vão para a mesa de quem responde; as outras ficam anotadas no registro da pergunta. Se ele não acha nenhuma, a pergunta fica registrada, mas não é preparada para envio.

**Ele acende o mapa.** Só a primeira ficha vira o ponto de partida no mapa da cidade. Dali, acende os vizinhos até uma distância combinada, com um limite de tamanho. Entre os vizinhos aparecem também os "mesmos nomes" (homônimos), que não são ligações certas. O mapa é só para a pessoa ver: quem responde não recebe o mapa, só as fichas.

**O que não se avisa a quem responde.** Que havia outras fichas na lista, que alguma palavra da pergunta não achou nada e que o mapa foi cortado pelo limite. Isso fica no registro ou aparece só para a pessoa na tela.

**Um outro caminho de pergunta.** A pessoa também pode juntar as fichas com a mão e perguntar sobre elas. Aí o bibliotecário não procura nada, e o dicionário não entra. Todas as perguntas guardadas do segundo projeto de teste foram feitas assim.

---

## Observações novas (Parada 2)

- **MAP-O52.** As 14 perguntas do Healthchecks são "reunidas" (`/api/gathered-question`) e não buscaram. O `terms: []` vem de `chain_report` (`search.rs:355-357`), não do léxico vazio. A página mostra o mesmo texto, "nenhum termo após as stop words", para uma busca que tirou todos os termos e para uma pergunta que não buscou (`index.html:5443`). Por isso os `missed_terms` do Healthchecks são 0 (Parada 1, A): o léxico do Healthchecks nunca foi exercitado por uma pergunta nessa `.studio`.
- **MAP-O53.** Homônimo `hits`: no relatório da busca, é a lista de resultados (até `top`); nas configurações e no pedido, é quantos deles viram fichas (`search.rs:102` × `prepare.rs:25-26`).
- **MAP-O54.** A consulta que vai para fora não declara o que ficou de fora da busca: os hits além de `hits`, os termos sem hit e o corte da vizinhança. Só leva os textos fixos do perfil. O `search.json`, cujo hash entra no pacote (`evidence.rs:872`), guarda os hits até `top`, os termos e as razões; os candidatos depois de `top`, abaixo dos 60% ou pulados por sobreposição não são gravados em lugar nenhum (`search.rs:243`, `:264-280`). Na página, a vizinhança cortada (`truncated`) não é dita.
- **MAP-O55.** O mapa acende a vizinhança do grafo **sintático**, nos dois sentidos, com as arestas `name_candidate` (13 no caso do ray tracer, 5 no do Healthchecks). Ele não usa as ligações resolvidas de `/api/calls` (parte 3, C). Leitura minha: o que acende em volta do nó de entrada inclui homônimos, que a regra de ligação não afirmaria.
- **MAP-O56.** A busca de `/api/ask` roda duas vezes por pergunta, no `Store::ask` e no `prepare` (`store.rs:468`, `prepare.rs:171`). No caso real, os dois `search.json` são iguais.
- **MAP-O57.** Padrões diferentes conforme o caminho: `depth` 2 no studio × 1 na CLI (`store.rs:1482`, `cli.rs:14`); `numbered_lines` `true` num projeto novo × `false` num `librarian.json` que não diz nada (`store.rs:1485`, `:1522`). `radius` não tem teto além do tamanho do arquivo (`graph_engine.rs:217-218`).

## Perguntas (Parada 2)

- **MAP-Q18.** As 14 perguntas do Healthchecks foram todas feitas pelo caminho "reunido" (MAP-O52). O código não diz por quê. Foi uma escolha do piloto (escolher as declarações à mão em vez de buscar)? Ou ainda falta exercitar a busca pelo léxico no Healthchecks?
- **MAP-Q19.** "Quanto dos vizinhos acende no mapa" é o `depth`, que só a página lê (seção F). Ele também vale para os passos com alvos (`prepare.rs:154`) e acende homônimos (MAP-O55). É essa a configuração de que o usuário lembrava, ou havia outra (por exemplo, um controle só na página)? No código não há outra além do botão que liga e desliga o destaque (`index.html:1018`).
