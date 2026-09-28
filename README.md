# Librarian

O [contrato experimental de afirmações](crates/librarian-llm-engine/CLAIMS.md)
verifica formato, permissões e citações, mantendo sustentação semântica separada.
O [primeiro experimento da câmera](experiments/claims-camera-20260928-01/README.md)
preserva uma nova resposta e sua rejeição mecânica; a Etapa 1 segue aberta.
As [séries com schema e mapa de linhas](experiments/claims-camera-20260928-04/README.md)
acrescentam nove chamadas: seis passam mecanicamente, mas ainda omitem o nome
da função exigido pela rubrica. Não representam aprovação semântica completa.
A [série com localização estruturada](experiments/claims-camera-20260928-05/README.md)
identifica nome e linhas corretos nas três respostas. O novo verificador rejeita
as três pela ausência da citação da declaração. Os metadados de símbolo foram
conferidos manualmente em E1; a integração automática com o ingest segue pendente.

O Librarian organiza fontes e prepara evidências rastreáveis. A implementação
atual extrai código Rust em snapshots/símbolos/trechos, confere hashes e oferece
busca lexical com relações sintáticas em memória, sem banco. O novo
[LLM Engine](crates/librarian-llm-engine/README.md) prepara e preserva chamadas
Ollama, com modelo explícito e avaliação semântica separada.

## Comece por uma única trilha

Siga o **[TESTME do renderer](../renderer/TESTME.md)**: criar acervo → pergunta →
léxico → ranking → grafo → inspecionar seleção → prompt → servidor/Modelfile →
API Ollama → avaliação. O [TESTME local](TESTME.md) é apenas uma entrada para o
mesmo documento, não uma segunda sequência de comandos. Os links entre projetos
pressupõem checkouts locais lado a lado.

O roteiro fica no renderer porque ele fornece o projeto estudado, o vocabulário,
o Modelfile e a política de seleção. Ingestão, busca e transporte pertencem ao Librarian e
pode ser usado em outros projetos Rust. O [WORKFLOW](WORKFLOW.md) mostra os papéis
dos dois projetos e do Ollama no mesmo fluxo.

Uma tentativa real já teve transporte/hashes conferidos e explicação parcialmente
correta; avaliação semântica continua separada. Busca → envelope manual → API foi
exercitado; a ponte automática até PreparedQuery permanece pendente. A biblioteca
LLM foi extraída, sem fechar o contrato semântico. As capacidades novas usam o
checkout local e não estão na release 0.1.0.

Referências técnicas, para consultar quando necessário: [ingestão](crates/librarian-ingest/README.md),
[léxico e grafo](crates/librarian-ingest/SEARCH.md) e [contrato formal de dossiês](crates/librarian-graph-engine/WORKFLOW-V0.1.md).
O restante deste README descreve os princípios e a API de fatos/dossiês.
## Motivação

A inspiração vem do Bibliotecário de *Snow Crash*, de Neal Stephenson. A ideia que
motiva o projeto é construir um assistente que ajude a encontrar e relacionar
informações sem perder o caminho até suas fontes.

Para isso, o Librarian separa o acervo, a conferência das evidências e a explicação.
Uma resposta convincente não deve substituir uma verificação, e uma informação
não deve perder sua origem ao ser usada em uma conversa.

## Princípio dos engines

> **Graph Engine → testa sem explicar.**
>
> **LLM Engine → explica sem interpretar.**

O **Graph Engine** aplica verificações determinísticas: confere referências,
linhas, hashes e registros de captura, seleciona evidências e prepara a consulta.
“Testar” aqui significa verificar esses vínculos e critérios explícitos; não
significa provar automaticamente que uma afirmação é verdadeira.

O **LLM Engine** tem o papel de apresentar as evidências em linguagem natural.
“Explicar sem interpretar” é a diretriz de não acrescentar conclusões próprias,
preencher lacunas ou transformar hipóteses em fatos. A explicação deve citar suas
fontes e conservar os limites do material recebido. Essa diretriz precisa ser
avaliada nas respostas; não é uma garantia automática de um modelo de linguagem.

