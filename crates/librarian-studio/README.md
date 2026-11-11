# librarian-studio

Servidor local do [Librarian Studio](../../STUDIO.md). Como iniciar e usar a página está
no `STUDIO.md`; aqui ficam só os detalhes técnicos.

```powershell
cargo run --release --locked -p librarian-studio [-- --port 8765]
cargo test --locked -p librarian-studio
```

Variáveis: `LIBRARIAN_ROOT` (raiz do Librarian; padrão, a do workspace) e
`LIBRARIAN_CLAUDE` (caminho do `claude.exe`).

## Módulos

| Arquivo | Papel |
| --- | --- |
| `src/main.rs` | Escuta em `127.0.0.1`, recusa `Host` e `Origin` de fora, roteia e envia com o vínculo da conferência |
| `src/http.rs` | HTTP/1.1 mínimo sobre `std`: uma requisição por conexão, corpo por `Content-Length` (até 16 MiB) |
| `src/store.rs` | Projetos, execuções e perguntas em `.studio/`; importa `librarian.json`; grava critérios antes da query; confere a seleção e grava o pacote de evidências de cada pergunta nova; reconfere bundle, claims, critérios e o pacote (H1 a H3) antes de cada envio e monta o vínculo; refaz a sugestão (H4); Modelfiles; navegação de pastas |
| `build.rs`, `src/builder.rs` | Grava na compilação o commit e o `git_dirty` do Librarian (ou `unknown`), que o pacote registra como `builder` |
| `web/index.html` | A página: o mapa como cidade (Three.js 0.180.0 fixado no jsDelivr), o fluxograma, projeto e conversa |
| `src/lexicon.rs` | Pedido de léxico, leitura da resposta e conferência mecânica de cada grupo |
| `src/facts.rs` | L4: portões, revisão por item (`reviews.jsonl`), acervo de fatos (`facts.jsonl`) e reconferência por texto contra uma edition |
| `src/llm.rs` | Provedores: Ollama (`ollama create` + `attempt_linked` do LLM Engine, com o vínculo), Claude Code CLI (mesma conferência do hash da query) e os ainda não implementados |

O studio usa os crates do workspace (`librarian-ingest`, `librarian-llm-engine`) e não
adiciona dependências: o `reqwest` é o mesmo do LLM Engine. O contrato do pacote de
evidências (tipos, `lines-v1`, conferências puras) fica em `librarian-ingest`, em
`src/evidence.rs`, sem nenhuma regra de linguagem.

## Evidências

O **Evidence Builder** escolhe, por regras e sem modelo, o que vai ao LLM: sugere fontes a
partir das medições e do grafo, e a pessoa junta, tira ou aceita. Desde a Etapa 4:

- **Seleção.** A página manda o registro da seleção (formato 2) e os hashes das sugestões que
  usou. O servidor confere tudo contra os registros de sugestão e a edition antes de criar a
  pergunta, e recusa `notes`.
- **Pacote.** Cada pergunta nova ganha um pacote de evidências (`evidence.json`): de onde veio
  cada parte da query, quem declarou cada coisa (`client`, `server`, `server_lacuna`,
  `server_retry`) e os hashes que a ligam à edition.
- **Conferências.** H1 (integridade dos arquivos), H2 (cada trecho sai de novo da edition) e
  H3 (o código atual refaz a mesma query e o mesmo pacote) rodam antes de cada envio; se uma
  falha, nada é enviado. H4 refaz a sugestão e só informa.
- **Consulta.** `GET /api/evidence` mostra, sem gravar nada, o que foi gravado e o que foi
  conferido agora; a conversa mostra o mesmo na seção "Pacote de evidências e conferências".
- **Formatos.** Pacotes de formato 1 (gravados até 11/11/2026, antes da P1) continuam valendo
  como foram gravados, com o aviso de que o `measure_diverges` deles nunca foi calculado. O formato 2
  grava a comparação das medições (`measure_divergence`), que não bloqueia o envio.
- **Limites.** As conferências provam que os arquivos são consistentes, não que uma resposta
  é verdadeira, que as evidências bastam ou que uma pessoa agiu.

Detalhes no [STUDIO.md](../../STUDIO.md#pacote-de-evidências-etapa-4).

## API

A tabela lista as rotas principais; a lista completa está em `src/main.rs`.

| Rota | Função |
| --- | --- |
| `GET /` | `web/index.html`, lido do disco a cada requisição (editar a página não exige recompilar) |
| `GET /api/status` | Ollama (executável, se responde, modelos), Claude CLI, provedores e Modelfiles |
| `GET /api/browse?path=` | Subpastas (marca as que têm `Cargo.toml`); sem `path`, unidades e pasta do usuário |
| `POST /api/project {root}` | Abre ou cria o projeto da pasta |
| `GET /api/project?key=` · `PUT /api/project {key, settings}` | Lê ou salva as configurações (a pasta não muda) |
| `POST /api/generate {key}` | Nova execução: edition, grafo v2 e conferência |
| `GET /api/graph?key=&run=` · `GET /api/turns?key=&run=` | Grafo e perguntas de uma execução |
| `POST /api/ask {key, run, question}` | Critérios, busca, `prepare` e `verify` |
| `POST /api/envelope/suggest {key, run, nodes}` | Sugestão do envelope, gravada em `suggestions/<sha256>.json`; a resposta traz o hash |
| `POST /api/gathered-question {key, run, question, nodes, with_facts?, origin?}` | Pergunta com as fontes juntadas; recusa `notes` em qualquer parte do corpo e confere a seleção (formato 2) antes de criar |
| `GET /api/evidence?key=&run=&turn=` | Pacote e registro da seleção como gravados, H1 a H3 conferidos agora, H4 refeito agora; só leitura |
| `POST /api/send {key, run, turn, provider, modelfile?, claude_model?}` | `verify-prepared`, H1 a H3 do pacote e envio; `501` para provedores não implementados |
| `POST /api/paste {key, run, turn, text}` | Resposta trazida de fora, com a mesma conferência do envio |
| `POST /api/ladder` | Escada: recusa `notes` em qualquer parte do corpo, confere a seleção na entrada, pergunta, envia e registra o que as rodadas de lacuna acrescentaram |
| `POST /api/continue {key, run, turn, question}` | Cadeia: nova pergunta com as declarações das respostas que passam de novo na mecânica e os fatos vigentes sobre elas |
| `POST /api/review {key, run, turn, send, claim, decision, note}` | Aprova (`approved`, com nota, passando pelos portões) ou recusa (`rejected`, revoga a promoção anterior) um item |
| `GET /api/facts?key=&run=` | Acervo de fatos reconferido contra a edition da execução (vigente, desatualizado, revogado) |
| `DELETE /api/turn {key, run, turn}` | Apaga a pergunta e tudo o que foi enviado para ela |
| `POST /api/lexicon/suggest {key, run, provider, modelfile?, claude_model?}` | Vocabulário da edition, pedido ao LLM (ou só o pedido, em `manual`) e conferência dos grupos |
| `POST /api/lexicon/check {key, run, id, text}` | Confere uma resposta colada para um pedido anterior |
| `POST /api/project/export {key, dry_run}` | Lista (`dry_run` true, padrão) ou grava `librarian.json` e os arquivos dele no projeto |
| `GET /api/modelfiles` · `PUT /api/modelfiles {name, content}` | Lista ou salva `modelfiles/<nome>.Modelfile` (exige `FROM`) |

Erros voltam como `{"error": "..."}` com status 400, 403, 404 ou 501.
