# Arrumação das baterias de tela antigas: Parada 2

25/11/2026. Implementa a [Parada 1](baterias-parada-1.md), aprovada com as respostas P1 a P7. Não é passo da Fase 2: nenhuma regra nem código de produção mudou. Nada commitado; sem push.

**[M]** medido nesta sessão, rodando. **[L]** leitura do agente.

## Resultado

| O quê | Resultado |
|---|---|
| `node tools/precisao/telas.js` | **saída 0**: 12 baterias, 1.028 verificações rodadas, 1.028 passam, 0 falham, 15,6 min **[M]** |
| `cargo test --workspace --locked` | **557 aprovados, 0 falhas, 2 ignorados**, saída 0 **[M]**; igual à linha de base |
| Portão sem `--fontes` | **APROVADO**, saída 0 **[M]**. Python (171) 96/1/15/8/51, Rust (70) 44/0/1/10/15, fatos 15/9/1/1, invariante do `used_by` com 0 exceções (Python 2.156 links, Rust 868). O `resultado.json` é igual à `linha-de-base.json` campo a campo, salvo os ids dos dois runs novos |
| Hash das fontes | Antes: `3aeb2fa77adc9ec44716bc113822faa14395b71cb7f7217deb46b4e2b87c4726` (250 arquivos). Depois: o mesmo **[M]**. Nenhum arquivo não `.md` de `crates/` mudou (P1). O `--fontes` do próximo passo sai da Parada 2 dele, como sempre |

## Cada bateria viva

| Bateria | Rodadas | Falhas | Puladas | Tempo |
|---|---|---|---|---|
| 2.1 | 51 de 51 | 0 | 0 | 32 s |
| 2.2 | 56 de 56 | 0 | 0 | 41 s |
| 2.3 | 84 de 84 | 0 | 0 | 48 s |
| 2.3b | 102 de 102 | 0 | 0 | 78 s |
| 2.4a | 88 de 88 | 0 | 0 | 46 s |
| 2.4c | 193 de 193 | 0 | 0 | 272 s |
| 2.4d-0 | 240 de 240 | 0 | 0 | 296 s |
| 2.4d | 71 de 71 | 0 | 0 | 64 s |
| g3 | 14 de 14 | 0 | 0 | 8 s |
| `tela-estado` (parte A) | 46 de 46 | 0 | parte B (1 linha "pulada", exigida) | 8 s |
| `tela-simplificacao` | 47 de 47 | 0 | 0 | 19 s |
| `tela-passo6` | 36 de 36 | 0 | 0 | 22 s |

A 2.4d, que caiu com 0 de 71 na Parada 1 (defeito do `telas.js`), roda até o fim.

## Aposentadas, uma vez

| Bateria | Saída | O que a fez parar |
|---|---|---|
| As 18 do Studio e o `rodar.js` | 2 | "Falta a variável de ambiente `LIBRARIAN_TESTE_RENDERER`" (sem o Playwright apontado, saída 1 antes, sem achar o `playwright-core`) |
| 2.4b | 1 | 38 de 98 (32 passam, 6 falham); para no `box.click()` com duas caixas `many:` |
| g3 antigo (de `HEAD`) | 1 | `ECONNREFUSED 127.0.0.1:8773` |
| Parte B da `tela-estado` | — | pulada dentro da execução viva |

Registradas em [APOSENTADAS.md](../../../crates/librarian-studio/tests/tela/APOSENTADAS.md).

## P2: as 16 que a DEC-44 tornou falsas

As 16 da 2.4b esperavam `is_noop` como link único (`type` → `Transport.is_noop`). **A extensão não as substitui pelo comportamento novo; verifica outra coisa:** as 16 de `num_checks_available` exercitam a regra da `@property` de link único (uma propriedade sem sobrescrita, que continua `type`, linha cheia, `type_rule: propriedade`).

