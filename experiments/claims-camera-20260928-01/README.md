# Primeiro experimento de afirmações — câmera

Pergunta: Onde a câmera calcula a direção dos raios?
Modelo chamado: renderer-analyst:latest via Ollama. Configuração reportada antes
do envio: model-show.json. Request: CLAIMS_CAMERA_20260928_01.

E1 veio da consulta histórica policy-camera-20260927-02 do renderer; o trecho não
foi ampliado. context.json preserva a seleção e desabilita inferência/hipótese.
provenance.json identifica sua origem. criteria-before.md foi escrito antes da geração.
Não houve alteração do SYSTEM instalado, dos parâmetros ou da política do renderer.
A variável experimental foi a query com contrato explícito de afirmações/citações.

## Resultado

Transporte completed. A resposta em attempt/response.txt contém JSON dentro de
cercas Markdown, apesar da exigência de somente JSON. mechanical-report.json
rejeita com invalid_response_schema e conserva accepted=false/semantic_status=pending.

A afirmação localiza a operação em src/camera.rs:86 e cita a fórmula correta.
Não afirma espaço geométrico, execução ou desempenho, nem cria lacunas extras.
Contudo, não identifica Camera::ray_from_pixel no texto, conforme critério prévio,
e a citação omite os oito espaços de indentação exigidos pelo contrato literal.
O conteúdo está mais restrito neste caso, mas a tentativa não é aprovada.
Uma amostra e uma mudança de formato não demonstram melhoria geral ou causalidade.

diagnostic-without-fence.json é uma cópia derivada para diagnóstico, com apenas
as cercas externas removidas pelo avaliador, sem alterar a resposta original.
diagnostic-report.json mostra quote_mismatch. Esse arquivo derivado não substitui
a saída do modelo nem permite aprovar sua tentativa.

## Avaliação posterior

| Critério prévio | Resultado |
| --- | --- |
| JSON puro e citações exatas | Reprovado: Markdown e remoção de indentação. |
| Localizar método e reproduzir fórmula | Parcial: linha e fórmula corretas, método omitido. |
| Sem geometria/execução/desempenho inventados | Aprovado para esta resposta. |
| Sem inferência/hipótese não solicitada | Aprovado para esta resposta. |
| Sem lacuna irrelevante/contraditória | Aprovado; não criou lacunas. |
| Sustentação e aceitação global | A afirmação de localização é sustentada; tentativa reprovada pelo contrato. |

Avaliação feita pelo assistente após a geração. verification.json liga artefatos,
critérios, configuração e contexto por hashes; mede integridade, não autenticidade.
O array context e métricas originais permanecem em attempt/response.bin.
O resultado de transporte não foi reescrito. A consulta usa evidência histórica;
não afirma conferência da árvore atual ou snapshot atômico.

Próximo passo: decidir e testar explicitamente a rigidez do formato/citação, sem
normalização silenciosa. Depois exercitar casos integral, parcial, impossível,
inferência e hipótese, e avaliar cada afirmação semanticamente. Etapa 1 permanece aberta.

## Verificações de desenvolvimento

`cargo test --locked --offline --workspace` passou. O LLM Engine tem 24 testes
aprovados (16 existentes e 8 novos), com 1 gravador ignorado. Os novos testes
cobrem citação alterada, referências/intervalos inválidos, permissões independentes,
campos condicionais, JSON inválido, preservação de originais e recusa de sobrescrita.
O consumidor renderer passou no cargo check dos três binários ligados ao engine;
seu lockfile recebeu somente a dependência serde do crate atualizado.
