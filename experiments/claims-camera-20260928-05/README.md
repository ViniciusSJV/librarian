# Localização estruturada — câmera, série 05

Pergunta: Onde a câmera calcula a direção dos raios?

As três respostas identificaram `ray_from_pixel`, `src/camera.rs`, declaração
na linha 75 e cálculo na linha 86. Todas citaram literalmente:

```rust
        let direction = (pixel - origin).normalize();
```

Entretanto, omitiram a citação da declaração. As três foram rejeitadas pelo
checker com `location_declaration_not_cited`. O caso não foi aprovado.

## Mudança e critérios

O perfil `require_location=true` acrescenta campos verificáveis à resposta.
Cada FACT deve identificar a evidência, arquivo, símbolo, linha da declaração
e intervalo da operação. As citações devem cobrir declaração e operação.

O consumidor forneceu metadados de `ray_from_pixel` por inspeção manual de E1:
declaração 75, coluna 8 (base 1, caracteres Unicode), fim 89. O checker confere
o nome na posição informada e a coerência dos limites, mas não faz parsing.
Não houve integração automática com o ingest nesta série. O excerpt e a
pergunta são iguais aos da série 04; a linha da operação não foi fornecida
nos metadados ou como resposta esperada no schema.

O escopo `Camera` não aparece no excerpt preservado e não foi acrescentado.
Nome em `location` satisfaz a identificação, sem exigir duplicação em `text`:
essa mudança de critério foi registrada antes das chamadas em
[protocol-before.md](protocol-before.md). As respostas também incluíram o nome
no texto livre, sem extrapolações geométricas ou categorias adicionais.

## Resultado e preservação

| Critério | Resultado |
| --- | --- |
| Transporte completed / JSON válido | 3/3 |
| Nome, arquivo, declaração e linha do cálculo corretos | 3/3 |
| Citação literal do cálculo | 3/3 |
| Citação da declaração | 0/3 |
| Aprovação mecânica / aprovação do caso | 0/3 |

As três respostas são idênticas em bytes. Mesma semente e cache não constituem
validação independente. Modelo/configuração e versão do servidor estão em
`model-show.json` e `server-version.json`; não houve alteração do SYSTEM ou
troca de modelo. `inputs-before.json` fixa os hashes dos insumos antes das
chamadas. `evaluation.json` vincula os artefatos e registra a avaliação posterior
separada dos relatórios mecânicos. Não houve reparos nem novas tentativas.

Em comparação com a série 04, o nome deixou de ser omitido. A exigência de
citar a declaração passou a ser verificável e detectou a falha restante.
Não se deve comparar diretamente as taxas de aprovação: o checker e o perfil
de resposta mudaram. O checker mantém `accepted=false/semantic_status=pending`.

## Verificação e continuidade

`cargo test --locked --offline --workspace`: 65 aprovados, 1 ignorado;
31 aprovados no LLM Engine, incluindo seis novos testes do perfil. A cobertura
inclui símbolo inventado, outra função no mesmo trecho, intervalos inválidos,
citações incompletas, quote divergente, coluna Unicode, contexto inválido,
LACUNA sem símbolo e regressão da resposta original da série 04.

Os bins foram compilados e os consumidores `prepare_ollama`, `send_ollama` e
`explain_evidence` do renderer passaram em `cargo check --locked --offline`.

A próxima mudança pode vincular as citações diretamente aos campos de
declaração/operação, reduzindo a duplicação entre `location` e `citations`;
isso exige um novo protocolo, sem reparar os resultados desta série.
A Etapa 1 permanece aberta. Outros casos e integração com metadados do ingest
continuam pendentes. Nenhuma alteração automática de código foi autorizada.
