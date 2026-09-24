# Busca lexical e grafo sintático

Esta capacidade é reutilizável em consumidores do Librarian. Usa `Catalog::load`
para conferir uma edição existente; não altera o formato 1 nem seus snapshots.
Não depende de banco, LLM ou renderer. O parser de relações continua específico a
Rust. O vocabulário é fornecido pela aplicação.

## API e comandos

`search::tokens` normaliza acentos portugueses e separa snake_case/camelCase/siglas.
`SearchIndex::new(&catalog, lexicon)` carrega conjuntos lexicais em memória.
`search(question, top, depth)` devolve candidatos, motivos, trechos e vizinhança do
primeiro resultado. `Graph::build(&catalog)` deriva o grafo completo dos snapshots;
`graph.verify(&catalog)` rederiva e compara todos os registros.

Na raiz do Librarian, após gerar `sources-demo` com o roteiro de ingestão:

```sh
cargo run --locked -p librarian-ingest --bin librarian_ingest -- search ./sources-demo "prepare query" - 5 1
cargo run --locked -p librarian-ingest --bin librarian_ingest -- graph ./sources-demo ./source-graph-new.json
cargo run --locked -p librarian-ingest --bin librarian_ingest -- verify-graph ./sources-demo ./source-graph-new.json
```

O arquivo de grafo precisa ser novo. Para vocabulário próprio, substitua `-` por
um JSON como:

```json
{"groups":[["raio","raios","ray","rays"],["direcao","direction"]],"stop_words":["calcula"]}
```

Os grupos equivalentes não podem se sobrepor; cada entrada deve normalizar para
um token. Flexões/plurais são explícitos, não inferidos por lematização geral.
Sem vocabulário adicional, aplica apenas normalização e palavras de ligação.

Exemplo com o primeiro consumidor, ainda a partir do Librarian:

```sh
cargo run --locked -p librarian-ingest --bin librarian_ingest -- search ../renderer/ai/acervo/renderer-auto-v1/edition "Onde a câmera calcula a direção dos raios?" ../renderer/ai/acervo/renderer-lexicon.json 5 2
```

Esse comando deve recuperar `impl Camera::ray_from_pixel` primeiro. Os caminhos
são argumentos; o crate não conhece esse domínio.

## Ranking e limites

Busca nome, escopo sintático, caminho e corpo do símbolo. Cada grupo de termos
conta uma vez; cobertura mínima de 60%. Score = 100 por grupo encontrado + maior
peso por grupo (nome 8, escopo 3, caminho/código 1), menos 20 para `#[test]`.
Desempate: não testes, menor trecho, ID. `reasons` registra campos e expansões.
Não é probabilidade ou prova de sustentação semântica.

Módulos/impls ficam no grafo, mas não competem na busca. Resultados sobrepostos
no mesmo arquivo são deduplicados. A pergunta deve ser não vazia, até 8192 bytes;
`top` aceita 1..10. Trechos têm até 4000 bytes UTF-8 cada e 16000 no total; cortes
são explícitos. Hashes e intervalos se referem ao trecho completo, não ao prefixo.

A navegação preserva direções das arestas, mas percorre ambos os sentidos. Profundidade
0..3; no relatório de busca, até 48 nós e 96 arestas. `truncated` indica corte por
quantidade; a profundidade é registrada. Arquivos não são expandidos. Rótulos na
vizinhança têm até 256 bytes UTF-8; `labels_truncated` contabiliza os cortes.
O grafo completo mantém rótulos integrais.

Saída 0 indica candidatos sem diagnósticos; 2, nenhum candidato ou diagnósticos;
1, falha de entrada/I/O/integridade. A consulta vazia de termos úteis também retorna
`no_lexical_evidence`. Isso pode ser falha lexical: não prova inexistência no código.

## Relações e procedência

`declares` liga fonte a símbolo; `contains` liga intervalos estruturais aninhados.
`type_reference` e `declared_return` registram tipos escritos. `local_binding`
registra o padrão de uma variável local. `call_observed` e `method_call_observed`
registram expressões, sem resolver destino/receptor ou comprovar execução.

Referências a tipos ganham ligações `name_candidate` para declarações homônimas,
inclusive múltiplas e entre arquivos. Elas são candidatas lexicais, não resolução
de imports, aliases, escopos ou genéricos. Não são grafo de chamadas resolvidas.
Cada aresta guarda ID/caminho/hash da fonte, intervalo original e hash do trecho.
Spans respeitam Unicode, CRLF, BOM e shebang. Macro/cfg não são expandidos/avaliados.

O JSON do grafo identifica a versão do algoritmo e o hash da serialização canônica
do manifesto desserializado; esse hash não é necessariamente o hash dos bytes
formatados de `manifest.json`. O hash do vocabulário na busca segue a mesma regra.
`verify-graph` compara com uma nova derivação integral. Não autentica autoria.

Grafos e buscas são derivados de snapshots históricos, mesmo que a árvore atual
tenha mudado. A verificação atual é uma operação separada. Resposta JSON não é
`PreparedQuery`: converter resultados em dossiê/bundle ainda é uma próxima etapa.

## Verificação

```sh
cargo test --workspace --locked
```

Testes do crate incluem normalização/aliases, ausência de evidência, ranking,
orçamentos, Unicode, proveniência das arestas, candidatos ambíguos, chamadas não
resolvidas e rejeição de grafo adulterado. O consumidor renderer mantém seis
perguntas de desenvolvimento e um teste para os resultados esperados. Esse conjunto
não mede generalização independente nem qualidade de resposta de um LLM.
