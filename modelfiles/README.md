# Modelfiles

Biblioteca de Modelfiles do Librarian, fora dos crates: modelo, parâmetros e `SYSTEM` são
escolha de quem pergunta, não do núcleo. Cada arquivo `<nome>.Modelfile` aparece no
[Librarian Studio](../STUDIO.md#5-modelfiles), onde pode ser editado ou criado.

| Arquivo | Uso |
| --- | --- |
| `analista.Modelfile` | Explica somente as evidências selecionadas (FACT, LACUNA; inferência e hipótese só quando pedidas). |
| `lexico.Modelfile` | Propõe grupos de sinônimos para o léxico (**Sugerir léxico** no studio); contexto maior, saída só em JSON. |
| `verificavel.Modelfile` | Responde no formato **claims**: só o JSON pedido, com cada item carregando a própria prova (declaração e operação citadas em cada item). O `analista` foi escrito para prosa. |
| `performance.Modelfile` | Etapa de performance em claims, com `deepseek-coder-v2:16b`: o que o código faz hoje como FACT e cada mudança como HYPOTHESIS com `patch` e plano de medição. Exige **permitir HYPOTHESIS** e **permitir patch** no projeto. |
| `verificavel-deepseek.Modelfile` | O `verificavel` com `deepseek-coder-v2:16b` no lugar do qwen, com o mesmo SYSTEM e parâmetros, para comparar modelos. Exige `ollama pull deepseek-coder-v2:16b`. |

- **Ollama:** o studio cria o modelo `librarian-<nome>` com `ollama create` a cada envio e
  guarda a cópia usada na pasta do envio.
- **Claude:** só o `SYSTEM` é usado, como system prompt. `FROM` e `PARAMETER` são do Ollama.

Os arquivos são marcados `-text` no `.gitattributes` para manter os bytes iguais em
qualquer sistema.