O acervo pertence ao **Librarian**. A conferência pertence ao **Graph Engine**.
A explicação cabe ao **LLM Engine**, e sua avaliação permanece uma etapa separada.

```text
Fontes → Librarian → Graph Engine → LLM Engine → avaliação
         organiza    testa          explica
```

## O que existe hoje

- **`librarian-core`**: contratos de fatos, fontes, seleção e dossiês.
- **`librarian-graph-engine`**: conferência de evidências, seleção ordenada,
  contexto por linhas e exportação de consultas com registros de origem.
- **`maintenance-example`**: exemplo com um manual de manutenção fictício,
  mostrando o fluxo de cadastro, conferência e exportação.

O `librarian-llm-engine` implementa o preparador e transporte Ollama como biblioteca
e CLIs. Não seleciona fontes nem valida semanticamente a resposta. O fluxo formal
de dossiês continua terminando em PreparedQuery; a aplicação coordena o envio.

## Como as informações são organizadas

Uma **fonte** contém o texto e seus metadados. Um **fato catalogado** registra uma
afirmação com referência à fonte e à linha correspondente. Um **dossiê** reúne
fontes, fatos e lacunas declaradas. Uma **seleção** indica quais fatos serão usados
na consulta e quanto contexto será incluído.

A exportação gera um **bundle** com o dossiê, a pergunta, a consulta e um registro
de origem com hashes. Isso permite conferir a correspondência entre os arquivos
usados e o material preparado para explicação.

## Experimentar

Na raiz do projeto:

```sh
cargo test --workspace --locked
cargo run --locked -p maintenance-example -- examples/maintenance/manual.txt ./maintenance-bundle
```

O diretório de destino deve ser novo. O exemplo usa dados fictícios e não exige
banco de dados ou servidor LLM.

Veja o [roteiro de consulta de uma fonte](examples/maintenance/README.md) para
entender o cadastro, inspecionar o bundle e selecionar novamente um único fato.

## Usar a biblioteca

```rust,ignore
let mut checks = librarian_graph_engine::filesystem::SourceChecks::default();
let prepared = librarian_graph_engine::prepare_query(
    &dossier_json, Some(&question), &["F1"], 2,
    |source| checks.validate(source),
)?;
prepared.write_bundle(destination, "dossier.json", "question.txt")?;
```

`prepare_query` confere todas as fontes e referências do dossiê, inclusive as que
não foram selecionadas. O callback define a verificação do armazenamento;
`SourceChecks` oferece a conferência de arquivos e capturas locais. Caminhos
relativos são resolvidos a partir do diretório atual.

`write_bundle` preserva os bytes preparados em um diretório novo. A publicação de
`origin.json` conclui a escrita; uma falha pode deixar um diretório parcial.
Fontes não são reconferidas nessa escrita nem em um envio posterior: a aplicação
precisa solicitar uma nova conferência quando necessário.

## Limites

Os [critérios da versão 0.1 e a auditoria da API](V0.1.md) delimitam o fechamento
determinístico. Para exportar um dossiê de outro projeto, execute a partir da
base dos caminhos das fontes (o destino deve ser novo):

```sh
cargo run --manifest-path /caminho/Librarian/Cargo.toml --locked -p librarian-graph-engine --example export -- dossier.json question.txt bundle-novo 2 F1 F2
```

- Uma referência válida não comprova a verdade da afirmação.
- Hashes conferem bytes; não autenticam autoria ou execução.
- Os fatos são fornecidos pela aplicação; não há extração automática de evidências.
- IDs são locais ao dossiê; ainda não existe um catálogo persistente de versões.
- O Graph Engine não inclui um banco de grafos nesta versão.
- Respostas de um LLM não são promovidas automaticamente a fatos do acervo.
