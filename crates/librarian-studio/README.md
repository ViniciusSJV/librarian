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
| `src/main.rs` | Escuta em `127.0.0.1`, recusa `Host` e `Origin` de fora, roteia e reconfere o bundle antes de cada envio |
| `src/http.rs` | HTTP/1.1 mínimo sobre `std`: uma requisição por conexão, corpo por `Content-Length` (até 16 MiB) |
| `src/store.rs` | Projetos, execuções e perguntas em `.studio/`; importa `librarian.json`; grava critérios antes da query; Modelfiles; navegação de pastas |
| `web/index.html` | A página: grafo 3D (Three.js 0.180.0 e d3-force-3d 3.0.6 fixados no jsDelivr), projeto e conversa |
| `src/lexicon.rs` | Pedido de léxico, leitura da resposta e conferência mecânica de cada grupo |
| `src/llm.rs` | Provedores: Ollama (`ollama create` + `attempt_linked` do LLM Engine), Claude Code CLI e os ainda não implementados |

O studio usa os crates do workspace (`librarian-ingest`, `librarian-llm-engine`) e não
adiciona dependências: o `reqwest` é o mesmo do LLM Engine.

## API

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
| `POST /api/send {key, run, turn, provider, modelfile?, claude_model?}` | `verify-prepared` e envio; `501` para provedores não implementados |
| `DELETE /api/turn {key, run, turn}` | Apaga a pergunta e tudo o que foi enviado para ela |
| `POST /api/lexicon/suggest {key, run, provider, modelfile?, claude_model?}` | Vocabulário da edition, pedido ao LLM (ou só o pedido, em `manual`) e conferência dos grupos |
| `POST /api/lexicon/check {key, run, id, text}` | Confere uma resposta colada para um pedido anterior |
| `POST /api/project/export {key, dry_run}` | Lista (`dry_run` true, padrão) ou grava `librarian.json` e os arquivos dele no projeto |
| `GET /api/modelfiles` · `PUT /api/modelfiles {name, content}` | Lista ou salva `modelfiles/<nome>.Modelfile` (exige `FROM`) |

Erros voltam como `{"error": "..."}` com status 400, 403, 404 ou 501.
