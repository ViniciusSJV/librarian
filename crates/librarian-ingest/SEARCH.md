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

Exemplo com o primeiro consumidor, a partir do Librarian e com destino novo:

```sh
cargo run --locked -p librarian-ingest --bin librarian_ingest -- generate ../renderer ../renderer/ai/acervo/renderer-sources.json ./renderer-edition-new
cargo run --locked -p librarian-ingest --bin librarian_ingest -- search ./renderer-edition-new "Onde a câmera calcula a direção dos raios?" ../renderer/ai/acervo/renderer-lexicon.json 5 2
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

No perfil v3 (G6), campos também são símbolos (`field`: struct nomeada em Rust;
corpo da classe e `self.<nome>` no `__init__` em Python), e há duas ocorrências novas:
`attribute_access` (`a.b`, leitura ou escrita) e `base_class` (base escrita numa classe
Python). Acessos, chamadas e bases ganham `name_candidate` para campos, funções/métodos e
classes de mesmo nome. Acima de 8 declarações candidatas, nenhuma ligação é desenhada e o
nó da ocorrência guarda `ambiguous_candidates` com a contagem. Os perfis v1 e v2 continuam
lidos e reproduzíveis sem nada disso.

No perfil v4 (G6d), o adaptador de Rust registra também `struct_literal`
(`Camera { .. }`, `Self { .. }`), `field_init` (cada campo nomeado que o literal preenche),
`field_assign` (`x.campo = …` e `x.campo += …`, que deixam de ser `attribute_access`) e
`impl_self_type` (o tipo escrito no `impl`, que deixa de ser `type_reference`). Literais e
`impl` são candidatos a struct, enum, union e type; campos preenchidos e escritos, a
campos. O Python continua com o adaptador do v3. No passo de cadeia, o `prepare` usa essas
ocorrências para o contexto do dono (`owner_context`): o tipo dono de um método e quem
escreve os campos que ele lê, registrados em `owner_context` no relatório `chain-step-v1`.

Referências a tipos ganham ligações `name_candidate` para declarações homônimas,
inclusive múltiplas e entre arquivos. Elas são candidatas lexicais, não resolução
de imports, aliases, escopos ou genéricos. Não são grafo de chamadas resolvidas.
Cada aresta guarda ID/caminho/hash da fonte, intervalo original e hash do trecho.
Spans respeitam Unicode, CRLF, BOM e shebang. Macro/cfg não são expandidos/avaliados.

O JSON do grafo identifica a versão do algoritmo e o hash da serialização canônica
do manifesto desserializado; esse hash não é necessariamente o hash dos bytes
formatados de `manifest.json`. O hash do vocabulário na busca segue a mesma regra.
`verify-graph` compara com uma nova derivação integral. Não autentica autoria.

### Contrato neutro `syntax-graph-v2`

`graph <edition> <novo.json> v2` exporta o mesmo grafo em formato neutro,
derivado de qualquer edition verificada, inclusive v1; o padrão continua `v1`.
`verify-graph` escolhe o contrato pelo campo `version` do arquivo.

- Nó: `id`, `category` (`source`, `symbol` ou `occurrence`), `kind` (vocabulário
  do extractor; `file` para fontes), `language` (classificação da edition) e `label`.
  Opcional: `ambiguous_candidates` (perfil v3), ausente quando não se aplica; `type_rule`
  (perfil v9, passo 2.3a), só num nó `receiver_type`: a regra que deu o tipo escrito.
  Chunks continuam registros de inspeção, não nós.
- Aresta: `id`, `from`, `to`, `kind`, `resolution` e `evidence`. Os tipos de
  relação são só `declares`, `contains`, `observes` e `name_candidate`; o tipo da
  ocorrência fica no nó. `id` = `edge:` + SHA-256 da serialização de
  `(from, to, kind, resolution, evidence)`.
- Topo: `version`, `derived_from` (versão do grafo do perfil), `profile`
  (identidade do extractor da edition), `catalog_sha256`, `nodes` e `edges`.

É uma projeção: mesmos nós e uma aresta v2 para cada aresta v1, sem relações novas.
Campos desconhecidos são recusados. Candidatos por nome só ligam a mesma linguagem.

### Onde é usado (`used_by`)

`v2::Graph::used_by(seed, depth, max_nodes)` parte de um símbolo e volta pelos nomes
escritos: cada `name_candidate` que chega nele leva à ocorrência e, pelo `observes`, ao
símbolo que a escreveu; repete por camadas até `depth` (1–6), com teto de nós (1–300) e
`truncated` explícito. Cada uso traz o texto escrito, a evidência, o ID da aresta e
`candidates`, o número de declarações que aquele nome combina (acima de 1, é um entre
homônimos). Não resolve nada: é o mapa dos candidatos pelo nome.

### O que chama (`calls_from`, `POST /api/calls`)

`v2::Graph::calls_from(seed, depth, max_nodes, syntax, chosen)` segue para a frente as
chamadas escritas, por camadas, com os mesmos limites de `used_by`. A regra que decide cada
chamada é uma só, em `src/resolve.rs` (passo 2.1; forma da chamada e `import` no 2.2). O inventário também a usa, e o envelope e
o contexto do dono aceitam o que ela decide com um alvo só (`resolve::SETTLED`). Ela é uma
camada sobre o grafo: lê nós e arestas e não cria aresta. As arestas continuam candidatos
por nome, nunca resolução de compilador.

**Forma da chamada (passo 2.2).** Antes da precedência, a regra só admite os candidatos que a
forma escrita alcança. As arestas do grafo não mudam; a regra é que deixa de considerar algumas.

| Forma | Candidatos admitidos |
|---|---|
| `x.m()`, `self.m()` | Todos menos a função solta |
| `x.m()` com `x` ligado a um módulo do projeto (`import m`, `import m as a`, `from p import m`, `import a.b` com receptor `a.b`). Python | Só as funções e, desde o perfil v9 (2.3a), as classes do arquivo desse módulo (`forms.WebhookForm(..)`) |
| `x.m()` com `x` ligado a um import de fora do projeto (`import time`, `from django.contrib import auth`). Python | Nenhum |
| `f()`, sem receptor | Todos menos os métodos. Exceção: no corpo de uma classe, ou num método dela, os métodos da própria classe continuam |

- **Tabela de imports.** Vem dos escopos do arquivo (módulo, classe, função, `lambda` e comprehension).
  - Decide o escopo mais interno que liga o primeiro nome do receptor.
  - Nesse escopo, uma ligação local de mesmo nome (parâmetro, atribuição, alvo de `for`, `with`, `except`, comprehension ou `match`, definição aninhada) vence o import.
  - O corpo de uma classe não é visto pelas funções dentro dela.
- **Onde se acha o módulo.** Entre os caminhos da própria edition: `a.b` é `a/b.py` ou `a/b/__init__.py`, inclusive relativo ao arquivo (`from . import m`). Sem esses, vale o único caminho que termina assim (um pacote em `src/`).
  - `from p import Nome`, quando `Nome` não é arquivo do projeto mas `p` é: um nome do projeto (classe ou função), e a regra comum vale.
  - `mod.Classe.m()` também segue a regra comum.
- **Leitura sem gravação.** Os imports, as ligações locais e o tipo escrito do receptor que não é do projeto são lidos dos snapshots da edition, pelo extrator da linguagem, no arquivo de quem chama e só quando a regra precisa (`CallSyntax::facts`, `Extractor::call_facts`). Nada disso é gravado no grafo, e o perfil não muda.

Precedência, sempre sobre os admitidos: `chosen` > `unique` > `import` > `receiver` > `type` >
`trait_default` > tipo derruba > `trait` > `homonym`.

| `resolution` | Quando | `basis` |
|---|---|---|
| `chosen` | A pessoa escolheu este entre os candidatos admitidos | `graph_edge` |
| `unique` | Um só admitido, e ele era o único candidato do grafo, ou a chamada não tem receptor, ou o receptor é módulo do projeto | `graph_edge` |
| `import` | `x.m()` com `x` ligado a um módulo do projeto: havia vários candidatos, e só uma função daquele nome no arquivo do módulo. Python | `import` |
| `receiver` | A chamada é `self.m()` (o receptor da linguagem e nada mais antes do nome) e só este homônimo está no bloco de quem chama. Acima do teto, pelas declarações desse bloco, em Rust e, desde o passo 2.3a-0, em Python (até então o método do Python era procurado pelo rótulo inteiro, `Classe.metodo`). `self.x.m()` nunca é `receiver` | `graph_edge` |
| `type` | Método do tipo escrito do receptor: `receiver_type` do extrator ou caminho `T::f`. No Rust, também a variante de `T::V(..)` (2.3a). No Python, desde o perfil v9 (2.3a), o método que a própria classe do receptor declara | `type` |
| `trait_default` | O tipo escrito não define o método; é o padrão de um trait que ele implementa | `type` |
| `homonym` | Nada decide; cada admitido aparece, e nenhum é seguido. Inclui um só admitido que sobrou de vários num `x.m()` | `graph_edge` |

**O tipo derruba um `unique`.** Quando o receptor tem tipo escrito e o único candidato admitido
não serve para ele, a chamada fica sem candidato.
- **Tipo do projeto:** derruba um método que não é desse tipo, nem de um trait que ele implementa, nem do trait que ele é.
- **Tipo de fora do projeto** (`Vec`, `str`): derruba só um método inerente de um tipo do projeto (`impl Groups::len` para `v.len()` com `v: Vec<_>`). Não derruba quando:
  - o receptor está escrito como referência (`&T`, `&mut T`) ou como `Box`, `Rc` ou `Arc`;
  - o projeto implementa um trait para aquele tipo (`impl Equivalence<f64> for f64`).
- Um `unique` que o tipo confirma mantém o rótulo.

`basis: "type"` quer dizer que o tipo foi lido do que está escrito, pelas regras sintáticas
do extrator, e não inferido como um compilador faria. As regras e os limites estão no
`receiver_type` de `src/extractors/mod.rs`:
- parâmetro ou `let` anotado;
- `self` no impl;
- `T::f(..)` e `T { .. }`;
- campo;
- item de uma coleção declarada como campo, inclusive em `for (i, x) in
  self.v.iter().enumerate()`;
- padrão de variante.

Os nomes `Vec`, `iter` e `enumerate` são lidos sem resolver os caminhos. Uma nova
declaração do mesmo nome vale para a função inteira, porque blocos não são escopos. O grafo
não guarda o nome de um tipo que não é do projeto. Desde o perfil v9 (2.3a), cada `receiver_type`
guarda a regra que deu o tipo (`type_rule`): `parametro`, `anotacao`, `self`, `construtor`
(`T { .. }`), `caminho` (`T::f(..)`: o tipo nomeado no caminho, sem ler o retorno de `f`, que é
o 2.3c), `retorno` (`f(..)` de função do mesmo arquivo), `campo`, `item` e `variante`.

**Tipos em Python (passo 2.3a, perfil v9; DEC-39).** O extrator grava um `receiver_type` quando o
tipo do receptor está escrito no arquivo, com a regra:
- `parametro` e `anotacao`: `T`, `m.T`, `"T"`, `T | None`, `Optional[T]`, `Union[T, None]`,
  `Annotated[T, ..]`. A anotação vale para a função inteira: uma atribuição sem anotação não a troca.
- `construtor`: `x = T(..)`, `x = m.T(..)` e `T(..).m()`. Sem anotação, todas as atribuições de `x`
  na função precisam concordar; um `None` não conta; um `for`, `with`, `except`, desempacotamento,
  `global` ou `case` tira o tipo.
- `self`: o primeiro parâmetro de um método (não `staticmethod` nem `classmethod`), também numa
  função aninhada.
- `campo`: `self.x` (ou `a.x`, com `a` de uma classe do mesmo arquivo), com `self.x = T(..)` ou
  `self.x = p` (`p: T`) no `__init__`, `self.x: T` num método ou `x: T` no corpo da classe. Uma
  escrita em outro método, salvo `None`, tira o tipo. `x = T()` no corpo da classe não conta
  (descritor do Django).

O candidato do `receiver_type` é o nome escrito; qual classe ele nomeia é lido na consulta: a classe
de topo do próprio arquivo, a de um `from p import Nome` (seguindo até três reexportações) ou a
classe `T` do módulo do projeto `m`. Um `as` não vale (N8, 2.4). Um nome que não é classe do
projeto (de fora, embutido, `Any`) não dá tipo. Acima do teto de homônimos, a classe é procurada
pelo nome escrito. Com a classe, a regra:
- `type`: os métodos daquele nome que a própria classe declara;
- sem nenhum, a classe derruba os candidatos, salvo se tiver base (herança é do 2.4), definir
  `__getattr__` ou `__getattribute__`, ou for um `Protocol`;
- um `Protocol` não dá link único: o método dele é só a assinatura, e o corpo que roda é o do
  objeto passado;
- um tipo de fora do projeto não derruba nada em Python.

**Instanciação (passo 2.3a; DEC-39).** Uma chamada que a regra liga a uma classe (Python), a uma
struct de tupla ou a uma variante (Rust) instancia; não chama. O link sai com `relation:
instantiates` e a mesma `resolution`. O percurso não entra na declaração construída: segue para
o `__init__` que a classe declara (`runs`) e, sem ele, para ali. O inventário segue igual.

**Decorador e campo (passo 2.3b; DEC-40).** Nada muda no grafo nem no perfil: quem observa uma
chamada continua sendo onde ela está escrita. Muda o que a regra diz de quem a roda.
- **Decorador** (Python). Uma chamada escrita num decorador, em qualquer nível (`@cors("GET")`,
  `@patch(.., side_effect=Erro(..))`), roda quando o `def` ou a `class` roda, não quando a
  declaração decorada é chamada. O link sai com `relation: decorates` e a mesma `resolution`, na
  declaração decorada, e o percurso não o segue, nem para um `__init__`; o inventário também não.
  Os intervalos dos decoradores são lidos do arquivo na consulta (`CallFacts::decorators`), como os
  imports do 2.2, e valem também para os runs antigos.
- **Campo do `__init__`**. O valor de `self.x = T()` num método é observado pelo campo
  `Classe.x`, que o método contém (`contains`). O `/api/calls` do método responde também por essas
  chamadas, com `from` = o método e `via` = o campo; a resposta é a mesma que o campo dá, porque a
  regra decide pelo símbolo que observa. O campo continua respondendo pelas suas, sem `via`.
- **Contagem única.** Numa resposta, cada ocorrência aparece uma vez. Quem conta chamadas por dono
  conta pelo `observes` do grafo (inventário, população do portão) ou ignora a ocorrência já contada
  (`called` do contexto do dono e do envelope). O `used_by` não muda: mostra o campo, com o
  `container` (o método).
- **Consumidores.** O `called` do contexto do dono guarda a `relation` quando ela não é `calls`, e
  o `prepare` escreve "Instanciada por" ou "Decorador de" no lugar de "Chamada por". O envelope
  dá a um decorador o motivo `decorator`, sem tipo devolvido nem tipo dono.

`basis: "import"` quer dizer que o alvo veio da tabela de imports do arquivo, lida do texto
com o parser do extrator, sem executar nada.

Um resultado com vários alvos (`trait`, em que todos os candidatos são de um trait, ou um
tipo que nomeia vários impls, como `Matrix<3>` e `Matrix<4>`, porque os genéricos não são
comparados) aparece como `homonym` sobre os candidatos admitidos, sem ser seguido. O
inventário segue todos eles.

Cada link traz `from`, `to`, `occurrence`, `occurrence_kind`, `text`, `candidates`,
`resolution`, `candidate_edge`, `basis`, `relation` (2.3a: `calls` ou `instantiates`; 2.3b: `decorates`), `runs`
(só numa instanciação com `__init__` próprio), `via` (2.3b: o campo, quando o link vem do valor de um
campo que o método declara; também nas entradas de `unresolved`) e `evidence`.
- `candidates` conta os candidatos admitidos pela forma da chamada, não todas as declarações
  com o nome. Desde o 2.2, num link `receiver` (`self.m()`), a função solta homônima não é
  admitida e sai da contagem: o link e o alvo são os mesmos, mas `candidates` pode ser menor
  que antes (163 links no Healthchecks).
- `candidate_edge` é o ID da aresta `name_candidate`. É `null` só num link `type`,
  `trait_default` ou `receiver` de uma chamada acima do teto de homônimos, para a qual o grafo
  guardou só a contagem. `candidates`, nesse caso, é a contagem registrada
  (`ambiguous_candidates`).
- Os links que já existiam antes do 2.1 têm os mesmos campos e valores, mais `basis`, salvo o
  `candidates` dos links `receiver` descrito acima.

`unresolved` lista o que não virou link:
- `above_candidate_limit`, sem tipo que decida;
- `no_candidate_in_project`: sem candidato no grafo, ou sem candidato admitido pela forma ou pelo tipo, com `candidates: 0`. A chamada sai sem link, nunca com um link para outro símbolo;
- `node_limit`.

Acima do teto, um receptor ligado a import de fora do projeto continua `above_candidate_limit`:
não há aresta para tirar.

Não registrados no v3, e portanto ausentes de `used_by`: corpos de macro, literais de
struct e padrões em Rust; constantes, `static` e variantes de enum como alvo; argumentos
nomeados e `getattr` em Python; nomes acima do teto de homônimos. Ausência não prova
que não há uso.

## Da busca ao PreparedQuery

`prepare <edition> <request.json> <novo-dir>` liga a busca ao Graph Engine de dossiês.
O pedido é autocontido e recusa campos desconhecidos:

```json
{"question": "...", "lexicon": {"groups": [], "stop_words": []}, "top": 5, "depth": 2,
 "hits": 1, "radius": 0, "numbered_lines": false, "instructions": ["..."],
 "accept_diagnostics": false}
