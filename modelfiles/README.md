# Modelfiles

Biblioteca de Modelfiles do Librarian, fora dos crates: modelo, parâmetros e `SYSTEM` são
escolha de quem pergunta, não do núcleo. Cada arquivo `<nome>.Modelfile` aparece no
[Librarian Studio](../STUDIO.md#5-modelfiles), onde pode ser editado ou criado.

| Arquivo | Uso |
| --- | --- |
| `analista.Modelfile` | Explica somente as evidências selecionadas (FACT, LACUNA; inferência e hipótese só quando pedidas). |
| `lexico.Modelfile` | Propõe grupos de sinônimos para o léxico (**Sugerir léxico** no studio); contexto maior, saída só em JSON. |
| `verificavel.Modelfile` | Responde no formato **claims**: só o JSON pedido, com cada item carregando a própria prova (declaração e operação citadas em cada item), com um exemplo completo da forma. O `analista` foi escrito para prosa. |
| `performance.Modelfile` | Etapa de performance em claims, com `deepseek-coder-v2:16b`: cada mudança como HYPOTHESIS com edições de linhas (`edits`) e plano de medição, com um exemplo completo da forma. Exige **permitir HYPOTHESIS** e **permitir patch** no projeto. |
| `performance-qwen.Modelfile` | O `performance` com `qwen2.5-coder:14b`, com o mesmo SYSTEM e parâmetros, para comparar os dois modelos na mesma etapa. |
| `verificavel-deepseek.Modelfile` | O `verificavel` com `deepseek-coder-v2:16b` no lugar do qwen, com o mesmo SYSTEM e parâmetros, para comparar modelos. Exige `ollama pull deepseek-coder-v2:16b`. |
| `<tarefa>-deepseek-pt` / `<tarefa>-deepseek-en` | Pares por idioma para `analista`, `lexico`, `verificavel` e `performance`, com `deepseek-coder-v2:16b` e os parâmetros da tarefa: o `-pt` tem o SYSTEM em português (igual ao da tarefa), o `-en` o mesmo SYSTEM traduzido, com a resposta em inglês. Servem para ver se o modelo acerta mais em inglês. |
| `<tarefa>-claude-pt` / `<tarefa>-claude-en` | Os mesmos pares para o Claude: só o SYSTEM. A linha `FROM claude` existe porque o studio exige `FROM`; com o Ollama, `ollama create` falha. |

Nos pares por idioma, só o SYSTEM muda de língua: as instruções que o Librarian põe na query
(as regras da casa, os limites da evidência) continuam em português. No **Sugerir léxico**, um
Modelfile cujo nome termina em `-en` pede palavras em inglês, e o pedido inteiro vai em inglês.

- **Ollama:** o studio cria o modelo `librarian-<nome>` com `ollama create` a cada envio e
  guarda a cópia usada na pasta do envio.
- **Claude:** só o `SYSTEM` é usado, como system prompt. `FROM` e `PARAMETER` são do Ollama.

Os arquivos são marcados `-text` no `.gitattributes` para manter os bytes iguais em
qualquer sistema.
