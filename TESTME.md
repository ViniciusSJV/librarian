# TESTME — entrada para testar o Librarian

**A trilha é o [Librarian Studio](STUDIO.md).** O renderer é o projeto de referência:

1. Na raiz do Librarian, dê duplo clique em `STUDIO.cmd` (abre <http://127.0.0.1:8765/>).
2. Abra a pasta `..\renderer`. O `librarian.json` dele traz entradas, léxico, instruções e critérios.
3. Clique em **Gerar grafo**: edition, grafo v2 e conferência contra os snapshots.
4. Pergunte `Onde a câmera calcula a direção dos raios?` e confira:
   - chips `camera`, `direcao` e `raios` realçados;
   - E1 = `impl Camera::ray_from_pixel`, aceso no grafo;
   - critérios prévios gravados com hash;
   - query com `{"number": 86, "text": "        let direction = (pixel - origin).normalize();"}`.
5. Com **Nenhum**, veja e copie a query. Com **Ollama** e o Modelfile `analista-qwen3coder`, ou com o
   **Claude Code CLI**, envie e leia a resposta. Avalie contra os critérios; `completed`
   não é aprovação.

Os testes automáticos: `cargo test --workspace --locked`. A trilha pela CLI está no
[WORKFLOW](WORKFLOW.md). Para API e uso independente, consulte
[ingestão](crates/librarian-ingest/README.md), [busca/grafo](crates/librarian-ingest/SEARCH.md)
e [LLM Engine](crates/librarian-llm-engine/README.md). O roteiro manual antigo e o PLAYME
estão no [HISTORICO](HISTORICO.md).

O exemplo [maintenance](examples/maintenance/README.md) permanece disponível como
referência de cadastro manual de fatos da versão 0.1. Não é pré-requisito para
extrair código, fazer perguntas ou usar um projeto.
