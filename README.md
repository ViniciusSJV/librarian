# Bibliotecário

Bibliotecas Rust para catalogar fatos com fontes, conferir referências e preparar
consultas rastreáveis. Extraídas do projeto renderer; não dependem dele.

```text
consumidor → bibliotecario-core (contratos)
          → bibliotecario-graph-engine (conferência, seleção, consulta e bundle)
                                              ↓
                                  LLM do consumidor, se necessário
```

- `bibliotecario-core`: fato compartilhado, fonte com metadados genéricos e dossiê.
- `bibliotecario-graph-engine`: metadados de execução/captura, validação, janelas,
  seleção ordenada, exportação e conferência local de arquivos.
- `maintenance-example`: consumidor de outro domínio com dados fictícios.

## Executar

Na raiz deste repositório:

```sh
cargo test --workspace --locked
cargo run --locked -p maintenance-example -- examples/maintenance/manual.txt /tmp/maintenance-bundle
```

O destino deve ser novo. No Windows, substitua `/tmp/maintenance-bundle` por um
caminho local novo. Nenhum renderer, banco ou servidor LLM é necessário.

## API

```rust,ignore
let mut checks = bibliotecario_graph_engine::filesystem::SourceChecks::default();
let prepared = bibliotecario_graph_engine::prepare_query(
    &dossier_json, Some(&question), &["F1"], 2,
    |source| checks.validate(source),
)?;
prepared.write_bundle(destination, "dossier.json", "question.txt")?;
```

`prepare_query` valida todas as fontes e referências do dossiê, incluindo as não
selecionadas. O callback faz as verificações de armazenamento e captura; retornar
`Ok(None)` sem verificar arquivos limita a operação aos dados fornecidos.
`SourceChecks` verifica caminhos, hashes, linhas e capturas locais. Caminhos
relativos são resolvidos a partir do diretório atual, como na CLI original.

O resultado mantém os bytes originais do dossiê e da pergunta. `write_bundle`
preserva esses bytes, a consulta e os hashes em um diretório novo. A escrita não
reconfere fontes: o consumidor deve reconferir antes de um envio posterior.
`origin.json` é publicado por último; uma falha pode deixar um diretório parcial,
que não deve ser tratado como bundle concluído nem sobrescrito numa nova tentativa.

## Contratos e limites

`bibliotecario_graph_engine::Fact` é o próprio `BibliotecarioFact` do core.
`Source` especializa o contrato genérico do core com `ExecutionRecord` e
`CaptureLink`; `Evidence` também é compartilhado. A autoria opcional faz parte do
fato. Os tipos antigos `SourceRef`/`EvidenceBundle`/`QueryExport` são uma fachada
de compatibilidade de fonte única; novos consumidores devem usar o dossiê
multi-fonte e `prepare_query`. `EvidenceBundle::to_query` apenas serializa seus
dados: valide explicitamente ou use o pipeline novo.

IDs são locais ao dossiê. Caminho e hash identificam uma edição observada, mas
ainda não há um catálogo persistente de versões ou IDs globais. Essa decisão deve
preceder a implementação do banco de grafo.

Referência válida não prova que a afirmação é verdadeira. Hashes conferem bytes,
não autenticam autoria, computador, execução ou código compilado. Fatos são
fornecidos pelos adapters; não há extração automática nem promoção de resposta
LLM a fato. Este Graph Engine ainda não é um banco de grafos.

## Prova em outro domínio

O exemplo de manutenção cria fontes e fatos pelos contratos do core, confere o
arquivo pela API do Graph Engine, seleciona fatos e publica um bundle. Os testes
públicos verificam referências inválidas, ordem, fontes alteradas fora da seleção,
preservação do bundle anterior e recusa de sobrescrita.

Isso prova o fluxo Bibliotecário → Graph Engine → bundle fora do renderer.
Não prova uma avaliação semântica por LLM no domínio de manutenção; nenhuma
consulta real foi feita neste exemplo.

## Origem e próximos passos

Origem: `ViniciusSJV/renderer`, base `68dde8f` e alterações locais posteriores
revisadas em 22/09/2026. Este repositório inicia um histórico próprio; não contém
ZIPs de consultas, cenas ou dados privados do consumidor.

O renderer permanece consumidor de referência. O LLM Engine continua nele e
será extraído posteriormente. A integração planejada deve distinguir provedor,
endpoint e modelo; Qwen/DeepSeek via Ollama serão escolhas de configuração,
enquanto APIs hospedadas exigirão adapters e validação próprios.

Depois: definir contrato de armazenamento e versionamento, implementar um adapter
de banco de grafo e comparar sua consulta com o pipeline em arquivos. Não há banco
implementado nesta versão.
