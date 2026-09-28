# Schema enviado ao Ollama — câmera, série 02

Três tentativas previstas em protocol-before.md, mesma pergunta/contexto da série
01 e mesma rubrica. A nova query contém response_schema, enviado também como format
na API. Modelo/SYSTEM/parâmetros preservados em model-show.json; versão do servidor
em server-version.json. Não houve limpeza ou correção da saída.

Resultado: 3 transportes completed, 3 respostas em JSON puro, 3 rejeições mecânicas
por quote_mismatch. O modelo atribuiu a operação à linha 88, mas ela está na linha
86. Citou 88–89 com texto da operação e do retorno, omitindo indentação. Também
omitiu o nome da função exigido pela rubrica. A avaliação semântica rejeita a
localização errada. Nenhuma das tentativas foi aceita.

evaluation.json liga request_id, hashes dos artefatos, query, contexto, rubrica,
protocolo e configuração e registra a decisão posterior do assistente por tentativa.
Os três textos são idênticos; cache e semente fixa impedem tratar as repetições
como amostras independentes de confiabilidade. Metrics de tokens não provam
processamento semântico integral do contexto.

O schema resolveu o formato nas amostras observadas, não a correção de referências.
A série 03 acrescenta mapa de linhas derivado de E1; o checker permanece exato.
