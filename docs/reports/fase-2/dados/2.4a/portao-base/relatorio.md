# Portão de precisão da Fase 2

Binário: c:\Users\beatl\Documents\Projetos\librarian\target\release (compilado agora); fontes 5d9aae37baab0f94c934e28625f028718ce008c014ab80a78679b2c77e5fdb55 (238 arquivos: crates/ sem os .md, Cargo.toml, Cargo.lock e o que um .rs embute de fora de crates/); identidade recorded:49fcfb2bd8d9d5dd30af12e520f8eac83801a46b:true; sha256 librarian_studio 58246c5f7b779ff2daf143ee629aaeda3764709242aff9a065ffd930204bf792, librarian_ingest ecf7a411050e828c298c92af92bfd3bca136f02418f7750f3ce437e7389c83d6 (só registro: muda a cada link). Gerado em 2026-11-25T01:10:28.872Z.

## Amostra (contagens reais)

| | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato | Não encontradas |
|---|---|---|---|---|---|---|
| Python (Healthchecks) (123) | 60 | 1 | 14 | 5 | 43 | 0 |
| Rust (ray tracer) (70) | 44 | 0 | 1 | 10 | 15 | 0 |

Parte C (as 30 originais, dentro da amostra):

- Python (Healthchecks): sem_candidato 16, correta 4
- Rust (ray tracer): correta 3, sem_candidato 6, ambigua 1

## Estimativa para a população (ponderada pelo peso de cada estrato; é **estimativa**, não contagem)

| | População | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato |
|---|---|---|---|---|---|---|
| Python (Healthchecks) | 1956 | ~364 | ~31 | ~132 | ~0 | ~1344 |
| Rust (ray tracer) | 515 | ~161 | ~0 | ~0 | ~31 | ~323 |

## População de chamadas obj.metodo() (contagem real)

| | Total | Sem candidato | Única | Homônimo | Acima do teto | Receptor | Outras | Método → função solta |
|---|---|---|---|---|---|---|---|---|
| Python (Healthchecks) | 1956 | 1313 | 325 | 206 | 27 | 0 | type 69, import 16 | 30 |
| Rust (ray tracer) | 515 | 323 | 94 | 35 | 3 | 0 | type 53, trait_default 7 | 0 |

## Chamadas da amostra que mudaram de categoria

Nenhuma.

## Regra da DEC-40 contra a anterior

Chamadas classificadas pelas duas regras: 193, das quais 193 estão na linha de base. Com veredito diferente:

Nenhuma.

## Executor (DEC-40; registrado, não reprova)

| Id | Executor | Dono no grafo | O /api/calls do executor mostra a chamada |
|---|---|---|---|
| HC-94 | TestServer.__init__ (hc/api/tests/test_smtpd.py:48) | TestServer.stdout | sim |
| HC-95 | Command.__init__ (hc/api/management/commands/sendalerts.py:61) | Command.executor | sim |
| HC-96 | CreateHelper.__init__ (hc/lib/webauthn.py:18) | CreateHelper.server | sim |
| HC-97 | GetHelper.__init__ (hc/lib/webauthn.py:53) | GetHelper.credentials | sim |

## Níveis N0–N8 (projetos sintéticos)

