# Histórico

Documentos que saíram do uso ativo e ficam aqui para consulta. O estado atual do projeto está no [ESTADO-ATUAL.md](../../ESTADO-ATUAL.md).

## PLANO-ate-18-11.md

**De onde veio:** é o `PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md` como estava em 18/11/2026, no commit `aae51d7`, copiado inteiro. Na consolidação documental (passo 1.2 do roadmap), o plano da raiz passou a guardar só o roadmap ativo, e todo o resto ficou aqui: o diagnóstico de 27/09, a ETAPA 2, os roteiros de 19/10, 22/10 e 30/10, a Fase L, a fase R, o Evidence Builder de 04/11, as Etapas 0–7 do Studio, a consolidação de 17/11, o Piloto 1 e o roadmap anterior.

**O que mudou na cópia:** só o destino de 22 links relativos, que ganharam o prefixo `../../` para continuar apontando para os mesmos arquivos a partir desta pasta (`STUDIO.md`, `CONTEXTO-CONTINUIDADE.md`, `crates/…`, `docs/…`, `modelfiles/…`). O número de linhas (2302) e o texto visível de cada linha são os do original. Nada foi acrescentado no topo nem no meio, para que as linhas continuem idênticas.

**Como citar:** `PLANO-H:N` é a linha N deste arquivo, que é a mesma linha N do plano em `aae51d7`. Toda referência `PLANO:N` feita até 18/11/2026 (por exemplo, no [relatório 03 do Piloto 1b](../reports/piloto-1b/03-inventario-e-auditoria.md)) vale aqui com o mesmo número. As âncoras das seções também são as mesmas.

**O original exato:**

```
git show aae51d7:PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md
```

**Conferência da cópia** (Git Bash, na raiz do repositório): o mesmo número de linhas e o mesmo texto visível, comparando as linhas com o destino de cada link apagado:

```
vis() { sed -E 's/\]\([^)]*\)/]()/g'; }
diff <(git show aae51d7:PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md | vis) <(vis < docs/historico/PLANO-ate-18-11.md) && echo identico
diff <(git show aae51d7:PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md) docs/historico/PLANO-ate-18-11.md | grep -c '^>'   # 22
```

### Estado corrigido

O texto é histórico e não foi corrigido. Onde ele descreve um estado que mudou, vale o seguinte (inventário D6 do Piloto 1b):

| Onde | O que o texto diz | Estado em 18/11/2026 |
|---|---|---|
| PLANO-H:1612, 1933 | Etapa 4 "em andamento, sem commit" | Encerrada e publicada: `09e7e53` ("conclui etapa 4") e `9bc92b0` |
| PLANO-H:1973–1979 | P1–P4 da Etapa 4 "aguardam avaliação"; P4 (Ubuntu) pendente | P1–P3 foram para os commits acima. P4 é **ambígua**: houve validação no Ubuntu em 18/11, mas da consolidação (CTX:873), não da Etapa 4 |
| PLANO-H:1613 | Etapa 5 "não iniciada" | Parcial: C1 (`/api/ladder` com `turn`) e "todo envio sai do Pacote" foram feitos em 18/11 (PLANO-H:2087–2098). Congelada até o fim da Fase 2B |
| PLANO-H:1614–1615 | Etapas 6 e 7 "não iniciadas" | Absorvidas: DEC-30 e DEC-31 no [DECISOES.md](../../DECISOES.md) |
| PLANO-H:884 (item 7 de 30/10) | Tipo do receptor, sem estado | Feito só no Rust e só no inventário; não existe em Python nem em "o que chama?". Vai para a Fase 2 (2.1) |
| PLANO-H:2233 | Fase 1 "não iniciada" | P1 (`987d7a8`) e G3 (`aae51d7`) commitados; Q6 feita em 18/11 |
| PLANO-H:1555 | Homônimos: investigação suspensa | Reaberta na Fase 2 (DEC-11) |
