# PLAYME

Trilha completa e genérica: edition, grafo, busca, E1 conferido, query, Ollama e
preservação da tentativa. Configurada por um perfil JSON do consumidor.

```powershell
.\PLAYME.ps1 -ProfilePath <perfil.json> [-Question <texto>] [-PrepareOnly] [-OutputDirectory <novo>]
```

Campos do perfil, fluxo e artefatos: [WORKFLOW](../../WORKFLOW.md).
Requer Windows PowerShell 5.1, Cargo e, sem `-PrepareOnly`, Ollama.
