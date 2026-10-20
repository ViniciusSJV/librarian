# TESTME — entrada para a trilha única

**A trilha executável é o [PLAYME](tools/playme/README.md) deste repositório**,
configurada por um perfil do consumidor. Fluxo, perfil e artefatos estão no
[WORKFLOW](WORKFLOW.md). O renderer é o consumidor de referência:

```powershell
..\renderer\PLAYME.cmd -Question 'Onde a câmera calcula a direção dos raios?'
```

1. Compilar `librarian_ingest`, `prepare_ollama` e `send_ollama`.
2. Gerar/verificar a edition e o grafo das fontes do perfil.
3. Buscar com o léxico do perfil e preservar ranking, trechos e relações.
4. Reconstruir E1 do snapshot; gravar query, prompt e critérios do perfil.
5. Iniciar Ollama, obter o modelo base e aplicar o Modelfile do perfil.
6. Enviar, preservar o retorno, reconferir hashes e avaliar separadamente.

O [TESTME do renderer](../renderer/TESTME.md) continua como roteiro manual comentado
dos mesmos passos para aquele consumidor; não é uma segunda trilha. Para API e uso
independente, consulte [ingestão](crates/librarian-ingest/README.md),
[busca/grafo](crates/librarian-ingest/SEARCH.md) e [LLM Engine](crates/librarian-llm-engine/README.md).

O exemplo [maintenance](examples/maintenance/README.md) permanece disponível como
referência de cadastro manual de fatos da versão 0.1. Não é pré-requisito para
extrair código, fazer perguntas ou usar um consumidor.
