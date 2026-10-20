# Modelfiles

Biblioteca de Modelfiles do Librarian, fora dos crates: modelo, parâmetros e `SYSTEM` são
escolha de quem pergunta, não do núcleo. Cada arquivo `<nome>.Modelfile` aparece no
[Librarian Studio](../STUDIO.md#5-modelfiles), onde pode ser editado ou criado.

| Arquivo | Uso |
| --- | --- |
| `analista.Modelfile` | Explica somente as evidências selecionadas (FACT, LACUNA; inferência e hipótese só quando pedidas). |
| `lexico.Modelfile` | Propõe grupos de sinônimos para o léxico (**Sugerir léxico** no studio); contexto maior, saída só em JSON. |

- **Ollama:** o studio cria o modelo `librarian-<nome>` com `ollama create` a cada envio e
  guarda a cópia usada na pasta do envio.
- **Claude:** só o `SYSTEM` é usado, como system prompt. `FROM` e `PARAMETER` são do Ollama.

Os arquivos são marcados `-text` no `.gitattributes` para manter os bytes iguais em
qualquer sistema.
