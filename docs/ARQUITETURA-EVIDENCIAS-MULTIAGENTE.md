# Arquitetura de evidências para múltiplos consumidores

**Direção registrada em 18/11/2026. Nada aqui está implementado.** Este documento registra para onde o Librarian evolui e por quê. A sequência das fases e o que está autorizado ficam no [plano](../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md#piloto-python-1-e-direção-multiagente-18112026). A base factual é o [Piloto Python 1 (Healthchecks)](reports/PILOTO-1-HEALTHCHECKS.md) e a extração do Evidence Builder como crate independente ([`librarian-evidence`](../crates/librarian-evidence/README.md)).

## 1. A mudança de propósito

Até aqui, o Librarian foi pensado como uma ferramenta de investigação usada por uma pessoa no Studio. A direção agora é outra:

> **O Librarian não é um agente nem um orquestrador. É uma infraestrutura independente que permite a qualquer agente ou aplicação solicitar, receber e reutilizar evidências rastreáveis para realizar seu trabalho.**

Os consumidores podem ser agentes de IA, orquestradores, aplicações externas, ferramentas especializadas e sistemas de análise e engenharia de software.

O Librarian **não** escolhe agentes, não controla a ordem das tarefas, não planeja investigações, não executa ferramentas e não decide propostas. A responsabilidade dele é **construir, verificar e fornecer pacotes de evidências rastreáveis** e preservar os fatos e suas origens.

## 2. Separação de responsabilidades

| Papel | Responsabilidades | Fica fora |
|---|---|---|
| **Orchestrator externo** | Escolhe os agentes que participam; distribui tarefas; controla iterações; gerencia dependências entre agentes; decide quando pedir novas investigações; organiza o fluxo de execução; mantém o histórico da coordenação | Montar pacotes; aprovar mudanças |
| **Agentes especializados** | Recebem objetivos; declaram as evidências de que precisam; interpretam os pacotes; formulam hipóteses; produzem propostas; pedem complementos, justificando a necessidade | Validar sozinhos o que propuseram |
| **Librarian** | Recebe pedidos de evidências; consulta estruturas e fontes por seus componentes e adaptadores; organiza os dados; monta pacotes com o Evidence Builder; confere integridade e proveniência; apresenta fontes, relações, justificativas e lacunas; preserva fatos e suas origens | Escolher agentes; orquestrar; planejar a investigação do consumidor; executar ferramentas; transformar hipóteses em fatos |
| **Ferramentas externas** | Executam comandos e testes; fazem medições, cobertura e profiling; produzem resultados observáveis, sob o controle e as aprovações do ambiente consumidor | Interpretar resultados |
| **Validador independente** | Confronta propostas com as evidências; verifica resultados; analisa regressões; usa testes e medições; não depende só do agente que fez a proposta | Decidir a aplicação |
| **Humano** | Aprova ou rejeita mudanças; decide conflitos; define quando uma proposta é aplicada | — |

Dentro do Librarian, a divisão continua como hoje: o Graph Engine identifica estruturas e relações; o Ingest prepara e organiza; o Evidence Builder monta e confere pacotes; o LLM Engine interage com modelos quando um consumidor do próprio Librarian (o Studio) pede; o Studio coordena a investigação de uma pessoa e apresenta.

## 3. Pacotes pela necessidade do consumidor

Não haverá pacotes fixos por perfil de agente. O conteúdo depende do **objetivo** e das **evidências necessárias**, que o consumidor declara. Exemplos:

| Projeto | Consumidor | Pede |
|---|---|---|
| API Python | DDD | Responsabilidades, relações, dependências |
| API Python | Performance | Caminhos, laços, consultas, medições |
| API Python | Segurança | Entradas, permissões, mecanismos de autenticação |
| Ray tracer em Rust | Performance | Cadeias de execução, implementações de transformações, medições |
| Ray tracer em Rust | Validação | Testes, resultados, hashes de imagens, evidências de comportamento |

O Librarian monta o pacote com os dados e as operações disponíveis. **Nem todo tipo de evidência estará sempre disponível.** Quando algo não puder ser localizado ou comprovado, a ausência aparece explicitamente no pacote. O piloto mostrou o risco do contrário: `/api/calls` omitia chamadas ambíguas sem avisar (G3 no relatório do piloto).

## 4. Ciclo de complementação

1. O agente pede evidências.
2. O Librarian constrói e confere o pacote.
3. O agente interpreta e produz uma hipótese.
4. O agente identifica uma insuficiência.
5. Pede evidências adicionais, justificando a necessidade.
6. O Librarian produz um pacote complementar.
7. O agente continua a investigação.

O ciclo pode se repetir. **O Orchestrator controla quando e como as iterações acontecem. O Librarian não decide quando a investigação terminou.** A proteção da escada de 18/11 já aplica esse princípio dentro do Studio: nenhuma rodada pede um complemento por conta própria.

## 5. Contrato público e independência de linguagem

A implementação interna continua em Rust. Agentes e orquestradores não precisam ser escritos em Rust. Direção futura:

- interface pública independente de linguagem;
- contratos versionados;
- comunicação inicial por HTTP/JSON;
- operações: pedir evidências, receber o pacote, pedir complementos;
- identificação do consumidor;
- rastreabilidade por identificadores e hashes;
- SDKs opcionais para as linguagens mais usadas.

Os consumidores **não** precisam importar crates Rust nem conhecer estruturas internas como `BuildInput`, `TurnFiles` e `QuestionRecord`. A interface pública será um **adaptador sobre os componentes internos**, e não uma exposição deles.

## 6. Agentes e orquestradores externos

**O Librarian não terá um orquestrador obrigatório no núcleo.** Cada consumidor pode implementar o seu. Por exemplo:

- uma aplicação Python pode usar LangGraph;
- uma aplicação Rust pode ter coordenação própria;
- outra pessoa pode integrar um framework diferente;
- um agente pode consumir diretamente o serviço de evidências.

A linguagem e o framework dos agentes não são dependência arquitetural do Librarian. O primeiro protótipo de orquestração pode ser em Python como experimento; isso não é uma obrigação para os consumidores. **Nenhum framework de agentes (LangGraph, CrewAI ou outro) entra nos crates centrais.**

## 7. Extratores e linguagens

O piloto mostrou que o extrator Python já ingere um projeto real (657 arquivos, 4.718 símbolos, nenhum diagnóstico), mas tem limitações importantes nas relações: não registra imports, resolve só por nome e não vê despacho dinâmico. A direção é preservar e evoluir os extratores Python e Rust e, mais tarde, um extrator Java. Cada extrator deve produzir informação compatível com uma **representação intermediária comum**, que preserve:

- linguagem de origem;
- arquivos e posições;
- símbolos e declarações;
- imports;
- ocorrências de chamadas;
- candidatos de resolução e ambiguidades;
- evidências estruturais;
- particularidades semânticas da linguagem, como a diferença entre a linha do decorador e a linha do `def`, que causou o P1 do piloto.

O Java fica para uma fase própria. Antes, é preciso consolidar os contratos e corrigir as limitações demonstradas em Python sem prejudicar o Rust.

## 8. Papel do Studio

O Studio continua importante, mas **não define os limites de uso do Librarian**. É um consumidor entre outros. O foco principal dele passa a ser mostrar os workflows das aplicações:

- origem dos workflows;
- caminhos entre endpoints, funções e módulos;
- decisões e condições;
- persistência, inclusive relações mediadas pelo banco, como `Flip(processed=None)` no piloto;
- relações entre componentes;
- evidências de cada etapa;
- fatos verificados;
- pacotes de evidências.

Medição, cobertura, execução de testes, hotspots e aprovação de comandos **continuam como capacidades auxiliares**. Não precisam ocupar o centro da navegação e **não serão removidas**. No futuro, agentes especializados poderão usá-las, sempre sob as permissões do ambiente consumidor.

**Execução hoje e o princípio 4.** Hoje o Studio roda comandos do perfil do projeto, com aprovação por comando exato ("Medir tudo", Régua, portão da escada). Isso continua assim. O princípio de que o Librarian não executa ferramentas externas vale para a **infraestrutura de evidências**: o Evidence Builder já não roda comandos (`tests/independence.rs`), e a interface pública futura também não vai rodar. No Studio, a execução é capacidade de um consumidor, controlada pela pessoa. O Builder recebe só a referência e a leitura da medição.

## 9. Autoria e proveniência dos fatos

O contrato atual de fatos não distingue quem criou, analisou, revisou e aprovou. Todo fato grava `authorship: "revisão humana no Librarian Studio"`, e no piloto isso descreveu de forma incorreta 26 fatos revisados pelo agente. A evolução precisa separar:

- o agente que identificou o candidato;
- o agente que elaborou a afirmação;
- o agente ou processo que fez a revisão semântica;
- a verificação mecânica;
- a confirmação por teste;
- a pessoa que aprovou, quando houver;
- a pessoa ou o agente que revogou uma decisão.

**Revisão por agente não é aprovação humana.**

Decisão futura sobre **promoção por item**: um item correto não deveria ser perdido só porque outro item da mesma resposta foi reprovado, desde que cada item tenha passado pelos próprios critérios de verificação.

O formato atual dos fatos **não muda agora**. A evolução do contrato e o formato 3 (proposto no README do `librarian-evidence`) precisam de especificação própria, compatibilidade com os formatos 1 e 2 e testes.

## 10. Rastreabilidade de uma investigação com vários agentes

Requisito futuro: deve ser possível reconstruir a trilha completa.

```text
Solicitação → Consumidor → Objetivo → Evidências → Pacote → Interpretação → Proposta
            → Execução → Medição → Validação → Decisão
```

Quando aplicável, a trilha identifica:

- o agente que pediu o pacote e o objetivo;
- as evidências pedidas, as incluídas e as que ficaram de fora, com o motivo;
- o pacote entregue;
- a hipótese e a proposta;
- a ferramenta executada e os resultados obtidos;
- quem validou e quem tomou a decisão final.

Essas responsabilidades **não ficam todas no Evidence Builder**:

- **O Orchestrator externo** guarda o histórico da coordenação.
- **O Librarian** guarda a proveniência dos dados e dos pacotes que produziu.

O formato 3 pode completar as identidades e os vínculos que faltam, como quem pediu, o objetivo e o hash de cada envio citado, mas não será implementado agora.

## 11. Princípios permanentes

1. O Librarian não escolhe os agentes.
2. O Librarian não orquestra os agentes.
3. O Librarian não planeja a investigação do consumidor.
4. O Librarian não executa ferramentas externas.
5. O Librarian não transforma hipóteses em fatos.
6. O agente declara as evidências de que precisa.
7. O Evidence Builder monta e verifica pacotes.
8. A seleção deve ser justificável e rastreável.
9. A ausência de evidência deve ser explícita.
10. Revisão por agente não é aprovação humana.
11. Quem propõe não deve ser o único responsável por validar.
12. A decisão de aplicar mudanças continua humana.
13. Diferentes linguagens e frameworks devem poder consumir o Librarian.
14. O extrator deve evoluir sem tornar o núcleo dependente de uma linguagem específica.

*Condensar fatos a partir de vapores de nuances.*
