# Testar o Librarian em poucos passos

Requisitos: Git, Rust/Cargo e linker instalados; rede para obter dependências no
primeiro uso. Execute na raiz deste repositório. Não precisa de banco ou Ollama.
Este roteiro corresponde à árvore que contém o exemplo `export`.

## 1. Usar uma fonte

```sh
cargo run --locked -p maintenance-example -- examples/maintenance/manual.txt ./demo-bundle
```

Esperado: `Bundle validado: ./demo-bundle`, com `dossier.json`, `question.txt`,
`query.json` e `origin.json`. O manual é fictício; fatos são cadastrados pelo
adapter, não extraídos automaticamente.

## 2. Consultar um fato

```sh
cargo run --locked -p librarian-graph-engine --example export -- demo-bundle/dossier.json demo-bundle/question.txt demo-record 0 F_RECORD
```

Esperado: término sem erro. Abra `demo-record/query.json`:

| Campo | Esperado |
| --- | --- |
| `evidence.fact_id` | `F_RECORD` |
| `evidence.source.id` | `MANUAL_V1` |
| `evidence.source.line` | `3` |
| `evidence.source.excerpt` | `Registrar a data e o resultado da inspeção.` |
| `evidence.evidence_unknowns` | Exemplo fictício; não comprova manutenção executada. |

A aplicação escolheu o ID; o Graph Engine não interpretou a pergunta para fazer
busca. O resultado é material conferido, não uma resposta automática em prosa.

## 3. Observar uma rejeição

Repita o comando do passo 2. Deve falhar porque `demo-record` já existe, preservando
o bundle anterior. Para uma nova consulta, escolha outro destino; não apague o
histórico para continuar. ID desconhecido ou repetido também impede a preparação.

## Verificação final opcional

Após alterações de código, execute uma vez:

```sh
cargo test --workspace --locked
```

Não é necessário repetir a suíte para cada consulta. Falta de arquivo indica que
o diretório atual ou o caminho cadastrado precisa ser conferido; divergência de
hash indica outra edição da fonte. Não substitua o hash para contornar a falha.

Para outra fonte, adapte o [cadastro de exemplo](examples/maintenance/src/main.rs),
incluindo afirmações, linhas e lacunas próprias. O exemplo tem fatos fixos para
seu manual: trocar apenas o arquivo não adapta o significado das afirmações.

Veja o [fluxograma e regras](WORKFLOW.md) e o
[roteiro explicado](examples/maintenance/README.md).
