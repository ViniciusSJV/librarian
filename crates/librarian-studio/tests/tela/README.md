# Baterias de tela do Studio

Regressão dos Passos 1, 2, 4 e 6 da Etapa 3 e da seleção da Etapa 2 (`origin.selection`), na página real, num
Chrome sem janela, contra o servidor real. Cobrem o que os testes em Rust não alcançam: o que a tela diz, o
cartão de desempenho, a dica da cidade, o fluxograma e o registro da seleção.

Elas ficam fora do `cargo test` e do CI: o cargo não executa arquivos `.js`, e as baterias precisam de um
ambiente e de dados que não estão no repositório.

| Bateria | Verificações na última execução | O que protege |
|---|---|---|
| `tela-cartao.js` | 37 | O cartão diz de onde veio a resposta (`execution`, ambíguo, não encontrado), com e sem medição |
| `tela-zero.js` | 29 | A semântica do zero e a resolução do alvo no `renderer` real |
| `tela-origem.js` | 32 | O texto de cada `count_kind` no ranking, em Separar, na cidade, no fluxograma e na pergunta |
| `tela-nomes.js` | 12 | Função com mais de uma linha contada, nas mesmas telas |
| `tela-passo2.js` | 9 | Ordem por contagem, sem alvo automático, seletor de Separar |
| `tela-4a.js` | 30 | Passo 4, entrega 4A: ranking identificado como Top 20 e as listas de sem passagens e sem cobertura, comparadas com `/api/hotspots`, em português e inglês |
| `tela-4b.js` | 55 | Passo 4, entrega 4B: o estado de cada item no painel da sugestão, comparado com `/api/envelope/suggest`, sem medição, com estado ausente ou desconhecido, em português e inglês |
| `tela-passo6.js` | 36 | Passo 6: contagens manuais, inclusive depois de uma execução que falha, projeto sem fonte Rust e a mesma pergunta antes e depois da medição, em projetos Python e misto criados na hora, em `LIBRARIAN_TESTE_SAIDA` |
| `etapa2c.js` | 45 | Os bloqueadores e as combinações de `origin.selection`, gravadas pelo servidor real |

Nada é enviado a um modelo: as baterias abortam `/api/send`, as execuções e `/api/generate`, e simulam ou
abortam `/api/ladder`. Só `tela-passo6.js` executa comandos, fora da página e pela API: `git grep` nos projetos que cria.

## O que precisa existir

| Variável | O que é |
|---|---|
| `LIBRARIAN_TESTE_STUDIO` | Executável `librarian_studio`, compilado do código a testar |
| `LIBRARIAN_TESTE_RAIZ_RS` | Cópia de uma raiz do Librarian, com `.studio/projects/renderer-librarian-*` e as rodadas `20261104-014710` e `20261104-020244` |
| `LIBRARIAN_TESTE_RAIZ_PY` | Cópia de uma raiz, com o projeto Python `backend-*` e a rodada `20261029-053623`, sem medição |
| `LIBRARIAN_TESTE_COPIAS` | Pasta para as cópias de execução |
| `LIBRARIAN_TESTE_RENDERER` | Caminho do projeto `renderer`, a que as rodadas se referem |
| `LIBRARIAN_TESTE_PYTHON` | Caminho do projeto Python, a que a rodada se refere |
| `LIBRARIAN_TESTE_SAIDA` | Pasta para capturas de tela e o resultado do `etapa2c.js` |
| `LIBRARIAN_TESTE_PLAYWRIGHT` | Opcional: caminho do `playwright-core`, se ele não estiver no `NODE_PATH` |
| `LIBRARIAN_TESTE_CASOS` | Opcional: `casos.json` do gabarito; o padrão é `.studio/gabarito/casos.json` neste repositório, pasta que também fica fora do Git |
| `LIBRARIAN_TESTE_CHAVE_RENDERER`, `LIBRARIAN_TESTE_CHAVE_PYTHON` | Opcionais: chaves dos projetos, se os caminhos não forem os originais |

Também é preciso Node 18 ou mais novo, Google Chrome e as portas 8841 e 8842 livres. Com isso:

```
node crates/librarian-studio/tests/tela/rodar.js
```

A execução completa leva mais de 10 minutos.

Para rodar uma só, suba os servidores nas duas portas e chame `node tela-zero.js <pasta de saída>` com as
mesmas variáveis. `etapa2c.js` pede também o caminho do projeto Python: `node etapa2c.js <saída> <python>`.

**Limites que valem para quem for rodar:**

- **O código não tem caminho fixo, mas os dados têm.** Os resultados esperados (`6.410.258`, 38 e 125 no
  zero, 12 candidatos de `default`) são dessas rodadas do `renderer`, que não estão no Git. O conjunto usado
  aqui ocupa cerca de 1,2 GB.
- **A chave do projeto é o nome mais o hash do caminho.** Em outro caminho, o Studio procura outra pasta em
  `.studio/projects`. As variáveis de chave existem para isso, mas rodar assim, ou em outra máquina, **nunca
  foi testado**.
- **As baterias gravam na raiz:** `etapa2c.js` cria turnos. `rodar.js` recusa o próprio repositório como raiz
  e exige que a página da raiz seja igual à do repositório.

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
arquivo de preservação, descrito no [plano](../../../../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md). O `etapa2c.js`
desse arquivo é anterior ao daqui: a busca do par de funções foi ajustada depois.
