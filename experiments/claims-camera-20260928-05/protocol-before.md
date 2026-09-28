# Protocolo registrado antes da geração
Três tentativas sequenciais, sem reparos ou novas tentativas em caso de falha.
Pergunta e bytes do excerpt E1 iguais à série 04; mesmo modelo renderer-analyst:latest e servidor local.
Mudança: perfil require_location=true, campos estruturados de localização e metadados de símbolo fornecidos pelo consumidor.
Metadados conferidos manualmente em E1: ray_from_pixel, declaração 75, coluna 8 (base 1, caracteres Unicode), fim 89. Não houve extração automática pelo ingest. O escopo Camera não está no excerpt e não será exigido nem acrescentado.
A linha do cálculo não é pré-selecionada nos metadados ou no schema: o modelo deve localizá-la.
O checker valida identidade, intervalos e citações, mas não analisa sintaxe nem decide se a operação responde à pergunta.
Critérios: JSON válido; FACT com location identificando ray_from_pixel e src/camera.rs, declaração 75, operação 86; citações literais cobrindo declaração e cálculo (pixel - origin).normalize(); sem geometria, execução, desempenho ou comportamento ausente; sem inferências, hipóteses e lacunas não solicitadas/irrelevantes.
O nome em location satisfaz a identificação; não se exige duplicá-lo no texto livre. Este critério substitui explicitamente a exigência textual da série 04 antes das chamadas.
Aceitar este caso apenas se todas as três respostas cumprirem os critérios. O relatório mecânico mantém accepted=false/semantic_status=pending; parecer posterior separado. A Etapa 1 continua aberta e repetições com mesma semente/cache não são validação independente.