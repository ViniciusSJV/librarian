# Localização completa — câmera, série 04

Pergunta: Onde a câmera calcula a direção dos raios?
Evidência: somente E1, src/camera.rs:75–89, preservada desde a série 01.

## Alteração e resultado

Esta série manteve schema, mapa de linhas, modelo, contexto e checker. Acrescentou
instrução explícita de que text deve nomear a função escrita no trecho e citar
sua declaração além da operação. Três tentativas foram definidas antes da geração
em protocol-before.md. Critérios, configuração e respostas estão preservados.

As três passaram mecanicamente, mas repetiram exatamente o texto da série 03:

> A câmera calcula a direção dos raios na linha 86 do arquivo src/camera.rs.

A citação é literal, inclusive os espaços iniciais:

```rust
        let direction = (pixel - origin).normalize();
```

Esse conteúdo é sustentado e responde onde está a operação. Contudo, a rubrica
prévia também exigia nomear ray_from_pixel, e isso foi omitido. Não alteramos a
rubrica para aprovar depois da geração: decisão global não aceita, por incompletude.
Não houve extrapolação geométrica ou categorias/lacunas extras nessas três saídas.

## Comparação das séries desta etapa

| Série | Mudança | Transporte | Mecânica | Conteúdo/completude |
| --- | --- | --- | --- | --- |
| 02 | JSON Schema no format do Ollama | 3/3 completed | 0/3: citação divergente | Linha errada e nome omitido |
| 03 | Mapa determinístico de linhas em E1 | 3/3 completed | 3/3 | Localização/fórmula sustentadas; nome omitido |
| 04 | Exigência explícita de nome em text | 3/3 completed | 3/3 | Mesmo resultado da série 03 |

evaluation.json contém hashes, conferências e parecer por tentativa. O checker
sempre mantém accepted=false/semantic_status=pending; a avaliação posterior é
separada. Nenhuma resposta original foi reparada, aparada ou reatribuída.

As repetições dentro de cada série usaram mesma query, modelo, semente e servidor,
com cache reportado pelo provedor. Textos idênticos não demonstram confiabilidade
geral; não houve avaliação independente com perguntas novas. Modelo e parâmetros
reportados foram preservados; não houve troca para DeepSeek ou modificação do SYSTEM.

## Implementação e continuidade

`claims::response_schema` descreve formato e categorias permitidas sem fornecer
a resposta. O adaptador mapeia response_schema para format, preservando o prompt
integral. `claims::query` gera line_map a partir de E1, sem acrescentar contexto.
O verificador de citações não mudou. Queries legadas sem schema continuam com o
corpo HTTP anterior.

A documentação oficial consultada em 28/09/2026 descreve o campo format como JSON
Schema: [API generate](https://docs.ollama.com/api/generate) e
[saídas estruturadas](https://docs.ollama.com/capabilities/structured-outputs).
O comportamento observado nas nove chamadas, e não a documentação por si só,
sustenta os resultados acima.

Etapa 1 permanece aberta. O próximo experimento pode separar a localização
estruturada da explicação livre, preservando critérios prévios. Depois continuam
necessários os casos integral, parcial, impossível, inferência e hipótese.
Não há validação semântica automática nem autorização para alterações de código.

## Verificação de desenvolvimento

A suíte `cargo test --locked --offline --workspace` passou: 59 testes aprovados,
1 gravador ignorado. O LLM Engine soma 25 aprovados (16 anteriores e 9 do contrato).
Os testes verificam também mapeamento opcional do schema, ausência de format em
queries legadas, categorias autorizadas independentes e geração do mapa de linhas.
Os binários consumidores do renderer passaram em cargo check --locked --offline.
Nenhuma chamada real foi usada como substituta dos testes de falhas do transporte.
