# librarian-evidence

O Evidence Builder do Librarian. Ele monta, confere e refaz o pacote de evidências de uma
pergunta a partir dos dados que recebe, e de nada mais. Não lê arquivos, não consulta o Git
durante a execução, não roda comandos e não chama modelos. Também não conhece o Studio, as
rotas HTTP dele, as pastas, as execuções ou o armazenamento. Quem consome o Builder (o Studio,
um agente ou outra aplicação) lê o que é preciso e entrega pelos tipos deste crate.

```text
Projeto / Studio → dados de entrada → Evidence Builder → pacote verificável → consumidor
```

```powershell
cargo test --locked -p librarian-evidence
```

## Dependências

```text
librarian-core ← librarian-graph-engine ← librarian-ingest ← librarian-evidence ← librarian-studio
```

- **Os tipos do pacote ficam em `librarian-ingest`** (`src/evidence.rs`), junto com `lines-v1`
  e as conferências puras (`compose`, `check_structure`, `check_sources`, `check_selection`).
  O motivo é que `prepare`, que está no ingest, usa esses tipos. Se eles viessem para cá, o
  ingest dependeria deste crate e este crate dependeria do ingest, formando um ciclo.
- **Este crate usa esse contrato.** Ele depende só do ingest, de `serde` e de `serde_json`.
  Ninguém além do Studio depende dele, e ele não depende do Studio nem do `librarian-llm-engine`.
- O teste `tests/independence.rs` confere duas coisas: que nenhum arquivo do núcleo lê arquivos,
  roda comandos, lê variáveis de ambiente ou cita um consumidor; e que o `Cargo.toml` não
  depende do Studio.

## Módulos

| Arquivo | Papel |
| --- | --- |
| `src/package.rs` | `build(BuildInput)`: os bytes do pacote ou o código estável da falha. Também `measure_at_question`, `edition_origin` e os tipos de entrada |
| `src/check.rs` | H1 e H2 (`check_package`) e H3 (`rederive`) sobre os artefatos recebidos (`TurnFiles`) e o registro da pergunta (`QuestionRecord`) |
| `src/selection.rs` | `accept`: confere a seleção contra as sugestões citadas e a edição, e gera o registro dela |
| `src/suggestion.rs` | A sugestão de envelope (`answer`, `record`, anotação da cobertura), `verify`, e o H4 (`replay` e `reproduce`) |
| `src/lacuna.rs` | Os nomes que uma LACUNA pede (`lacuna_names`), as declarações que eles apontam (`resolve`) e a conferência dos complementos (`check_additions`) |
| `src/keeper.rs` | `Keeper`: o que o Builder pede durante uma conferência, quando a chave só aparece no meio dela |
| `src/identity.rs`, `build.rs` | A identidade do código que monta o pacote. É gravada na compilação: o commit e o `git_dirty` do repositório, ou `unknown` |

## Interfaces

### Entrada empurrada pelo consumidor

O consumidor lê tudo e entrega de uma vez:

- **`BuildInput`:** a edição (`Catalog`, já carregado), a composição do `prepare`, a pergunta
  como foi feita (`Asked`), a origem da evidência (`Origin`), os arquivos preparados
  (`Prepared`), os arquivos de claims (`ClaimsFiles`) e os critérios (`Criteria`). Inclui também
  as linhas do acervo de cada fato revisado, a medição da hora da pergunta, a seleção aceita e o
  formato do pacote.
- **`TurnFiles` e `QuestionRecord`:** os artefatos e o registro da pergunta, para as conferências.

Um arquivo que o consumidor não conseguiu ler chega como ilegível (`None`, `Err` ou
`Criteria::Unreadable`). Cada um é recusado no mesmo ponto e com o mesmo código ou mensagem que
o Studio dava quando ele próprio lia o arquivo. A ordem das recusas foi preservada.

### Entrada pedida durante a conferência (`Keeper`)

Para o que só se sabe no meio da conferência:

- os bytes da sugestão citada por um hash;
- os bytes de uma execução;
- a leitura de uma execução como medição (`hot`);
- o grafo da execução;
- os textos de LACUNA de um envio.

O `Keeper` devolve os dados como estão. Um erro chega com a mensagem do próprio `Keeper`, e o
Builder recusa usando essa mensagem sem alterá-la.

**A leitura de uma execução fica com o `Keeper`.** O formato de uma execução é de quem mede. Um
consumidor que não lê medições devolve `None` em `hot`. Nesse caso, o H4 de uma sugestão feita
sobre uma medição diz `output_differs`, em vez de dar a sugestão como reproduzida.

### Saída

| Operação | Resultado |
| --- | --- |
| `build` | `Ok(bytes)` ou um código estável: `artifact_unreadable`, `snapshot_unavailable`, `claims_context_unreadable`, `composition_failed`, `acervo_unreadable`, `provenance_failed`, `measure_unidentified`, `edition_unreadable`, `server_additions_unreadable` ou `serialization_failed` |
| `check_package` | `(sha256, pacote, bytes)`, ou uma recusa com prefixo `H1:` ou `H2:` e terminada em `; nada foi enviado` |
| `rederive` | `()`, ou uma recusa `H3:` que nomeia a primeira parte que difere |
| `reproduce` | Uma `Reproduction` (H4), que informa e nunca recusa |

