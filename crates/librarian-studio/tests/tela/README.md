# Baterias de tela

Verificações na página real, num Chrome sem janela, contra o servidor real. Cobrem o que os testes em Rust não
alcançam: o que a tela diz, o desenho, os cliques. Ficam fora do `cargo test` e do CI: o cargo não executa
arquivos `.js`, e as baterias precisam de um navegador e, as da Fase 2, dos projetos de teste e do `.studio`.

## Como rodar

**Um comando só, para todas as baterias vivas** (desta pasta e as dos passos da Fase 2, em `docs/reports/`):

```
npm ci --prefix tools/precisao        # uma vez: instala o playwright-core da versão do package-lock.json
node tools/precisao/telas.js [--so <nome>] [--saida <pasta>]
```

- Compila o `librarian_studio` (como o portão) e roda as baterias uma a uma, cada uma com a pasta de saída própria.
- **Código de saída:** 0 se todas passarem; 1 se uma bateria falhar; 2 se não conseguir começar (sem Playwright, sem
  Chrome, compilação com erro). Uma bateria falha se uma verificação falha, se a saída dela não é 0 (caiu no meio,
  tempo esgotado), se roda outro número de verificações que o registrado no `telas.js`, ou se a porta que ela usa já
  está ocupada. Com `--so`, o resultado sai marcado como PARCIAL.
- **Regra de processo ([DEC-47](../../../../DECISOES.md)):** toda Parada 2 roda todas as baterias vivas por este
  comando, e uma falha conta como falha. Uma verificação que um passo torna falsa é corrigida nele, com o motivo
  escrito nela citando a DEC; aposentar uma bateria pede aprovação do usuário e entra em [APOSENTADAS.md](APOSENTADAS.md).

**O `rodar.js` desta pasta está aposentado** desde 25/11/2026: rodava as 20 baterias de 04/11, 18 delas aposentadas.
A lista do que ele roda está em [APOSENTADAS.md](APOSENTADAS.md). Não foi apagado nem alterado, porque os `.js` desta
pasta entram no hash das fontes do portão.

**Requisitos:** Node 18 ou mais novo; Google Chrome instalado na máquina (`C:/Program Files/Google/Chrome`, o
mesmo que as baterias abrem; não é baixado pelo Playwright); `playwright-core` 1.48.2, de
`tools/precisao/package-lock.json` (`node_modules` fora do Git). A variável `LIBRARIAN_TESTE_PLAYWRIGHT`, se
existir, aponta outro `playwright-core`. As da Fase 2 pedem também o `renderer` e o Healthchecks nos commits do
portão e o `.studio` com os dois projetos (só lido: cada bateria trabalha numa cópia e confere o original).

**Limite: o Chrome não tem versão fixa.** Ele se atualiza sozinho (158.0.8127.37 em 25/11/2026), e só o
`playwright-core` está fixo. Uma atualização que quebre o 1.48.2 aparece como falha do `telas.js`, não como falha
do Studio: antes de procurar o defeito na página, confira a versão do Chrome.

## Baterias vivas

O número é o de verificações registrado no `telas.js` (última execução aprovada).

