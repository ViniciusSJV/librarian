# Fixture histórica v1

Edition congelada do perfil `librarian-ingest/0.1.0;syn/3.0.5;rust-structural-v1`,
gerada uma única vez em 19/10/2026 a partir de `project/`, antes da abstração do
extractor (etapa 2B). Testada por [`v1_regression.rs`](../../v1_regression.rs).

| Arquivo | Origem |
| --- | --- |
| `project/` | Fontes da fixture; `config.json` seleciona `src` e `scripts/helper.py` |
| `edition/` | `librarian_ingest generate` a partir de uma cópia fora do Git (`git.head`/`dirty` nulos) |
| `graph.json` | `librarian_ingest graph` |
| `graph-v2.json` | `librarian_ingest graph … v2`, acrescentado na etapa G2 sobre a mesma edition |
| `prepare-request.json`, `prepare-camera/` | `librarian_ingest prepare` (etapa G4): `search.json`, `dossier.json` e `query.json` do bundle |
| `search-camera.json`, `search-abstain.json` | `librarian_ingest search` com `lexicon.json` |
| `search-test-penalty.json` | `librarian_ingest search` sem léxico, com símbolo `#[test]` no ranking |

Casos cobertos: declarações Rust da v1, `impl Trait for`, `#[test]`, `cfg` observado,
macro não expandida, homônimos (`Ray`), BOM/shebang/CRLF/Unicode, erro de parse,
UTF-8 inválido, `.py` explícito como `auxiliary` e `.py` em diretório não descoberto.

**Não regenere estes arquivos para fazer um teste passar.** Divergência significa
mudança de comportamento da v1. Um perfil novo exige identidade nova e fixture própria;
esta continua sendo verificada como está. `.gitattributes` marca a pasta como `-text`
para preservar CRLF, BOM e bytes inválidos.
