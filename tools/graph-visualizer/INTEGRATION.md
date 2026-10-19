# Integração sugerida

## Librarian

```text
librarian/
└── tools/
    └── graph-visualizer/
```

O visualizador fica genérico e conhece apenas os contratos dos artefatos.

## Renderer

O PLAYME continua produzindo a execução. Futuramente ele pode apenas abrir o viewer ou gravar um pequeno `viewer.json` apontando para a pasta da execução.

Assim o mesmo viewer pode ser reutilizado no Renderer, no xadrez Angular e em outros projetos.