**O comportamento novo de `is_noop`** (ambíguo, caixa própria com 14) **já era verificado pela 2.4c commitada**: a caixa `is_noop` tem 13 verificações por idioma (uma caixa com a contagem, linha tracejada, a lista das 14 com arquivo, traz `Transport.is_noop`, `homonym` com `basis: type`, `type_rule: propriedade` e 14 candidatas, o texto sem "abstrato", sem `abstract_method`, sem chave crua, clicar abre a lista, a escolha vira `chosen` para `Webhook.is_noop`, limpar volta à caixa). A extensão acrescenta 2 por idioma: "nenhuma candidata vira caixa própria" (14 candidatas, 0 desenhadas à parte) e "a caixa some e a linha fica cheia". Com o conserto, esta passou a mostrar o que achou: `{"box":false,"lines":1,"dashed":0}` nas 6 vezes (3 caixas × 2 idiomas) **[M]**.

## P6: Playwright e navegador

- `playwright-core` 1.48.2, instalado com `npm install playwright-core@1.48.2 --save-exact` em `tools/precisao` **[M]**. A integridade no `package-lock.json` (`sha512-sjjw+qrL…`) é a mesma da cópia da pasta temporária de outra sessão, que as baterias usavam até aqui. `node_modules` fica fora do Git (`.gitignore`).
- **O navegador não vem da pasta temporária:** é o Google Chrome instalado na máquina, `C:/Program Files/Google/Chrome/Application/chrome.exe` (versão 158.0.8127.37 na pasta). As baterias da Fase 2 e o g3 abrem esse executável pelo caminho; as do Studio pedem `channel: 'chrome'`, que acha o mesmo. O `playwright-core` não baixa navegador. **Proposta: fica onde está.** O `telas.js` recusa começar (saída 2) se ele não existir. Limite: o Chrome se atualiza sozinho, e a versão não é fixa; uma atualização que quebre o 1.48.2 aparece como falha do `telas.js`.

## O que mudou, arquivo por arquivo

| Arquivo | Mudança |
|---|---|
| `tools/precisao/telas.js` (novo) | O comando único (DEC-47). Sobre a versão da outra sessão: argumentos por bateria (só a `tela-passo6` recebe a pasta de saída; a 2.4d usa o `target/release` compilado); número de verificações **igual** ao registrado; recusa porta 8842 ou 8841 ocupada; tira do ambiente as variáveis das aposentadas e exige "parte B pulada" na `tela-estado`; falha se o servidor da `tela-passo6` não ficar pronto; rodadas, falhas e puladas por bateria; "PARCIAL" com `--so`; saída 2 sem Playwright, sem Chrome ou com a compilação falhando. 2.4c registrada com 193 |
| `tools/precisao/package.json`, `package-lock.json` (novos) | `playwright-core` 1.48.2, fixo; `private` |
| `.gitignore` | `/tools/precisao/node_modules/` |
| `tools/precisao/lib.js` | `playwright()`: a variável `LIBRARIAN_TESTE_PLAYWRIGHT` ou a cópia de `tools/precisao`, com o `require` preguiçoso. Sai a constante com o caminho temporário. Nenhuma função do portão mudou |
| `2.1`, `2.3b`, `2.4a`, `2.4d`, `2.4d0` (`tela-*.js`) | `require(L.playwright())`; a 2.1 grava as capturas na pasta temporária, não mais em `docs/…/2.1/tela/` |
| `2.2/tela-22.js` | As 2 verificações do texto do `import` (DEC-42), com o motivo escrito nas duas |
| `2.3/tela-23.js` | "decidida, sem seguir" na instanciação sem `__init__` (DEC-40), com o motivo |
| `2.4c/tela-24c.js`, `seeds.json` | A extensão vinda da 2.4b (149 → 193), com "a linha fica cheia" exigindo a linha |
| `piloto-1b/dados/g3/tela-g3.js` | Reescrita: Studio próprio sobre o sintético, PT e EN (14). As três ressalvas: grupo achado pelo título exato da página; pasta temporária apagada; comentário cita a DEC-38 (`a34059c`). Compila o binário antes, como as outras |
| `crates/librarian-studio/tests/tela/README.md` | Reescrito: o `telas.js` como único comando, o `rodar.js` aposentado, a regra (DEC-47), requisitos (Chrome da máquina, `npm ci`), a lista das 12 vivas. A tabela das 21 e a de 18/11 saíram (o conteúdo foi para o APOSENTADAS) |
| `crates/librarian-studio/tests/tela/APOSENTADAS.md` (novo) | O registro único: resumo, o `rodar.js` com a lista do que rodava, cada bateria com o que fica sem cobertura, e a execução de despedida |
| `tools/precisao/README.md` | A regra (DEC-47) em "Como rodar"; o limite da trava (`.js` de teste em `crates/` entram no hash); `telas.js` e `package*.json` na tabela de arquivos; `playwright()` no `lib.js` |
| `DECISOES.md` | DEC-46 movida para dentro da tabela, texto igual (conferido por hash da linha); DEC-47 nova |
| `ESTADO-ATUAL.md` | A linha "Próximo passo" na ordem pedida (estrato de 30 → Parada 1 do 2.4e → 2.4f → 2.5 → 2B → 2.6); a linha "Git" com os quatro commits do 2.4d, o push e o CI verde; o item das baterias marcado como feito, com as propostas antigas tiradas; a suíte; "recriar as baterias antigas" nos adiados do usuário atualizado |
| `docs/reports/fase-2/baterias-parada-1.md`, `baterias-parada-2.md` (novos) | As duas Paradas |

