# librarian-ingest — acervo estrutural de fontes

`librarian-ingest` lê fontes locais e produz uma **edition** verificável em arquivos,
sem banco nem LLM. Sobre a edition verificada, oferece o grafo sintático, a busca
lexical e a ponte `prepare` até um `PreparedQuery`; veja [SEARCH](SEARCH.md).
Nada disso gera afirmações semânticas.

| Comando | Resultado |
| --- | --- |
| `generate`, `verify`, `show` | Edition, conferência histórica ou contra o projeto atual, consulta de símbolo |
| `graph` (formato `v1` ou `v2`), `verify-graph` | Grafo derivado dos snapshots; `v2` é o contrato neutro |
| `search` | Ranking lexical e vizinhança do primeiro resultado |
| `prepare`, `verify-prepared` | Fichas estruturais, dossiê e `PreparedQuery` conferidos, refeitos byte a byte |

## Extractors e perfis

A linguagem entra somente por `src/extractors/`. Cada extractor declara as
declarações (`extract`), as ocorrências sintáticas (`observe`), os tipos alvo de
candidatos por nome e os tipos de navegação que a busca não ranqueia. Um **perfil**
versionado do registry reúne a identidade gravada no manifest, as extensões
descobertas, os extractors, os limites, a versão do grafo e os limites da busca.
`generate` usa o perfil atual. A verificação usa o perfil da identidade gravada
na edition e recusa as desconhecidas; editions antigas nunca são reinterpretadas.
Há dois perfis:

| Perfil | Linguagens | Uso |
| --- | --- | --- |
| v1 (`librarian-ingest/0.1.0;syn/3.0.5;rust-structural-v1`) | Rust | Congelado por fixtures e goldens em `tests/fixtures/v1`; ainda lido e reproduzível com `generate_with_profile` |
| v2 (`librarian-ingest/0.2.0;syn/3.0.5;rustpython-parser/0.4.0;rust-structural-v1;python-structural-v1`) | Rust e Python | Atual. O adaptador Rust é o mesmo do v1; a descoberta ignora `.git`, `.venv`, `venv`, `__pycache__`, `site-packages`, `node_modules`, `target` e caches |

`tests/agnostic.rs` impede nomes de linguagem em grafo, busca e `prepare`.
`tests/python.rs` cobre o G5: um projeto misto Rust + Python passa por edition, grafo,
busca e `prepare` sem mudar esses módulos.
`suggested_inputs(root)` lista as pastas e os arquivos do topo que o perfil atual
percorreria. O studio usa isso para projetos sem `Cargo.toml`.

## Executar

Crie `sources-config.json` com a seleção do seu projeto:

```json
{"project":"meu-projeto","inputs":["src","tests","Cargo.toml","Cargo.lock"]}
```

Diretórios incluem `.rs` recursivamente; arquivos explicitamente indicados entram
mesmo sem essa extensão, como fontes auxiliares. Todos os caminhos são relativos
à raiz informada, usam `/` e precisam existir. Entradas sobrepostas são deduplicadas.
Symlinks, escapes da raiz, arquivos especiais e caminhos não UTF-8 são recusados.
Erros de leitura interrompem a geração; erros de análise preservam o snapshot e
produzem diagnósticos. Não há exclusões implícitas dentro dos diretórios indicados.

Na raiz do Librarian, usando um destino novo cujo diretório pai já exista:

```sh
cargo run --locked -p librarian-ingest --bin librarian_ingest -- generate /caminho/projeto sources-config.json /caminho/edicao-nova
cargo run --locked -p librarian-ingest --bin librarian_ingest -- verify /caminho/edicao-nova
cargo run --locked -p librarian-ingest --bin librarian_ingest -- verify /caminho/edicao-nova /caminho/projeto
cargo run --locked -p librarian-ingest --bin librarian_ingest -- show /caminho/edicao-nova nome_da_funcao
```

`generate` e `verify` retornam 0 quando não há diagnósticos; 2 indica diagnósticos
de extração ou divergência na comparação atual. Erros de uso, leitura ou integridade
retornam 1. Uma edição com erro de sintaxe pode ser íntegra e incompleta; inspecione
`diagnostics.json`. `show` procura o nome exato e exibe todos os homônimos.

## Artefatos e contratos

| Arquivo | Conteúdo |
| --- | --- |
| `manifest.json` | Formato/extrator, seleção, metadados Git opcionais, contagens, hashes dos registros e limites. Publicado por último. |
| `sources.jsonl` | Caminho relativo, ID, hash, tamanho em bytes e linguagem de cada fonte. |
| `symbols.jsonl` | Declarações, escopo sintático local, referência ao trecho e presença literal de `#[test]`. |
| `chunks.jsonl` | Fonte, intervalo de bytes e linhas e hash do trecho. |
| `diagnostics.json` | Erros de parsing e de UTF-8 por fonte. |
| `snapshots/<sha256>` | Bytes originais, deduplicados por conteúdo, sem normalizar CRLF/BOM. |

