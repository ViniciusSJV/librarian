# Q6 repetida (Piloto 1b), 18/11/2026

Raiz: cópia de `.studio/projects/healthchecks-25597e41` (932 arquivos, iguais à linha de base) em `root/`.
O `.studio` original não foi tocado (932 arquivos com hashes iguais depois da Q6).

- Binário: `studio_q6.exe`, cópia de `target/release/librarian_studio.exe` (HEAD `aae51d7`, com P1 e G3).
- Edition nova: run `20261118-205150`. `graph_sha256` `2d3b3a3b…`, igual ao da run original `20261118-061046`;
  `catalog_sha256` `2b414d79…`, 657 fontes, 61.506 nós, 116.928 arestas, 0 diagnósticos, git `6c05a7a` limpo.
- Pergunta: turn `01` da run nova. É o texto de `turns/14/question.json` da original, com as 7 declarações e `with_facts: true`.
- Envio: um só, perfil `qwen3coder` (Ollama `librarian-verificavel-qwen3coder`, Modelfile `verificavel-qwen3coder`
  com sha256 `64bf8c2d6eaa…`, igual ao da original). `temperature 0` e `seed 42` estão no Modelfile, por isso foi 1 envio.
- Pendência conhecida, não corrigida: a query diz que os `reviewed_facts` foram "revisadas e aprovadas por uma pessoa" (PIL-P4).
  O mesmo texto está na query original.

| | Original (T14, envio 03) | Nova (run 205150, T01) |
|---|---|---|
| Fatos entregues | F18–F25 (8) | F2, F4, F5, F6, F18–F26 (13) |
| `claims/query.json` | 46.220 B | 47.777 B (+1.557 B, +3,4%) |
| `prompt_tokens` | 11.667 | 12.119 (+452, +3,9%) |
| Tempo | 31,3 s (modelo já carregado) | 53,6 s (modelo carregado do zero) |
| H1–H3 / H4 | passaram / não se aplica | passaram / não se aplica |
| Conferência mecânica | passed | **rejected**: C3 `location_operation_includes_declaration` |
| Falhas de envio | 2 (queda do `llama-server`) | 0 |

Arquivos:

- `prepare.js`: gera a edition nova e prepara a pergunta.
- `package.js`: compara os pacotes e as conferências.
- `compare.js`: lista as claims das duas respostas.
- `factbytes.js`: tamanho de cada fato no pacote.
- `prepared-response.json`, `evidence-before-send.json` e `send-response.json`.
- O registro completo: `root/.studio/projects/healthchecks-25597e41/runs/20261118-205150/turns/01/`.

Nenhum fato foi promovido. Nenhuma revisão foi gravada.