Não mudou: nenhum arquivo de `crates/` além de `.md`; o `rodar.js` e as baterias aposentadas; o portão e as fixtures; a linha de base.

## O que fica sem cobertura

Detalhe em [APOSENTADAS.md](../../../crates/librarian-studio/tests/tela/APOSENTADAS.md) **[L]**. Em resumo:
- **O fluxograma como vista da página** (`tela-centro`, `tela-inspecao`, `tela-navegacao`): barra do centro, ir e voltar preservando o estado, inspeção pelo clique, navegação entre áreas e projetos. O desenho de "o que chama?" e "onde é usado?" continua coberto pelas baterias 2.1 a 2.4d e pelo g3. Recriar é pergunta da Parada 1 da 2B.
- **As medições de 04/11** (cartão, zero, origem, nomes, ranking, sugestão, `origin.selection` na tela).
- **O resto do front antigo** (Seleção/Builder, rascunho na página, perfil e política, conversa, área Projeto, revisão e lote, pacote na tela): proposta de critério do front refeito depois do 6.5.
- **Parte B da `tela-estado`** (o estado local com servidor).
- **g3 sobre o Healthchecks**: a nota "Nenhuma chamada a desenhar" e o grupo "acima do limite" sobre dados reais.
- **2.4b**: "lista as 30" sem exigir `.notify` em cada item; o "exatamente 1 linha tracejada", que a DEC-44 tornou falso.

## Mensagem de commit proposta

```
test(telas): um comando para as baterias de tela vivas, conserto da 2.2, da 2.3 e do g3 e registro das aposentadas (DEC-47)

- tools/precisao/telas.js roda as 12 baterias vivas; falha com uma
  verificacao falha, uma bateria que nao roda ate o fim ou roda outro
  numero de verificacoes
- playwright-core 1.48.2 em tools/precisao (package.json e lockfile);
  lib.js acha o pacote sem caminho fixo
- 2.2 (DEC-42) e 2.3 (DEC-40) consertadas, com o motivo na verificacao;
  g3 sobre o projeto sintetico, com o proprio studio, em PT e EN
- 2.4c recebe o que a 2.4b cobria (149 para 193)
- aposentadas: 2.4b, as 18 do Studio, a parte B da tela-estado, a parte
  do Healthchecks do g3 e o rodar.js, em APOSENTADAS.md
- DEC-47; DEC-46 de volta para a tabela; READMEs e ESTADO-ATUAL

Nenhuma regra nem codigo de producao muda; fontes do portao iguais
(3aeb2fa7...).
```
