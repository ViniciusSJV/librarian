# Gabaritos do passo 2.4e

Ferramentas de medição da caça às lacunas (DEC-48). O gabarito é só régua: nada daqui entra no grafo, nos crates, no portão ou no acervo (DEC-48, item 6). Nenhuma delas muda um projeto: rodam sobre o projeto como está ou sobre uma cópia. Paradas: [2.4e-parada-1.md](../../../docs/reports/fase-2/2.4e-parada-1.md) e [2.4e-parada-2.md](../../../docs/reports/fase-2/2.4e-parada-2.md).

Nenhum caminho da máquina está no código: todo caminho é argumento. A única exceção é o `medir.js`, que, sem `--hc`/`--rt`, usa `GABARITO_HC`/`GABARITO_RT` e, sem elas, as pastas ao lado do repositório (`../pilotos/healthchecks` e `../renderer`). As saídas ficam fora do Git, numa pasta `<saida>` à escolha (Q13).

## Requisitos

- Projetos nos commits dos runs de referência: Healthchecks em `6c05a7a`, ray tracer em `e38254b`.
- Python 3.12 ou mais (`sys.monitoring`), num virtualenv fora do repositório e dos projetos, com as dependências do Healthchecks:
  ```
  uv venv --python 3.14.10 <venv>
  uv pip install --python <venv>/Scripts/python.exe -r <healthchecks>/requirements.txt -r <healthchecks>/requirements-dev.txt
  ```
- Node 18 ou mais; Rust (cargo e rust-analyzer do toolchain).

## Ordem e entradas / saídas

| Passo | Comando | Entrada | Saída em `<saida>` |
|---|---|---|---|
| 1. Resposta do Librarian | `node tools/precisao/gabarito/librarian.js --studio .studio --saida <saida> [--fontes <sha256>]` | `.studio` (lido; roda numa cópia) | `librarian-hc.json`, `librarian-rt.json`, `librarian-resumo.json` |
| 2. Execução, Python | `SECRET_KEY=dummy-key PYTHONDONTWRITEBYTECODE=1 <venv python> tools/precisao/gabarito/execucao_py.py <healthchecks> <saida>/execucao-hc.jsonl manage.py test` (sem `python -I`) | o projeto | `execucao-hc.jsonl` |
| 3. Tipos, Python | `SECRET_KEY=dummy-key PYTHONDONTWRITEBYTECODE=1 <venv python> tools/precisao/gabarito/tipos_py.py <healthchecks> <saida>/tipos-hc.jsonl hc --strict --platform linux` | o projeto (`mypy.ini` dele) | `tipos-hc.jsonl`; cache em `<saida>/mypy-cache` (ou `GABARITO_MYPY_CACHE`) |
| 4. Execução, Rust | `CARGO_TARGET_DIR=<fora> cargo build --release --manifest-path tools/precisao/gabarito/rs-sonda/Cargo.toml`, depois `rs-sonda <ray tracer> <cópia>` e, na cópia, com `G24E_SAIDA=<saida>/execucao-rt.jsonl` e `CARGO_TARGET_DIR=<fora>`: `cargo test --locked`, `cargo run --release --locked --bin librarian_measure -- <png> 160 85`, `--bin librarian_measure_mesh -- <png> teapot.obj 64 64`, `--bin bench_render` | o projeto (copiado) | `execucao-rt.jsonl` |
| 5. Tipos, Rust | no ray tracer: `CARGO_TARGET_DIR=<fora> rust-analyzer scip . --output <saida>/renderer.scip` | o projeto | `renderer.scip` |
| 6. Casamento | `node tools/precisao/gabarito/medir.js --saida <saida> [--hc <healthchecks>] [--rt <ray tracer>]` | as saídas acima | `medicao.json` |
| 7. Tabelas | `node tools/precisao/gabarito/tabelas.js --saida <saida>` | `medicao.json` | `tabelas.md` |

`scip.js` é o leitor do índice SCIP (sem dependências), usado pelo `medir.js`; sozinho, `node scip.js <índice> [<arquivo> <linha>]` mostra a codificação das colunas e as ocorrências de uma linha.

## Cuidados

- `PYTHONDONTWRITEBYTECODE=1` e o cache do mypy fora do projeto: sem isso, os dois gravam `__pycache__` e `.mypy_cache` no Healthchecks.
- `CARGO_TARGET_DIR` fora do projeto, nos passos 4 e 5: sem ele, o build vai para o `target/` do ray tracer.
- Registre as versões (`uv pip freeze`, rustc, rust-analyzer, node) e o SHA-256 de cada saída no relatório (Q5).
