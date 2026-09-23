# Consultar uma fonte antes de adotar um banco

Este exemplo usa um manual fictício de três linhas. Funciona apenas com arquivos,
Rust e os crates do Librarian; não requer renderer, banco ou servidor LLM.

## 1. Cadastrar e conferir a fonte

Execute na raiz do Librarian, usando um diretório de destino que ainda não exista:

```sh
cargo run --locked -p maintenance-example -- examples/maintenance/manual.txt ./maintenance-demo-v1
```

O adapter em `src/main.rs` lê `manual.txt`, calcula seu SHA-256 e cadastra
`MANUAL_V1` com as linhas do arquivo. Em seguida cadastra duas afirmações escritas
explicitamente no código:

| Fato | Referência | Afirmação cadastrada |
| --- | --- | --- |
| `F_INSPECTION` | `MANUAL_V1`, linha 2 | O manual registra inspeção mensal. |
| `F_RECORD` | `MANUAL_V1`, linha 3 | O manual pede registro da inspeção. |

O Graph Engine confere o dossiê e os bytes da fonte com `SourceChecks`, seleciona
`F_RECORD` e `F_INSPECTION` nessa ordem e exporta a pergunta “O que o manual
solicita?”, acompanhada das evidências e de seus limites.

O destino contém:

- `dossier.json`: cadastro completo da fonte, fatos e lacuna declarada.
- `question.txt`: pergunta e orientação para citar IDs.
- `query.json`: fatos selecionados e contexto conferido.
- `origin.json`: hashes dos arquivos exportados e alcance da conferência.

## 2. Consultar novamente o cadastro

Para selecionar somente a orientação sobre registro, reutilize o dossiê e a
pergunta exportados. Continue na raiz do Librarian; use outro destino novo:

```sh
cargo run --locked -p librarian-graph-engine --example export -- maintenance-demo-v1/dossier.json maintenance-demo-v1/question.txt maintenance-record-v1 0 F_RECORD
```

Esta consulta tem seleção explícita de `F_RECORD` e raio de contexto zero.
O Graph Engine reconfere todas as fontes e fatos, mesmo os não selecionados.
No `query.json`, `evidence.fact_id` deve ser `F_RECORD`,
`evidence.source.line` deve ser `3` e o trecho deve ser:

> Registrar a data e o resultado da inspeção.

Para uma única ficha, o contexto aparece em `evidence.source.context`.
Para várias, a exportação usa `evidence.selections` e `evidence.contexts`.
A lacuna “Exemplo fictício; não comprova manutenção executada.” acompanha ambos.

## O que significa consultar aqui

A aplicação escolhe os IDs. A pergunta não é interpretada pelo Graph Engine para
buscar fatos automaticamente. O resultado é evidência selecionada e conferida,
pronta para inspeção humana ou futura explicação por um LLM.

Uma leitura humana pode dizer: “O manual pede registrar a data e o resultado da
inspeção [F_RECORD / MANUAL_V1:3]”. O material não permite dizer que uma inspeção
ocorreu. Essa frase exemplifica uma leitura; não é uma resposta gerada pelo engine
nem uma avaliação semântica realizada.

## Reutilizar com outra fonte

O exemplo de manutenção tem afirmações e linhas fixas para este manual. Trocar
somente o arquivo não adapta essas afirmações: é necessário cadastrar e revisar
os fatos do novo domínio. Uma referência existente não garante sustentação textual.

Para outra fonte, monte um novo dossiê com identidade própria, caminho, hash,
linhas, fatos e lacunas, seguindo o adapter. Use o exemplo genérico `export` para
conferir e consultar esse cadastro. Mantenha a edição anterior e exporte em novo
destino. Se a fonte mudar depois do cadastro, a reconferência deve rejeitar a
divergência; não atualize hashes apenas para silenciar o erro.

Um banco poderá persistir e relacionar esses registros posteriormente. A escolha
do armazenamento não é necessária para exercitar este contrato de ponta a ponta.