| Caso | Atual registrado | Agora | Alvo da Fase 2 | Passo | Estado |
|---|---|---|---|---|---|
| N0-py-livre | homonym (2) | import (2) | livre (pkg/models.py:63) [import] | 2.4 | no alvo |
| N0-py-unica | unique (1) | unique (1) | unica (pkg/models.py:67) [unique] | já correto | no alvo (já era) |
| N1-py-construtor | unique (1) | unique (1) | Cliente (pkg/models.py:1) [instancia] | 2.3 | no alvo |
| N1-py | homonym (4) | type (4) | Cliente.salvar (pkg/models.py:5) [type] | 2.3 | no alvo |
| N2-py | homonym (4) | type (4) | Cliente.salvar (pkg/models.py:5) [type] | 2.3 | no alvo |
| N3-py | homonym (4) | type (4) | Cliente.salvar (pkg/models.py:5) [type] | 2.3 | no alvo |
| N4-py | homonym (4) | type (4) | Repo.salvar (pkg/models.py:15) [type] | 2.3 | no alvo |
| N4-py-dono | ocorrência não encontrada (0) | unique (1) | Repo (pkg/models.py:14) [instancia] | 2.3b | no alvo |
| N5-py | homonym (2) | homonym (2) | Base.herdado (pkg/models.py:25) [type] | 2.4 | igual ao atual |
| N5-py-self | homonym (2) | homonym (2) | Base.herdado (pkg/models.py:25) [type] | 2.4 | igual ao atual |
| N6-py | homonym (5) | homonym (4) | Canal.notify (pkg/models.py:40) [type] | 2.6 | mudou |
| N7-py | no_candidate_in_project (0) | no_candidate_in_project (0) | notify (pkg/casos.py:42) [passa_referencia] | 2.5 | igual ao atual |
| N8-py-construtor | no_candidate_in_project (0) | import (0) | Cliente (pkg/models.py:1) [instancia] | 2.4 | no alvo |
| N8-py | homonym (4) | type (4) | Cliente.salvar (pkg/models.py:5) [type] | 2.4 | no alvo |
| N0-rs-livre | homonym (2) | import (2) | livre (src/models.rs:78) [import] | 2.4 | no alvo |
| N0-rs-unica | unique (1) | unique (1) | unica (src/models.rs:82) [unique] | já correto | no alvo (já era) |
| N1-rs-construtor | homonym (3) | type (3) | impl Cliente::new (src/models.rs:6) [type] | 2.1 | no alvo |
| N1-rs | homonym (3) | type (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.1 | no alvo |
| N2-rs | homonym (3) | type (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.1 | no alvo |
| N3-rs | homonym (3) | type (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.3 | no alvo |
| N3b-rs | homonym (3) | type (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.1 | no alvo |
| N4-rs | homonym (3) | type (3) | impl Repo::salvar (src/models.rs:34) [type] | 2.1 | no alvo |
| N5-rs | homonym (4) | type (4) | impl Notifica for Email::notify (src/models.rs:51) [type] | 2.1 | no alvo |
| N5-rs-padrao | homonym (2) | trait_default (2) | Notifica::padrao (src/models.rs:42) [trait_default] | 2.1 | no alvo |
| N6-rs | homonym (4) | homonym (4) | impl Notifica for Email::notify (src/models.rs:51) [type] | 2.6 | igual ao atual |
| N6b-rs | homonym (4) | homonym (4) | Notifica::notify (src/models.rs:40) [type] | 2.6 | igual ao atual |
| N7-rs | no_candidate_in_project (0) | no_candidate_in_project (0) | notify_one (src/casos.rs:62) [passa_referencia] | 2.5 | igual ao atual |
| N8-rs-construtor | homonym (3) | type (3) | impl Cliente::new (src/models.rs:6) [type] | 2.4 | no alvo |
| N8-rs | homonym (3) | type (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.4 | no alvo |
| N3c-py-mesmo | homonym (2) | type (2) | Pedido.fechar (pkg/retorno.py:7) [type] | 2.3c | no alvo |
| N3c-py-metodo | homonym (2) | type (2) | Pedido.fechar (pkg/retorno.py:7) [type] | 2.3c | no alvo |
| N3c-py-opcional | homonym (2) | type (2) | Pedido.fechar (pkg/retorno.py:7) [type] | 2.3c | no alvo |
| N3c-py-campo | homonym (2) | type (2) | Pedido.fechar (pkg/retorno.py:7) [type] | 2.3c | no alvo |
| N3c-py-fora | homonym (2) | homonym (2) | sem tipo [homonym] | 2.3c | igual ao atual |
| N3c-py-generico | homonym (2) | homonym (2) | sem tipo [homonym] | 2.3c | igual ao atual |
| N3c-py-sem-anotacao | homonym (2) | homonym (2) | sem tipo [homonym] | 2.3c | igual ao atual |
| N3c-py-dois-alvos | homonym (2) | type (2) | Pedido.fechar (pkg/retorno.py:7) [type] | 2.4 | no alvo |
| N3c-py-cadeia | homonym (2) | homonym (2) | sem tipo [homonym] | 2.3c | igual ao atual |
| N3c-py-laco | homonym (2) | homonym (2) | sem tipo [homonym] | 2.3c | igual ao atual |
| N3c-rs-outro | type (2) | type (2) | impl Nota::fechar (src/retorno.rs:37) [type] | 2.3c | no alvo |
| N3c-rs-option | type (2) | homonym (2) | sem tipo [homonym] | 2.3c | mudou |
| N3c-rs-box | type (2) | homonym (2) | sem tipo [homonym] | 2.3c | mudou |
| N3c-rs-ref | homonym (2) | type (2) | impl Pedido::fechar (src/retorno.rs:25) [type] | 2.3c | no alvo |
| N3c-rs-fora | unique (1) | no_candidate_in_project (0) | sem tipo [no_candidate_in_project] | 2.3c | mudou |
| N3c-rs-concordam | type (2) | type (2) | impl Pedido::fechar (src/retorno.rs:25) [type] | 2.3c | no alvo |
| N3c-rs-discordam | type (2) | type (2) | impl Pedido::fechar (src/retorno.rs:25) [type] | 2.4 | no alvo |
| N3c-rs-q | homonym (2) | homonym (2) | sem tipo [homonym] | 2.3c | igual ao atual |
| N0-py-modulo | homonym (2) | scope (2) | esc_modulo (pkg/escopo.py:13) [scope] | 2.4a | no alvo |
| N0-py-fora | unique (1) | no_candidate_in_project (0) | sem tipo [no_candidate_in_project] | 2.4a | mudou |
| N0-py-local | unique (1) | no_candidate_in_project (0) | sem tipo [no_candidate_in_project] | 2.4a | mudou |
| N0-py-overload | homonym (3) | scope (3) | esc_sobre (pkg/escopo.py:25) [scope] | 2.4a | no alvo |
| N0-py-condicional | homonym (3) | homonym (3) | sem tipo [homonym] | 2.4a | igual ao atual |
| N0-py-typechecking | unique (1) | unique (1) | so_tipo (pkg/escopo_apoio.py:13) [unique] | 2.4a | no alvo |
| N0-py-estrela | homonym (2) | homonym (2) | sem tipo [homonym] | 2.4a | igual ao atual |
| N0-py-reexport | homonym (2) | import (2) | reexp (pkg/sub/base.py:1) [import] | 2.4a | no alvo |
| N8-py-tipo | homonym (4) | type (4) | Cliente.salvar (pkg/models.py:5) [type] | 2.4a | no alvo |
| N8-py-local | homonym (4) | type (4) | Fornecedor.salvar (pkg/models.py:10) [type] | 2.4a | no alvo |
| N8-py-typealias | homonym (4) | type (4) | Fornecedor.salvar (pkg/models.py:10) [type] | 2.4a | no alvo |
| N8-py-fora | homonym (4) | homonym (4) | sem tipo [homonym] | 2.4a | igual ao atual |
| N0-rs-modulo | homonym (2) | scope (2) | esc_livre (src/escopo.rs:4) [scope] | 2.4a | no alvo |
| N0-rs-bloco | homonym (2) | scope (2) | e_bloco::esc_bloco (src/escopo.rs:13) [scope] | 2.4a | no alvo |
| N0-rs-glob | homonym (2) | import (2) | esc_livre (src/escopo.rs:4) [import] | 2.4a | no alvo |
| N0-rs-glob-fora | homonym (2) | homonym (2) | sem tipo [homonym] | 2.4a | igual ao atual |
| N8-rs-tipo | homonym (3) | type (3) | impl Cliente::salvar (src/models.rs:10) [type] | 2.4a | no alvo |

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
