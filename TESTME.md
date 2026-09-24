# TESTME — entrada para a trilha única

**O passo a passo completo está no [TESTME do renderer](../renderer/TESTME.md).**
Mantenha os checkouts `librarian` e `renderer` lado a lado. O link é local e exige
o consumidor; não aponta para um arquivo deste repositório quando visto isoladamente.

A trilha chama o binário `librarian_ingest` diretamente, explica cada passagem e
termina no Ollama. Ela fica no renderer porque o cliente HTTP e o Modelfile ainda
pertencem ao consumidor. Não há uma segunda sequência de comandos neste arquivo.

1. Preparar ferramentas e criar a pasta da consulta.
2. Gerar/verificar o acervo e o grafo com Librarian.
3. Criar a pergunta, passar pelo léxico e examinar ranking/trechos/relações.
4. Escolher evidências e preparar query/prompt e critérios.
5. Iniciar Ollama, obter Qwen e aplicar/conferir o Modelfile.
6. Enviar pela API, preservar o retorno e avaliar separadamente.

O Librarian implementa somente ingestão, conferência, busca e grafo nesta trilha;
é reutilizável em outros projetos Rust. Para sua API e uso independente, consulte
[ingestão](crates/librarian-ingest/README.md), [busca/grafo](crates/librarian-ingest/SEARCH.md)
e [WORKFLOW](WORKFLOW.md). São referências técnicas, não etapas extras da trilha.

O exemplo [maintenance](examples/maintenance/README.md) permanece disponível como
referência de cadastro manual de fatos da versão 0.1. Não é pré-requisito para
extrair código, fazer perguntas ou usar o consumidor renderer.