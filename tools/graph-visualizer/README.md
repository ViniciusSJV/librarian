# Librarian Graph Visualizer

Visualizador 3D genérico para os artefatos produzidos pelo Librarian/PLAYME.

> **Estado: protótipo; não abre o `graph.json` atual.** O Rust serializa `nodes`
> como objeto indexado por ID e `src/main.js` chama `.map` sobre ele
> (`map is not a function`). Diagnóstico e correção prevista (etapa 5A) em
> [PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md](../../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md), seção 4.
> As funcionalidades abaixo existem no código, mas não foram verificadas de ponta a ponta.

## Recomendação de localização

```text
librarian/
└── tools/
    └── graph-visualizer/
```

Deixe o visualizador no **Librarian**, porque ele conhece o contrato genérico do grafo e pode ser reutilizado por Renderer, xadrez Angular ou qualquer outro consumidor.

O Renderer/PLAYME deve apenas produzir/apontar os artefatos.

## O que ele lê

- `graph.json` — grafo completo.
- `edition/sources.jsonl` — fontes.
- `edition/symbols.jsonl` — símbolos.
- `edition/chunks.jsonl` — chunks/evidências.
- `graph-trace.json` — opcional, para destacar a consulta atual.
- `search.json` — opcional, para destacar E1 (`hits[0]`).

Os chunks são adicionados como nós **sintéticos de visualização** e ficam ocultos por padrão. Isso não altera o `graph.json`.

## Instalar

```powershell
npm install
```

## Rodar

```powershell
npm run dev
```

Abra o endereço mostrado pelo Vite e selecione a pasta de uma execução PLAYME, por exemplo:

```text
renderer/
└── ai/
    └── consultas/
        └── playme-20260925-XXXXXX-XXXXXXXX/
            ├── graph.json
            ├── graph-trace.json
            ├── search.json
            └── edition/
                ├── sources.jsonl
                ├── symbols.jsonl
                └── chunks.jsonl
```

## Princípio

A aparência é orgânica, mas a semântica continua determinística:

- arestas normais vêm de `graph.json`;
- `graph-trace.json` apenas destaca um subgrafo;
- E1 vem de `search.json`;
- `Symbol -> Chunk` é uma ligação sintética explicitamente criada apenas para navegação visual.

O visualizador não deve inferir relações novas por proximidade, nomes ou convenções.
