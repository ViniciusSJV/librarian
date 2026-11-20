# Portão de precisão da Fase 2

Binário: c:\Users\beatl\Documents\Projetos\librarian\target\release. Gerado em 2026-11-20T04:58:05.018Z.

## Amostra (contagens reais)

| | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato | Não encontradas |
|---|---|---|---|---|---|---|
| Python (Healthchecks) (87) | 38 | 1 | 12 | 3 | 33 | 0 |
| Rust (ray tracer) (65) | 39 | 0 | 1 | 10 | 15 | 0 |

Parte C (as 30 originais, dentro da amostra):

- Python (Healthchecks): sem_candidato 16, correta 4
- Rust (ray tracer): correta 3, sem_candidato 6, ambigua 1

## Estimativa para a população (ponderada pelo peso de cada estrato; é **estimativa**, não contagem)

| | População | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato |
|---|---|---|---|---|---|---|
| Python (Healthchecks) | 1956 | ~366 | ~31 | ~138 | ~0 | ~1344 |
| Rust (ray tracer) | 515 | ~161 | ~0 | ~0 | ~31 | ~323 |

## População de chamadas obj.metodo() (contagem real)

| | Total | Sem candidato | Única | Homônimo | Acima do teto | Receptor | Outras | Método → função solta |
|---|---|---|---|---|---|---|---|---|
| Python (Healthchecks) | 1956 | 1313 | 325 | 214 | 27 | 0 | type 61, import 16 | 30 |
| Rust (ray tracer) | 515 | 323 | 94 | 35 | 3 | 0 | type 53, trait_default 7 | 0 |

## Chamadas da amostra que mudaram de categoria

Nenhuma.

## Níveis N0–N8 (projetos sintéticos)

| Caso | Atual registrado | Agora | Alvo da Fase 2 | Passo | Estado |
|---|---|---|---|---|---|
| N0-py-livre | homonym (2) | homonym (2) | livre (pkg/models.py:63) [import] | a definir | igual ao atual |
| N0-py-unica | unique (1) | unique (1) | unica (pkg/models.py:67) [unique] | já correto | no alvo (já era) |
| N1-py-construtor | unique (1) | unique (1) | Cliente (pkg/models.py:1) [instancia] | 2.3 | no alvo |
| N1-py | homonym (4) | type (4) | Cliente.salvar (pkg/models.py:5) [type] | 2.3 | no alvo |
| N2-py | homonym (4) | type (4) | Cliente.salvar (pkg/models.py:5) [type] | 2.3 | no alvo |
| N3-py | homonym (4) | homonym (4) | Cliente.salvar (pkg/models.py:5) [type] | 2.3 | igual ao atual |
| N4-py | homonym (4) | type (4) | Repo.salvar (pkg/models.py:15) [type] | 2.3 | no alvo |
| N5-py | homonym (2) | homonym (2) | Base.herdado (pkg/models.py:25) [type] | 2.4 | igual ao atual |
| N5-py-self | homonym (2) | homonym (2) | Base.herdado (pkg/models.py:25) [type] | 2.4 | igual ao atual |
| N6-py | homonym (5) | homonym (4) | Canal.notify (pkg/models.py:40) [type] | 2.6 | mudou |
| N7-py | no_candidate_in_project (0) | no_candidate_in_project (0) | notify (pkg/casos.py:42) [passa_referencia] | 2.5 | igual ao atual |
| N8-py-construtor | no_candidate_in_project (0) | no_candidate_in_project (0) | Cliente (pkg/models.py:1) [instancia] | 2.4 | igual ao atual |
| N8-py | homonym (4) | homonym (4) | Cliente.salvar (pkg/models.py:5) [type] | 2.4 | igual ao atual |
| N0-rs-livre | homonym (2) | homonym (2) | livre (src/models.rs:78) [import] | a definir | igual ao atual |
| N0-rs-unica | unique (1) | unique (1) | unica (src/models.rs:82) [unique] | já correto | no alvo (já era) |
| N1-rs-construtor | homonym (3) | type (3) | impl Cliente::new (src/models.rs:6) [type] | 2.1 | no alvo |
| N1-rs | homonym (3) | type (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.1 | no alvo |
| N2-rs | homonym (3) | type (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.1 | no alvo |
| N3-rs | homonym (3) | homonym (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.3 | igual ao atual |
| N3b-rs | homonym (3) | type (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.1 | no alvo |
| N4-rs | homonym (3) | type (3) | impl Repo::salvar (src/models.rs:34) [type] | 2.1 | no alvo |
| N5-rs | homonym (4) | type (4) | impl Notifica for Email::notify (src/models.rs:51) [type] | 2.1 | no alvo |
| N5-rs-padrao | homonym (2) | trait_default (2) | Notifica::padrao (src/models.rs:42) [trait_default] | 2.1 | no alvo |
| N6-rs | homonym (4) | homonym (4) | impl Notifica for Email::notify (src/models.rs:51) [type] | 2.6 | igual ao atual |
| N6b-rs | homonym (4) | homonym (4) | Notifica::notify (src/models.rs:40) [type] | 2.6 | igual ao atual |
| N7-rs | no_candidate_in_project (0) | no_candidate_in_project (0) | notify_one (src/casos.rs:62) [passa_referencia] | 2.5 | igual ao atual |
| N8-rs-construtor | homonym (3) | homonym (3) | impl Cliente::new (src/models.rs:6) [type] | 2.4 | igual ao atual |
| N8-rs | homonym (3) | homonym (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.4 | igual ao atual |

## Auditoria dos fatos F1–F26 (Parte E)

| Não depende | Correto | Suspeito | Errado | Não encontrado |
|---|---|---|---|---|
| 16 | 9 | 0 | 1 | 0 |

- correto: F6, F7, F8, F9, F10, F13, F16, F17, F21
- suspeito: —
- errado: F1

## Editions antigas e .studio original

- healthchecks-25597e41/20261118-061046: verify-graph ok, arquivos iguais; grafo do run novo diferente ao antigo
- renderer-9b78f3a9/20261118-020654: verify-graph ok, arquivos iguais; grafo do run novo diferente ao antigo
- .studio original: inalterado (healthchecks-25597e41: 932 arquivos, 61cd25c10688…; renderer-9b78f3a9: 42 arquivos, 612cdbf65e15…)

## Regra do portão

Reprova se, em Python ou em Rust: link_falso aumentar; corretas diminuírem; uma chamada da amostra sumir do run novo; uma edition antiga falhar no verify-graph ou mudar; ou o .studio original mudar. candidatos_falsos, ambíguas, a população, os níveis e a auditoria dos fatos são registrados com a variação. Uma exceção só vale com justificativa caso a caso e aprovação do usuário.

**APROVADO** contra a linha de base.
