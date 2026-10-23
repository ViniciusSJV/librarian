# Afirmações vinculadas a evidências — contrato experimental 1

Esta API é opcional: o transporte legado e suas queries não mudaram. Não existe
aprovação semântica automática. `claims::check` pode aprovar a estrutura e as
citações de uma frase falsa; sempre devolve `accepted=false`, `semantic_status=pending`.

## Responsabilidades

O consumidor seleciona e confere as fontes antes de construir `Context`. O contexto
contém `schema_version=1`, permissões explícitas e evidências com ID, caminho,
intervalo inclusivo de linhas (base 1) e trecho. O módulo valida identidade e
coerência dos intervalos, sem ler caminhos nem conferir proveniência externa.
Hashes, seleção e origem ficam em um registro separado do consumidor.

As permissões são escolhidas antes da geração; o modelo não as concede a si mesmo.
Ambas devem ser false por padrão no consumidor. A API não interpreta a pergunta
para detectar autorização: o consumidor precisa habilitá-las apenas quando houver
pedido explícito. `Policy::default()` desabilita ambas.

`claims::query` produz uma query compatível com o transporte existente. Modelo e
provedor não fazem parte do contrato de afirmações. As instruções são em português
nesta primeira versão, sem regras específicas de renderer ou modelo embutidas no checker.