Bytes começam em zero e o fim é exclusivo. Linhas começam em um e o fim é
inclusivo. IDs de fontes combinam hash do caminho e hash do conteúdo; IDs de
símbolos/trechos acrescentam tipo e intervalo. Mudança de arquivo cria nova
identidade: IDs não representam continuidade histórica automática de um símbolo.

O extrator usa `syn` 3.0.5 e spans de `proc-macro2`. Declarações incluem módulos,
structs/enums/unions, traits, aliases, funções, métodos, impls, constantes e
estáticos. Trechos podem se sobrepor (módulo, impl e método). Documentação associada
por atributos entra no span; comentários comuns permanecem no snapshot. O flag
`is_test` observa `#[test]`, sem reconhecer automaticamente frameworks externos.
Não há expansão de macros, avaliação de `cfg`, resolução entre arquivos, imports
catalogados, grafo de chamadas ou inferência de comportamento. Ambas as alternativas
de compilação condicional podem aparecer. Rust inválido não recebe extração parcial.

O extrator Python usa `rustpython-parser` 0.4.0.
- **Declarações:** classes (`class`), funções (`function`, inclusive aninhadas) e métodos
  (`method`). O trecho começa no primeiro decorator. Os nomes qualificados usam ponto
  (`Classe.metodo`).
- **Ocorrências:** chamadas (`call_observed`, `method_call_observed`), anotações e bases
  (`type_reference`, com candidato por nome, inclusive referências em string),
  retornos declarados e atribuições (`local_binding`).
- **Candidatos e navegação:** só classes recebem candidatos, e somente da mesma
  linguagem. A classe é navegação: os métodos é que são as respostas da busca.
- **Testes:** `is_test` observa nomes `test*`/`Test*` (convenção do pytest), não execução.
- **Limites:** não há resolução de imports, despacho dinâmico nem avaliação de
  decorators. Um arquivo que não faz parse vira diagnóstico, sem extração parcial. O
  parser cobre a sintaxe até cerca do Python 3.12; sintaxe mais nova, como `except A, B:`
  (3.14), vira diagnóstico.

O catálogo **não é um dossiê de fatos** e não gera afirmações semânticas. `prepare`
transforma resultados da busca em fichas **estruturais** (onde está cada declaração)
para o Graph Engine de dossiês; essas fichas não respondem à pergunta.

## Integridade, memória e limites

`Catalog::load` verifica o marcador final, hashes dos registros, bytes dos
snapshots, identidades e contagens, e reexecuta a extração para conferir os registros.
Isso exige a versão do extrator registrada; outras versões são recusadas. Carrega
snapshots e índices de fonte/ID/caminho/nome em memória, com custo proporcional ao
acervo. `source_by_path`, `symbols_named`, `chunk` e `excerpt` permitem consultas
exatas. `SearchIndex` acrescenta ranking lexical de perguntas e `Graph` acrescenta
relações sintáticas; veja [SEARCH](SEARCH.md). Não há embeddings ou interpretação por LLM.

`compare_current` informa arquivos alterados/indisponíveis e novos arquivos do
escopo selecionado; erros ao enumerar esse escopo são erros explícitos. Verificação
histórica não depende da raiz original. Não corrige hashes nem reescreve edições.

Os arquivos são lidos individualmente uma vez durante a geração. Não há snapshot
atômico do repositório. HEAD/estado Git são observações opcionais anteriores às
leituras e podem ser nulos se Git estiver indisponível; não autenticam os bytes.
Registros de conteúdo são determinísticos para a mesma entrada e versão do
extrator; metadados Git podem mudar independentemente. Não há timestamp obrigatório.

Falha de escrita pode deixar uma pasta parcial; ausência de `manifest.json` impede
carregamento. Destinos existentes nunca são sobrescritos. Hashes não autenticam
autoria e não impedem substituição coerente de todo o acervo. Preserve os artefatos
como bytes ao versioná-los: por exemplo, configure `caminho/do/acervo/** -text`
em `.gitattributes`, evitando conversão de finais de linha pelo Git.

## Testar

```sh
cargo test --locked -p librarian-ingest
cargo test --workspace --locked
```

Os testes cobrem spans Unicode/CRLF/BOM/shebang, macros/cfg, reprodutibilidade,
destinos existentes, mudanças atuais, adulteração, diagnósticos e caminhos inválidos.
Também cobrem a regressão da edition v1 congelada (`v1_regression.rs`), o contrato
`syntax-graph-v2` (`graph_v2.rs`), a ponte `prepare` (`prepare.rs`) e a fronteira
sem linguagem (`agnostic.rs`).
