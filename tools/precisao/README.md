# Portão de precisão da Fase 2

Um comando mede a precisão das relações de "o que chama?" e decide se um passo da Fase 2 pode ser aceito. Ele não altera o código de produção nem o acervo. O Studio roda sobre uma cópia do `.studio` em uma pasta temporária, e o comando confere, no fim, que o original não mudou.

## Como rodar

Na raiz do repositório, com o binário do passo em teste já compilado (`cargo build --release`):

```
node tools/precisao/portao.js --studio .studio --base tools/precisao/linha-de-base.json
```

| Parâmetro | Padrão | O que é |
|---|---|---|
| `--studio` | (obrigatório) | Pasta `.studio` com `healthchecks-25597e41` e `renderer-9b78f3a9`. Só é lida |
| `--bin` | `target/release` | Pasta com `librarian_studio` e `librarian_ingest` |
| `--base` | nenhum | Linha de base para aplicar a regra. Sem ela, o comando só mede |
| `--saida` | pasta temporária | Onde gravar `relatorio.md` e `resultado.json` |
| `--gravar-base` | nenhum | Grava o resultado como nova linha de base. Só com aprovação do usuário |
| `--so-sinteticos` | — | Mede só os níveis N0–N8, sem dados reais. É a parte que pode entrar no CI |

Código de saída: 0 aprovado (ou só medição), 1 reprovado, 2 não conseguiu medir.

Requisitos: Node 18 ou mais novo. Os projetos-fonte precisam estar no commit dos runs de referência: Healthchecks em `6c05a7a` e o ray tracer em `e38254b`. Se não estiverem, o comando recusa.

O Healthchecks leva cerca de 2 minutos, porque `/api/calls` é chamado uma vez por função.

## O que o comando faz

1. Copia os dois projetos do `.studio` para uma pasta temporária e sobe o `librarian_studio` sobre a cópia, numa porta livre.
2. Gera um run novo de cada projeto com o binário em teste. É nele que tudo é medido.
3. Mede a população de chamadas `obj.metodo()` fora de testes, sem `self.m()`. Dá a contagem por `resolution` e por estrato, e os links de método para função solta.
4. Classifica a amostra das fixtures contra a verdade validada. Calcula também a estimativa ponderada para a população: a proporção de cada estrato na amostra vezes o tamanho do estrato. É **estimativa**.
5. Refaz a auditoria dos fatos F1–F26 da Parte E.
6. Gera os projetos sintéticos N0–N8 e compara a resposta com o resultado registrado e com o alvo da Fase 2.
7. Roda `librarian_ingest verify-graph` em cada run antigo e confere que os arquivos dele não mudaram durante a medição.
8. Confere o hash agregado do `.studio` original, antes e depois.

## Categorias da amostra

| Categoria | Quando |
|---|---|
| `correta` | Link `unique`, `receiver`, `type` ou outro de alvo único, apontando para o alvo certo |
| `link_falso` | Link `unique`, `receiver`, `type` ou outro de alvo único, apontando para o alvo errado |
| `candidatos_falsos` | `homonym` ou acima do teto, sem o alvo certo entre os candidatos. Inclui acima do teto com alvo externo |
| `ambigua` | `homonym` com o alvo certo entre os candidatos, ou acima do teto com alvo no projeto |
| `sem_candidato` | Sem link (`no_candidate_in_project`) |

## Regra do portão

Um passo da Fase 2 reprova se, em Python ou em Rust:

- `link_falso` aumentar;
- `correta` diminuir;
- uma chamada da amostra não for encontrada no run novo;
- uma edition antiga falhar no `verify-graph` ou mudar;
- o `.studio` original mudar.

`candidatos_falsos`, ambíguas, a população, os níveis e a auditoria dos fatos são registrados com a variação, mas não reprovam sozinhos.

A regra compara só as chamadas que a linha de base também tem. Um estrato acrescentado à amostra depois aparece numa tabela à parte até entrar numa nova linha de base. Foi o caso de RT-31 a RT-55, do passo 2.1, que entraram na linha de base em 19/11/2026. HC-61 a HC-75 e RT-56 a RT-62, do passo 2.2, entraram na linha de base em 19/11/2026, com a verdade validada pelo usuário. Uma chamada da amostra fora da população (por caminho, como `Ray::new`, ou em testes) é procurada entre todas as chamadas do run. Com a amostra original, o veredito é o mesmo da regra anterior (DEC-37). Uma exceção só vale com justificativa caso a caso e aprovação do usuário.

## Arquivos

| Arquivo | Conteúdo |
|---|---|
| `portao.js` | O comando |
| `lib.js` | Cópia do `.studio`, Studio numa porta livre, hashes, o conjunto de chamadas e as respostas |
| `amostra.js` | Como a amostra foi sorteada: rodado uma vez, sobre a linha de base de 18/11/2026. Não roda no portão |
| `fixtures/amostra-hc.json`, `fixtures/amostra-rt.json` | 75 chamadas do Healthchecks e 62 do ray tracer (RT-31 a RT-55 vêm do passo 2.1; HC-61 a HC-75 e RT-56 a RT-62, do passo 2.2), com verdade, certeza, justificativa e `verdade_validada_por` |
| `fixtures/fatos.json` | As chamadas que cada fato F1–F26 afirma, com a verdade |
| `fixtures/niveis.json` | N0–N8: resultado atual e alvo da Fase 2, com o passo |
| `sinteticos/py`, `sinteticos/rs` | Projetos dos níveis, copiados de `docs/reports/piloto-1b/dados/tipos/` |
| `linha-de-base.json` | Resultado com o binário do passo 2.2, sobre as 75 chamadas do Healthchecks e as 62 do ray tracer, gravado com `--gravar-base`. Aprovada pelo usuário em 19/11/2026 (a anterior, do passo 2.1, sobre 60 + 55 chamadas, está no histórico do git) |

A amostra é identificada pelo caminho e pelo intervalo de bytes da ocorrência (`arquivo@início-fim`), que são os mesmos em qualquer run do mesmo código-fonte. A verdade é do usuário (DEC-22). Uma verdade nova ou corrigida entra na fixture com `verdade_validada_por` indicando quem validou.
