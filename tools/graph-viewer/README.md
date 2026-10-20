# Graph Viewer

Visualizador 3D do contrato `syntax-graph-v2`, em um único `index.html`, sem build
nem npm. Three.js e d3-force-3d vêm do jsDelivr em versões fixadas (`three@0.180.0`,
`d3-force-3d@3.0.6`), então é preciso conexão na primeira abertura.

## Usar

1. Gere o grafo: `librarian_ingest graph <edition> <novo.json> v2`. O PLAYME já grava
   `graph-v2.json` em cada execução.
2. Abra `index.html` no navegador, com duplo clique (`file://`).
3. Escolha a **pasta de execução** do PLAYME ou os **arquivos**, ou arraste-os para a área do grafo.

| Arquivo | Uso |
| --- | --- |
| `graph-v2.json` | Obrigatório. Outras versões são recusadas com a instrução para gerar a v2. |
| `prepared/search.json` | Opcional: destaca os resultados da busca e a vizinhança do primeiro, **por ID**. É recusado se o `catalog_sha256` for diferente do grafo. |
| `prepared/request.json` | Opcional: indica quais resultados viraram fichas `E1…En` enviadas ao modelo. |

## O que mostra

- **Nós:** fonte (octaedro), símbolo (esfera) e ocorrência (tetraedro). Formato e cor
  identificam a categoria; a legenda traz as contagens e serve de filtro.
- **Arestas:** `declares`, `contains`, `observes` e `name_candidate` (tracejada, não
  resolvida), cada uma com filtro próprio.
- **Inspeção:** clique num nó para ver categoria, kind, linguagem, ID e relações. Clique
  numa relação ou numa aresta para ver **qual evidência autorizou a ligação**: arquivo,
  linhas, bytes e hashes.
- **Busca** por rótulo, kind ou ID. `F` enquadra, `Esc` limpa e o botão ◐ alterna o tema.

## Limites

- O layout de forças é só para leitura. Proximidade visual não cria relação, e o layout
  muda entre aberturas.
- Destaques usam somente IDs. Não há associação por nome, então homônimos não se
  confundem.
- Para inspeção automatizada, abrir com `#debug` expõe `window.viewerDebug`.

Esta página substitui o protótipo `tools/graph-visualizer` (Vite), que lia o formato
antigo e falhava com `nodes` indexado por ID.
