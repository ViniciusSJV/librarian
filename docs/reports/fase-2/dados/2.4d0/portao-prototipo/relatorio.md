# Portão de precisão da Fase 2

Binário: C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\a02a19ed-2252-4414-a09c-e12d986d249f\scratchpad\bin-depois (usado como estava, sem compilar); fontes desconhecidas (binário de outra pasta); identidade unknown; sha256 librarian_studio 62152b2fc3b981a3b6111a50ec5685af7c65a2c29d395ffd6b572a17ffefea46, librarian_ingest 43a3e2c7f0ba0005ec560bb0e92dc04efc58796177826b69eb021f06b3ef82b8 (só registro: muda a cada link). Gerado em 2026-11-25T07:13:46.343Z.

## Amostra (contagens reais)

| | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato | Não encontradas |
|---|---|---|---|---|---|---|
| Python (Healthchecks) (157) | 86 (+1) | 1 | 14 | 8 (-1) | 48 | 0 |
| Rust (ray tracer) (70) | 44 | 0 | 1 | 10 | 15 | 0 |

A tabela acima compara só as chamadas que a linha de base tem. A amostra inteira, com os estratos novos:

| | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato | Não encontradas |
|---|---|---|---|---|---|---|
| Python (Healthchecks) (171) | 96 | 1 | 15 | 8 | 51 | 0 |
- Python (Healthchecks), estrato novo classe_pelo_nome (6): correta 6
- Python (Healthchecks), estrato novo classe_pelo_nome_unica (3): correta 3
- Python (Healthchecks), estrato novo classe_pelo_nome_fora (3): sem_candidato 2, candidatos_falsos 1
- Python (Healthchecks), estrato novo classe_pelo_nome_atributo (2): sem_candidato 1, correta 1

Parte C (as 30 originais, dentro da amostra):

- Python (Healthchecks): sem_candidato 16, correta 4
- Rust (ray tracer): correta 3, sem_candidato 6, ambigua 1

## Estimativa para a população (ponderada pelo peso de cada estrato; é **estimativa**, não contagem)

| | População | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato |
|---|---|---|---|---|---|---|
| Python (Healthchecks) | 1956 | ~356 | ~31 | ~120 | ~0 | ~1342 |
| Rust (ray tracer) | 515 | ~161 | ~0 | ~0 | ~31 | ~323 |

## População de chamadas obj.metodo() (contagem real)

| | Total | Sem candidato | Única | Homônimo | Acima do teto | Receptor | Outras | Método → função solta |
|---|---|---|---|---|---|---|---|---|
| Python (Healthchecks) | 1956 | 1313 | 325 | 187 | 24 | 0 | type 91, import 16 | 30 |
| Rust (ray tracer) | 515 | 323 | 94 | 35 | 3 | 0 | type 53, trait_default 7 | 0 |

## Chamadas da amostra que mudaram de categoria

- HC-156: ambigua → correta (type (2) → Telegram.send (hc/integrations/telegram/transport.py:55))

## Regra da DEC-40 contra a anterior