Desde a tentativa 02, a query também declara `response_schema`: um JSON Schema
de objeto gerado das categorias autorizadas. O adaptador Ollama o envia no campo
`format`, conforme a [API oficial](https://docs.ollama.com/api/generate) e a
[documentação de saídas estruturadas](https://docs.ollama.com/capabilities/structured-outputs).
O schema permanece no prompt preservado. Queries legadas sem esse campo mantêm
o corpo HTTP anterior; campo presente mas incompatível é recusado, sem fallback
silencioso. O adaptador só confere que é objeto com type=object, não implementa
um validador geral de JSON Schema. Erros do provedor continuam registrados.

O schema restringe formato e categorias, não sustenta afirmações nem garante
citações corretas. `claims check` continua obrigatório e independente, sem
remover cercas, aparar indentação ou alterar texto. Permissões condicionais,
referências e igualdade dos trechos também continuam verificadas localmente.

`evidence.line_map` oferece os números originais e textos literais, derivados de
cada trecho selecionado (sem incluir arquivos externos). Mantém o excerpt original
e evita exigir que o modelo conte linhas. O checker continua usando Context, não
confia em mapas ou referências inventadas na resposta.

As [séries 02–04](../../experiments/claims-camera-20260928-04/README.md) registram
nove chamadas reais: schema resolveu Markdown; mapa de linhas permitiu citações
exatas em seis chamadas; a omissão do nome da função ainda impediu aprovação plena.

## Resposta

Somente JSON, sem cercas Markdown ou texto adicional:

```json
{"schema_version":1,"claims":[{"id":"C1","kind":"FACT","text":"Afirmação diretamente sustentada.","citations":[{"evidence_id":"E1","path":"sample.rs","start_line":5,"end_line":5,"quote":"let x = 1;"}],"premises":[],"verification_plan":null}]}
```

Campos desconhecidos e categorias desconhecidas são rejeitados. IDs de afirmação
devem ser únicos e textos não vazios. Não há obrigação de preencher categorias.

- FACT exige ao menos uma citação.
- INFERENCE exige permissão, citação e premissas não vazias.
- HYPOTHESIS exige permissão, citação e plano de verificação não vazio.
- LACUNA pode vir sozinha, sem citação. Sua necessidade/pertinência é avaliação semântica.
- Premissas só são aceitas em INFERENCE; plano só em HYPOTHESIS.
- Cada citação deve resolver E1/caminho/linhas no contexto selecionado.
  `quote` copia linhas completas, com indentação, unidas por LF. O snapshot não é
  normalizado; apenas a comparação das linhas desconsidera CRLF como terminador.

O checker não identifica inferência disfarçada em FACT, não valida premissas ou
planos semanticamente, nem decide completude/contradições/atomicidade das frases.
Não deve ser usado como selo de verdade ou autorização para executar código.

## Perfil opcional de localização

`Context.require_location` é false quando ausente, mantendo a leitura dos
contextos e respostas anteriores. Com true, a query exige o campo `location`
(objeto ou null) no schema. O checker exige objeto em cada FACT; as outras
categorias não podem declarar localização. Uma LACUNA sem localização continua
possível quando a evidência é insuficiente; sua pertinência continua pendente.

O consumidor pode fornecer `symbols` em cada evidência (lista vazia quando
ausente). Exemplo do perfil usado na série 05:

```json
{"name":"ray_from_pixel","declaration_line":75,"name_column":8,"end_line":89}
```

`name_column` conta caracteres Unicode (valores escalares), com base 1, na linha
da declaração do excerpt. O verificador confere o nome literal nessa posição,
identidade única por nome/linha e limites dentro da evidência. **Esses metadados
são fornecidos pelo consumidor**: isso não analisa a sintaxe nem prova que o
intervalo é uma função ou que seu escopo está correto. O consumidor deve obtê-los
de uma fonte conferida. Na série 05 houve inspeção manual do trecho preservado.
Desde a L2, o [Librarian Studio](../../STUDIO.md) deriva os símbolos da edition
conferida (veja "Formato da resposta" no guia).

Cada FACT do perfil inclui, por exemplo:

```json
{"evidence_id":"E1","path":"src/camera.rs","symbol_name":"ray_from_pixel","declaration_line":75,"operation_start_line":86,"operation_end_line":86}
```

O checker resolve evidence_id/caminho/nome/declaração contra o contexto, confere
se a operação está dentro do símbolo e exige citações cobrindo a declaração e
todo o intervalo da operação. Uma citação pode cobrir ambos; cada quote continua
sujeito à igualdade literal. Localizações fornecidas são conferidas mesmo quando
o perfil não é obrigatório. A função `response_schema(policy)` mantém o schema
básico; `query(context, question)` o estende quando o perfil está habilitado.

Não há nome, caminho, sintaxe de linguagem ou linha esperada embutidos no checker.
Os metadados não pré-selecionam a operação. Uma operação corretamente citada
pode não responder à pergunta: `semantic_status=pending` e `accepted=false`
continuam invariáveis. Fora do perfil `strict_text` (abaixo), as frases livres
não são comparadas aos campos estruturados. Este é um perfil aditivo experimental
da versão 1.

A [série 05](../../experiments/claims-camera-20260928-05/README.md) identificou
nome e linhas corretos em três respostas, mas todas foram rejeitadas por falta
da citação da declaração (`location_declaration_not_cited`).

## Campos opcionais do contexto (L2)

Todos são aditivos: ausentes, a query sai idêntica byte a byte à anterior, e os
contextos antigos continuam sendo lidos.

| Campo | Efeito na query | Efeito no checker |
| --- | --- | --- |
| `limits` | `evidence.limits` e uma instrução: são limites da evidência, não fatos sobre o código | nenhum |
| `project_instructions` | campo `project_instructions` e uma instrução: complementam as fixas e, em conflito, valem as fixas e o `response_schema` | nenhum |
| `strict_text` | `strict_text: true` e uma instrução descrevendo a regra | as duas regras abaixo |
| `allow_patch` | `allow_patch: true`, o campo `patch` no schema e uma instrução | as regras do perfil `allow_patch` (abaixo) |
| `reviewed_facts` | `evidence.reviewed_facts` (id, enunciado, autoria, path e linhas) e uma instrução: são afirmações revisadas por uma pessoa numa pergunta anterior, orientam mas não são citáveis | só a validação de identidade (id único, enunciado, linhas); não são `evidence_id` válidos nas citações |

As instruções novas entram depois das fixas, que mantêm a ordem.

### Perfil `strict_text`

O `text` de cada claim só pode afirmar o que as citações do próprio claim mostram:

- **`text_cites_uncited_line`**: um número de linha escrito no texto não está dentro
  de nenhuma citação do claim. São reconhecidos "linha 86", "linhas 84–86 e 88",
  "linhas 84 a 86", "lines 3 and 4" e `arquivo.rs:86`. Horas (`10:30`) e versões
  (`1.2:3`) não contam.
- **`text_quotes_uncited_code`**: um trecho entre crases não aparece nas linhas
  citadas. A comparação usa as linhas da fonte, não o `quote` da resposta, e
  ignora diferenças de espaços. `...` ou `…` dentro do trecho pulam qualquer texto,
  na ordem. O path e o id da evidência citada são aceitos entre crases.

Exemplo real ([fixture](tests/fixtures/camera-studio-sonnet.txt)): o Sonnet citou as
linhas 75 e 84–86, mas escreveu "o raio é construído na linha 88 com
`Ray::new(origin, direction)`". As duas regras recusam a resposta; citar a linha 88
basta para passar.

Limites: uma paráfrase sem número e sem crase ("o raio é construído depois") passa
sem ser vista. A regra também não confere se há uma só afirmação por claim. Ela
reduz o que o texto pode afirmar sem prova; não confere o sentido.

### Perfil `precise_location`

Com `precise_location: true`, a localização precisa apontar a operação, não a função:

- **`location_operation_includes_declaration`**: a operação começa na linha da declaração;
- **`location_operation_is_whole_body`**: a operação cobre todo o corpo, da linha seguinte
  à declaração até a última linha do símbolo.

Exceções: um símbolo de uma linha só (a operação só pode ser a declaração) e um corpo de
uma linha só (a operação só pode ser o corpo). Origem: na pergunta da esfera, o qwen
apontou `local_intersect` inteira (32–50) e passou; com a primeira regra, respondeu
33–50, que a segunda recusa.

Limite: uma faixa quase inteira (por exemplo 34–49) passa. Quanto da função é "a
operação" é questão de sentido; a comparação entre respostas mostra a diferença como
parcial.

### Perfil `allow_patch`

Com `allow_patch: true` no contexto, uma HYPOTHESIS pode trazer `patch`: um diff unificado
da mudança que propõe. A query ganha `allow_patch: true`, o campo `patch` (string ou null)
no `response_schema` e uma instrução. O checker lê o diff só contra a evidência:

- **`patch_not_allowed`**: patch com `allow_patch` desligado;
- **`patch_requires_hypothesis`**: patch num item que não é HYPOTHESIS;
- **`patch_malformed`**: não é um diff unificado (cabeçalhos `---`/`+++` do mesmo
  arquivo, blocos `@@ -a,b +c,d @@`), as contagens não batem com as linhas ou o bloco
  não muda nada;
- **`patch_unknown_path`** e **`patch_outside_evidence`**: o diff mexe num arquivo que
  não está na evidência ou em linhas fora dela;
- **`patch_does_not_apply`**: uma linha de contexto ou removida não é igual à linha da
  evidência naquele número;
- **`patch_changes_uncited_lines`**: uma linha removida não está citada no próprio item
  (numa inserção pura, ao menos uma linha de contexto precisa estar citada).

Linhas `\ No newline at end of file` são ignoradas, e uma linha vazia dentro do bloco
conta como contexto vazio. O checker não aplica, não compila e não executa nada: um diff
que encaixa não prova que o programa ficou correto nem mais rápido. O ganho continua
hipótese, com `verification_plan`. Ausente, a query sai idêntica e o campo `patch` não
existe no schema.

### Cercas de Markdown

`claims::check` continua recusando uma resposta cercada. `claims::extract_fenced`
devolve o corpo de uma resposta que é exatamente uma cerca (` ```json ` ou sem
rótulo), sem nada fora dela. Quem usar a extração precisa registrar que extraiu;
o studio grava `extracted_from_fence` no `check.json`.

## Comparação por conteúdo (L2b)

`claims::compare::compare` recebe as respostas de uma mesma pergunta (uma lista de
claims por resposta, ou `None` para uma resposta sem claims legíveis) e devolve uma
linha por conteúdo distinto, com uma célula por resposta: `equal`, `partial` ou
`absent`, e os ids dos claims por trás de cada célula.

O conteúdo de um claim é o tipo mais, com `location`, a evidência, a função, a linha
da declaração e as linhas da operação; sem `location`, as linhas citadas, sem ordem
nem repetição. O `text` não entra. É parcial a mesma função (ou as mesmas citações)
com linhas que se sobrepõem, ou o mesmo lugar com outro tipo.

A função não confere nada: quem chama decide quais respostas entram, e o studio
inclui as recusadas, marcadas. Concordância é sinal, não prova.

## CLI e preservação

Na raiz do Librarian, com destinos novos:

```powershell
cargo build --locked -p librarian-llm-engine --bins
.\target\debug\claims.exe prepare context.json question.txt query-nova.json
.\target\debug\send_ollama.exe query-nova.json http://127.0.0.1:11434/api/generate MODELO 300000 1048576 ID_NOVO tentativa-nova
.\target\debug\claims.exe check context.json tentativa-nova/response.txt report-novo.json
```

`claims` não usa rede. `prepare` retorna 0 quando prepara; `check` retorna 0 quando
passa mecanicamente, 2 quando rejeita a resposta e 1 para erro de entrada/I/O.
Zero não é aprovação semântica. O relatório liga os bytes do contexto e da resposta
por SHA-256. Destinos existentes são recusados. O consumidor preserva a rubrica
antes do envio e vincula separadamente query, política, configuração e avaliação.

## Regressões e primeiro experimento

Os testes preservam as duas respostas antigas da câmera como fixtures. Elas são
Markdown legado e não correspondem ao novo schema; isso não é uma nova descoberta
semântica. Outro teste encapsula interpretações geométricas com citação válida
da câmera para garantir que sucesso mecânico nunca promova a frase a aprovada.

O [experimento real](../../experiments/claims-camera-20260928-01/README.md)
usou a mesma pergunta e somente E1. O retorno foi preservado e rejeitado pelo
schema (cerca Markdown). Uma inspeção adicional identificou quote sem indentação.
Não houve correção silenciosa da resposta nem fechamento da Etapa 1.