| Bateria | Arquivo | Verificações | O que protege |
|---|---|---|---|
| 2.1 | `docs/reports/fase-2/dados/2.1/tela-21.js` | 51 | "O que chama?": os textos de `type`, `trait_default` e `receiver` (passo 2.1) |
| 2.2 | `…/2.2/tela-22.js` | 56 | Os textos de `import`, `unique`, `homonym` com um candidato admitido, o receptor acima do limite e o motivo de `no_candidate_in_project` (2.2) |
| 2.3 | `…/2.3/tela-23.js` | 84 | O tipo no Python (HC-F6), a relação "instancia", com e sem o `__init__` que roda, e a variante do Rust (2.3a); desde a DEC-40, "decidida, sem seguir" na instanciação sem `__init__` |
| 2.3b | `…/2.3b/tela-23b.js` | 102 | Decorador (`decorates`, não seguido) e chamada de campo pelo método que o declara (`via`) (2.3b) |
| 2.4a | `…/2.4a/tela-24a.js` | 88 | Chamada simples pelo escopo (`scope`), pelo `import` do arquivo e o `unique` que o escopo confirma (2.4a) |
| 2.4c | `…/2.4c/tela-24c.js` | 193 | Herança, `super()`, `@property` e o despacho polimórfico: caixa, escolha, texto do abstrato (2.4b e 2.4c) |
| 2.4d-0 | `…/2.4d0/tela-24d0.js` | 240 | A chamada pelo nome da classe e o que a 2.4c confere (2.4d-0) |
| 2.4d | `…/2.4d/tela-24d.js` | 71 | "Onde é usado?" pela regra (2.4d) |
| g3 | `docs/reports/piloto-1b/dados/g3/tela-g3.js` | 14 | As chamadas não apresentadas por motivo, com o `node_limit` à parte, no projeto sintético, PT e EN |
| `tela-estado` (parte A) | `tela-estado.js` | 46 | F1 do novo frontend: o estado local (navegação por projeto, rascunho por projeto e run, edition pelo `catalog_sha256`, duas abas, armazenamento bloqueado e cheio), numa página servida pela própria bateria, sem dados (a parte B, com servidor, está aposentada) |
| `tela-simplificacao` | `tela-simplificacao.js` | 47 | A simplificação de 17/11/2026, na página real e no servidor real, sobre o `renderer`: quem responde é um perfil da biblioteca de modelos (sem Modelfile, escada nem skills no compositor); a biblioteca só leitura, sem gemma4 nem gpt-oss; o Idioma do projeto nas Configurações; toda pergunta (busca, Seleção) só é preparada e abre na vista Pacote, com o pacote, a proveniência, H1 a H4, a tarefa e o idioma; Nenhum não envia; a conversa sem envio próprio e sem a seção de pacote; a Seleção e o Pacote com os nomes novos, em PT e EN; no modo Melhorar, a escada oferecida só no Pacote, sobre o mesmo pacote. Desde 18/11 (7 verificações a mais), o clique em "Enviar pela escada" com um comando aprovado: um único `POST /api/ladder`, com o `turn` do pacote revisado, `retries` e `lacuna_rounds` 0, sem `question`, `nodes`, `origin`, `with_facts` nem `models`, e nenhum outro envio. Monta a própria raiz em `LIBRARIAN_TESTE_SAIDA` e aprova nela só o comando `compilar`, que nada executa; precisa só de `LIBRARIAN_TESTE_STUDIO`, `LIBRARIAN_TESTE_RENDERER` e `LIBRARIAN_TESTE_SAIDA` (e, se não houver Chrome, `LIBRARIAN_TESTE_NAVEGADOR` com outro Chromium, como o Edge). `/api/send` é abortado; `/api/ladder` é respondido pela bateria |
| `tela-passo6` | `tela-passo6.js` | 36 | Passo 6: contagens manuais, inclusive depois de uma execução que falha, projeto sem fonte Rust e a mesma pergunta antes e depois da medição, em projetos Python e misto criados na hora, em `LIBRARIAN_TESTE_SAIDA` |

Nada é enviado a um modelo: as baterias abortam `/api/send` e as execuções, e simulam ou abortam `/api/ladder`. Só
`tela-passo6.js` executa comandos, fora da página e pela API: `git grep` nos projetos que cria.

## Baterias aposentadas

21 baterias e partes, com data, motivo e o que ficou sem cobertura: [APOSENTADAS.md](APOSENTADAS.md). Os arquivos
continuam nesta pasta (e em `docs/reports/`), sem mudança, e não contam como verificação.

## Origem

`tela-4a.js`, `tela-4b.js` e `tela-passo6.js` nasceram aqui. As outras são cópias das baterias usadas na auditoria dos Passos 1 e 2, com as verificações intactas. Só mudaram as linhas
de ambiente: o `require` do Playwright, os caminhos do `renderer`, do Python e do `casos.json` e as chaves.
Para conferir, compare com o original.

| Bateria | Sessão de origem | SHA-256 do original |
|---|---|---|
| `tela-cartao.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `51862c718f93167c0f544341a1aac7b69e4b238915f7a0d174a1de2b53fd260f` |
| `tela-zero.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `cc476a459d5a01ca1fd7518c0237bd3ddabf8351df77b0e2cec83f6138847c20` |
| `tela-origem.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `a284f8feec33c18c7f3fc3f5b8479baeeba40c619e82b59f0d94270d5f721aa8` |
| `tela-nomes.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `5eab6aeaf75a370325c4af5a7efe2b4e9b9f6a8440803388e83227a4d589f2b2` |
| `tela-passo2.js` | `fffe8c28-bb60-458b-8c64-078e0f7ff8e6` | `2da96f5c141c7501c4b7ce3e26ae4957fed363fab887f800661ef8dac99cfda2` |
| `etapa2c.js` | `465f4d1b-0d17-4dff-903f-d3077bb3c8f4` | `39ee0a736e32e7cabce2cc34948c0df9d4b708614d332eac31c07d119c51ac07` |

`etapa2.js` e `etapa2b.js` não estão aqui: seguem no scratchpad da sessão `465f4d1b` e, na mesma versão, no
arquivo de preservação, descrito no [plano até 18/11](../../../../docs/historico/PLANO-ate-18-11.md). O `etapa2c.js`
desse arquivo é anterior ao daqui: a busca do par de funções foi ajustada depois.