Chamadas classificadas pelas duas regras: 241, das quais 227 estão na linha de base. Com veredito diferente:

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
| N5-py | homonym (2) | type (2) | Base.herdado (pkg/models.py:25) [type] | 2.4 | no alvo |
| N5-py-self | homonym (2) | type (2) | Base.herdado (pkg/models.py:25) [type] | 2.4 | no alvo |
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
| N5-py-cadeia | homonym (2) | type (2) | Raiz.topo (pkg/heranca_apoio.py:5) [type] | 2.4b | no alvo |
| N5-py-generica | homonym (2) | type (2) | Raiz.topo (pkg/heranca_apoio.py:5) [type] | 2.4b | no alvo |
| N5-py-apelido | homonym (2) | type (2) | Raiz.topo (pkg/heranca_apoio.py:5) [type] | 2.4b | no alvo |
| N5-py-construtor | homonym (2) | type (2) | Raiz.topo (pkg/heranca_apoio.py:5) [type] | 2.4b | no alvo |
| N5-py-fora-depois | homonym (2) | type (2) | Raiz.topo (pkg/heranca_apoio.py:5) [type] | 2.4b | no alvo |
| N5-py-fora-antes | homonym (2) | homonym (1) | sem tipo [homonym] | 2.4b | mudou |
| N5-py-diamante | above_candidate_limit (9) | type (9) | DC.mm (pkg/heranca.py:64) [type] | 2.4b | no alvo |
| N5-py-sobrescrita | type (3) | homonym (2) | sem tipo [homonym] | 2.4c | mudou |
| N5-py-atributo | homonym (2) | homonym (2) | sem tipo [homonym] | 2.4b | igual ao atual |
| N5-py-instancia | homonym (2) | homonym (2) | sem tipo [homonym] | 2.4b | igual ao atual |
| N5-py-getattr | homonym (2) | homonym (2) | sem tipo [homonym] | 2.4b | igual ao atual |
| N5-py-condicional | homonym (2) | homonym (2) | sem tipo [homonym] | 2.4b | igual ao atual |
| N5-py-derruba | unique (1) | no_candidate_in_project (0) | sem tipo [no_candidate_in_project] | 2.4b | mudou |
| N5-py-so-fora | homonym (2) | no_candidate_in_project (0) | sem tipo [no_candidate_in_project] | 2.4b | mudou |
| N5-py-super | homonym (3) | type (3) | Raiz.passo (pkg/heranca_apoio.py:8) [type] | 2.4b | no alvo |
| N5-py-super-args | homonym (3) | type (3) | Raiz.passo (pkg/heranca_apoio.py:8) [type] | 2.4b | no alvo |
| N5-py-super-coop | above_candidate_limit (9) | above_candidate_limit (9) | sem tipo [above_candidate_limit] | 2.4b | igual ao atual |
| N5-py-super-mixin | above_candidate_limit (9) | above_candidate_limit (9) | sem tipo [above_candidate_limit] | 2.4b | igual ao atual |
| N5-py-mixin | no_candidate_in_project (0) | homonym (1) | sem tipo [homonym] | 2.4b | mudou |
| N5-py-cls | homonym (2) | type (2) | Raiz.criar (pkg/heranca_apoio.py:11) [type] | 2.4b | no alvo |
| N5-py-prop | homonym (3) | homonym (2) | sem tipo [homonym] | 2.4c | mudou |
| N5-py-prop-herdada | homonym (3) | homonym (2) | sem tipo [homonym] | 2.4c | mudou |
| N5-py-prop-sem-tipo | homonym (3) | homonym (3) | sem tipo [homonym] | 2.4b | igual ao atual |
| N5-py-abstrato | type (5) | homonym (2) | sem tipo [homonym] | 2.4b | mudou |
| N5-py-abstrato-meio | type (5) | type (5) | EnvioA.mandar (pkg/heranca_regras.py:21) [type] | 2.4b | no alvo |
| N5-py-abstrato-construtor | type (5) | type (5) | Envio.mandar (pkg/heranca_regras.py:9) [type] | 2.4b | no alvo |
| N5-py-abstrato-super | homonym (3) | type (3) | Envio.vazio (pkg/heranca_regras.py:13) [type] | 2.4b | no alvo |
| N5-py-abc | type (5) | homonym (1) | sem tipo [homonym] | 2.4b | mudou |
| N5-py-base-dinamica | above_candidate_limit (9) | above_candidate_limit (9) | sem tipo [above_candidate_limit] | 2.4b | igual ao atual |
| N5-py-meta | above_candidate_limit (9) | above_candidate_limit (9) | sem tipo [above_candidate_limit] | 2.4b | igual ao atual |
| N5-py-decorador | above_candidate_limit (9) | above_candidate_limit (9) | sem tipo [above_candidate_limit] | 2.4b | igual ao atual |
| DP-py-parametro | type (4) | homonym (4) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-self | receiver (4) | homonym (4) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-cls | type (2) | homonym (2) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-retorno | type (4) | homonym (4) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-campo | type (4) | homonym (4) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-prop | type (4) | homonym (4) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-anotado | type (4) | homonym (4) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-abstrato | homonym (2) | homonym (2) | sem tipo [homonym] | 2.4c | igual ao atual |
| DP-py-abstrato-meio | type (4) | homonym (2) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-mistura | type (2) | homonym (2) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-mistura-self | receiver (2) | homonym (2) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-new | type (2) | homonym (2) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-atributo | unique (1) | homonym (1) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-privado | receiver (2) | receiver (2) | Veiculo.__motor (pkg/despacho.py:16) [receiver] | 2.4c | no alvo |
| DP-py-meio | type (4) | type (4) | Carro.andar (pkg/despacho.py:41) [type] | 2.4c | no alvo |
| DP-py-irma | type (4) | type (4) | Moto.andar (pkg/despacho.py:60) [type] | 2.4c | no alvo |
| DP-py-sem-sobrescrita | unique (1) | unique (1) | Veiculo.parar (pkg/despacho.py:13) [unique] | 2.4c | no alvo |
| DP-py-construtor | type (4) | type (4) | Veiculo.andar (pkg/despacho.py:10) [type] | 2.4c | no alvo |
| DP-py-construtor-direto | type (4) | type (4) | Veiculo.andar (pkg/despacho.py:10) [type] | 2.4c | no alvo |
| DP-py-super | type (4) | type (4) | Veiculo.andar (pkg/despacho.py:10) [type] | 2.4c | no alvo |
| DP-py-classe | homonym (4) | type (4) | sem tipo [homonym] | 2.4c | mudou |
| DP-py-fora-antes | unique (1) | unique (1) | Motor.girar (pkg/despacho.py:99) [unique] | 2.4c | no alvo |

## Auditoria dos fatos F1–F26 (Parte E)

| Não depende | Correto | Suspeito | Errado | Não encontrado |
|---|---|---|---|---|
| 15 | 9 | 1 | 1 | 0 |

- correto: F6, F7, F8, F9, F10, F13, F16, F17, F21
- suspeito: F23
- errado: F1

## Editions antigas e .studio original

- healthchecks-25597e41/20261118-061046: verify-graph ok, arquivos iguais; grafo do run novo diferente ao antigo
- renderer-9b78f3a9/20261118-020654: verify-graph ok, arquivos iguais; grafo do run novo diferente ao antigo
- .studio original: inalterado (healthchecks-25597e41: 932 arquivos, 61cd25c10688…; renderer-9b78f3a9: 42 arquivos, 612cdbf65e15…)

## Regra do portão

Reprova se, em Python ou em Rust: link_falso aumentar; corretas diminuírem; uma chamada da amostra sumir do run novo; uma edition antiga falhar no verify-graph ou mudar; ou o .studio original mudar. candidatos_falsos, ambíguas, a população, os níveis e a auditoria dos fatos são registrados com a variação. Uma exceção só vale com justificativa caso a caso e aprovação do usuário.

**APROVADO** contra a linha de base.
