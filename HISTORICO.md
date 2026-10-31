# Histórico: como o Librarian foi construído

O Librarian nasceu dentro do [renderer](https://github.com/ViniciusSJV/renderer), com o
nome de Bibliotecário. Em 19/10/2026 o renderer passou a ser **somente uma fonte**: o
código do Bibliotecário, as aulas, os experimentos e os acervos antigos foram removidos
de lá. Tudo continua publicado no commit
[`a6e56ba`](https://github.com/ViniciusSJV/renderer/tree/a6e56ba19213d879f5de6e3e07f8a75ef84fe569),
o último que contém esse material.

Os links abaixo apontam para esse commit. Os comandos citados dependem do código da
época (por exemplo, `validate_evidence`, `catalog_sources` e `capture_execution`) e
não funcionam com a árvore atual. Resultados históricos descrevem aquelas execuções,
não o comportamento atual.

| Material | Conteúdo |
| --- | --- |
| [Laboratório de IA](https://github.com/ViniciusSJV/renderer/blob/a6e56ba19213d879f5de6e3e07f8a75ef84fe569/ai/README.md) | Visão geral, objetivos e estado ao final da Aula 25 |
| [Aulas 1–25](https://github.com/ViniciusSJV/renderer/tree/a6e56ba19213d879f5de6e3e07f8a75ef84fe569/ai/aulas) | Modelfile, evidências atomizadas, Bibliotecário em Rust, captura de execução, custos de conferência e primeira comunicação Rust–Ollama |
| [Experimentos 02–26](https://github.com/ViniciusSJV/renderer/tree/a6e56ba19213d879f5de6e3e07f8a75ef84fe569/ai/experimentos) | Dados, consultas, capturas e medições de cada etapa; inclui a baseline de performance da CPU (20) |
| [Política do LLM — câmera (26)](https://github.com/ViniciusSJV/renderer/blob/a6e56ba19213d879f5de6e3e07f8a75ef84fe569/ai/experimentos/26-politica-llm-camera/README.md) | Duas consultas reais reprovadas por extrapolação; origem do contrato de afirmações |
| [Contrato dos engines v1](https://github.com/ViniciusSJV/renderer/blob/a6e56ba19213d879f5de6e3e07f8a75ef84fe569/ai/contratos/engines-v1.md) | Doze critérios para distinguir implementação, transporte e avaliação |
| [Acervos](https://github.com/ViniciusSJV/renderer/tree/a6e56ba19213d879f5de6e3e07f8a75ef84fe569/ai/acervo) | `renderer-v1` (dossiê manual), `renderer-auto-v1` (primeira edition automática) e `renderer-search-v1` |
| [TESTME manual](https://github.com/ViniciusSJV/renderer/blob/a6e56ba19213d879f5de6e3e07f8a75ef84fe569/TESTME.md) | Roteiro PowerShell passo a passo, anterior ao PLAYME genérico |
| [Ferramentas antigas](https://github.com/ViniciusSJV/renderer/tree/a6e56ba19213d879f5de6e3e07f8a75ef84fe569/src/bin) | `validate_evidence`, `explain_evidence`, `capture_execution` (Unix), `bench_evidence`, `catalog_sources` |

Neste repositório, a extração do LLM Engine a partir do renderer (27/09/2026), com testes,
conferência da query preservada por SHA-256 e limites, está registrada no
[README do crate em `8b3994b`](https://github.com/ViniciusSJV/librarian/blob/8b3994b/crates/librarian-llm-engine/README.md#extração-e-continuidade).

Em 20/10/2026 o PLAYME (`tools/playme/PLAYME.ps1` e os atalhos `PLAYME.cmd`/`PLAYME.ps1`
do renderer) foi substituído pelo [Librarian Studio](STUDIO.md). O último commit com ele é
o [`41f447e`](https://github.com/ViniciusSJV/librarian/tree/41f447e/tools/playme); as
execuções antigas ficaram em `ai/consultas/` do renderer, fora do Git.

O caminho atual está no [STUDIO](STUDIO.md) e no [WORKFLOW](WORKFLOW.md), e o roteiro de evolução no
[plano](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md). A captura de execução será refeita no
Librarian, de forma genérica e multiplataforma, quando a fase de performance começar.
