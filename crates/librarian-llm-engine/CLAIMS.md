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
de uma fonte conferida. Na série 05 houve inspeção manual do trecho preservado;
a integração automática com símbolos do ingest ainda não foi feita.

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
continuam invariáveis. Tampouco se comparam automaticamente as frases livres
aos campos estruturados. Este é um perfil aditivo experimental da versão 1.

A [série 05](../../experiments/claims-camera-20260928-05/README.md) identificou
nome e linhas corretos em três respostas, mas todas foram rejeitadas por falta
da citação da declaração (`location_declaration_not_cited`).

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