```

1. Refaz a busca na edition verificada; recusa edition com diagnósticos, salvo
   `accept_diagnostics`, e menos candidatos que `hits`.
2. Cada um dos `hits` primeiros resultados vira uma ficha **estrutural** `E<n>`:
   "`<kind>` `<escopo>` declarado em `<path>`, linhas a–b", com `line`/`end_line`
   e autoria do perfil do extractor. Ela diz onde está a declaração; não responde
   à pergunta nem afirma comportamento.
3. As fontes do dossiê são os snapshots (`content: code`, `kind` = linguagem),
   conferidos contra a edition por hash e linhas antes da exportação.
4. `prepare_query_formatted` exporta seleção e query com as `instructions` do
   consumidor (ou as padrão do Graph Engine). Com `numbered_lines: true`, as linhas
   do contexto saem como `{"number", "text"}`, em que `number` é a linha original
   do arquivo, acompanhadas de uma nota `numbering`. Com `false` (padrão), o formato
   anterior é preservado byte a byte.
5. Grava `request.json` (bytes recebidos), `search.json` e `bundle/` (`dossier.json`,
   `question.txt`, `query.json`, `origin.json`). O ID do dossiê é `search:<sha256>`.

`verify-prepared <edition> <dir>` refaz tudo a partir de `request.json` e recusa
qualquer byte diferente em busca, dossiê, pergunta e query, além de conferir os
hashes do `origin.json`. O [Librarian Studio](../../STUDIO.md) executa essa
conferência imediatamente antes de cada envio. Não autentica autoria nem execução; `completed` continua sendo só transporte.

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