As mensagens são contrato: o `GET /api/evidence` do Studio identifica a etapa pelo prefixo
(`"H2:"`), e a tela mostra o texto.

## Identidade e proveniência

O contrato atual distingue quatro identidades. Nada no formato mudou nesta extração.

| O quê | Onde está | O que garante | O que não garante |
| --- | --- | --- | --- |
| **Edição do código** | `edition.origin` (`edition:` mais o sha256 do manifesto da edição), `edition.catalog_sha256`, `extractor`, `git_head` e `git_dirty` do projeto analisado | Que as unidades vêm dessas cópias do código; o H2 refaz cada unidade a partir delas | Que o projeto no disco ainda é esse |
| **Pacote** | sha256 dos bytes do pacote; a marca da pergunta guarda `{format_version, sha256}` | Que o pacote é aquele que a pergunta nomeia (H1) | Quem gravou o arquivo: hash mostra consistência, não autoria |
| **Conjunto de artefatos** | `bound` (request, search, dossier, query do bundle, contexto de claims, critérios), `query.sha256`, o hash do registro de seleção, os hashes das sugestões, o hash da execução da medição e as referências dos complementos | Que cada artefato vinculado é o que o pacote diz (H1) e que o pacote sai deles de novo (H3) | Ver as lacunas abaixo |
| **Proveniência da construção** | `versions`: `normalization`, `rules` e `builder` (`recorded` com commit e `git_dirty`, ou `unknown`), mais `format_version` | Qual regra e qual código montaram o pacote | Que esse código ainda existe; `code_recoverable` do H4 só diz se o commit estava limpo |

**Por que o hash da edição mais o hash do pacote não bastam como identidade de auditoria.** Há
cinco lacunas, todas registradas aqui e não resolvidas nesta extração:

1. **Os complementos de LACUNA apontam para o envio só por identificadores do Studio**
   (`from_turn` e `from_send`), sem o hash da resposta. O H1 relê a LACUNA pelo `Keeper`, mas
   o pacote não fixa os bytes que ela tinha.
2. **O registro da pergunta não tem hash próprio.** O pacote fixa o texto preparado, o texto da
   pessoa e os acréscimos, mas não `origin.json` do bundle nem o resto do `question.json`.
3. **O pacote não sabe quem pediu.** Não registra pessoa, agente nem perfil de consumidor.
4. **As respostas ficam fora do pacote.** O vínculo de envio do Studio nomeia o pacote
   (`evidence_sha256`), mas o pacote não nomeia o envio.
5. **Um build feito a partir de código fora do Git grava `builder: unknown`.** Foi o caso deste
   crate enquanto ele não estava no índice. O `build.rs` exige que o `Cargo.toml` do crate esteja
   rastreado.

**Proposta para o formato 3** (não implementada; precisa de validação):

- um identificador do conjunto de artefatos;
- o hash de cada envio citado por um complemento;
- o pedido do consumidor como artefato vinculado: objetivo, perfil e quem pediu.

Os formatos 1 e 2 continuam sendo lidos e refeitos como foram gravados.

A lacuna 3 ("o pacote não sabe quem pediu") é o ponto de partida do contrato de pedido de
evidências (Evidence Request, Fase 4) e da separação entre autoria, revisão e aprovação dos fatos
(Fase 3). As duas fases estão no [plano](../../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md#piloto-python-1-e-direção-multiagente-18112026)
e na [arquitetura para vários consumidores](../../docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md).
Nada disso está implementado.

## O que fica com quem consome (no Studio, hoje)

- **Ler e gravar arquivos:** `.studio/`, gravação imutável, `question.json`, `evidence.json`,
  `selection.json`, `suggestions/`.
- **Montar a pergunta:** preparar o bundle (`prepare::write`) e escrever os arquivos de claims
  (que usam o `librarian-llm-engine`). Também ler as configurações e o perfil do projeto
  (`librarian.json`, regras do inventário) e transformar os campos de `question.json` em
  `Origin` e `QuestionRecord`.
- **A medição:** rodar comandos, ler execuções e interpretá-las ("Onde pesa", cobertura). Ao
  Builder chegam só a referência e a leitura (`MeasureRead` e `Keeper::hot`).
- **O acervo de fatos:** as linhas de cada fato promovido e os fatos atuais de cada item de uma
  sugestão. O Builder grava esses fatos como recebidos e não os refaz.
- **As respostas e a escada:** enviar a pergunta aos modelos, ler a LACUNA de um envio
  (`ladder::outcome`) e decidir quantas declarações cabem (`lacuna_room`).
- **A ordem das conferências antes de um envio:** H1 e H2, `prepare::verify`, critérios,
  `claims::verify`, H3. Também o vínculo de envio.
- **O gancho de teste `failing_at("composition")`:** ele é conferido antes de chamar `build`. No
  código anterior, ele ficava depois da leitura dos artefatos. Só muda o código devolvido num
  teste que falhe nos dois pontos ao mesmo tempo, e nenhum teste faz isso.
