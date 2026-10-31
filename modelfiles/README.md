# Modelfiles

Biblioteca de Modelfiles do Librarian, fora dos crates: modelo, parâmetros e `SYSTEM` são
escolha de quem pergunta, não do núcleo. Cada arquivo `<nome>.Modelfile` aparece no
[Librarian Studio](../STUDIO.md#5-modelfiles), onde pode ser editado ou criado.

## Nomes

`<tarefa>-<modelo>.Modelfile`, com o SYSTEM em português, e `<tarefa>-<modelo>-en.Modelfile`,
com o mesmo SYSTEM em inglês. Para a mesma tarefa, todos os modelos têm o mesmo SYSTEM e os
mesmos parâmetros: só a linha `FROM` muda, para comparar modelos na mesma etapa. Um teste do
studio confere o padrão e que cada arquivo tem SYSTEM.

| Tarefa | Uso |
| --- | --- |
| `analista` | Explica somente as evidências selecionadas (FACT, LACUNA; inferência e hipótese só quando pedidas). Escrito para prosa. |
| `lexico` | Propõe grupos de sinônimos para o léxico (**Sugerir léxico** no studio); contexto maior, saída só em JSON. |
| `verificavel` | Responde no formato **claims**: só o JSON pedido, com cada item carregando a própria prova (declaração e operação citadas em cada item), com um exemplo completo da forma. |
| `performance` | Etapa de performance em claims: cada mudança como HYPOTHESIS com edições de linhas (`edits`) e plano de medição, com um exemplo completo da forma. Exige **permitir HYPOTHESIS** e **permitir patch** no projeto. |

| Modelo | `FROM` | Observação |
| --- | --- | --- |
| `qwen3coder` | `qwen3-coder:30b` | MoE (3,3B ativos), ~18 GB: numa GPU de 12 GB, parte fica na RAM. |
| `gemma4` | `gemma4:12b` | Denso, ~8 GB: cabe inteiro numa GPU de 12 GB. |
| `gptoss` | `gpt-oss:20b` | MoE (~3,6B ativos), ~13 GB. Raciocina antes de responder: o raciocínio também conta no `num_predict`. |
| `claude` | `claude` | Só o SYSTEM vale, como system prompt. A linha `FROM claude` existe porque o studio exige `FROM`; com o Ollama, `ollama create` falha. Tem as versões `-en`. |

Os modelos locais precisam de `ollama pull` (por exemplo `ollama pull qwen3-coder:30b`).

Nas versões `-en`, só o SYSTEM muda de língua: as instruções que o Librarian põe na query
(as regras da casa, os limites da evidência) continuam em português. No **Sugerir léxico**, um
Modelfile cujo nome termina em `-en` pede palavras em inglês, e o pedido inteiro vai em inglês.

- **Ollama:** o studio cria o modelo `librarian-<nome>` com `ollama create` a cada envio e
  guarda a cópia usada na pasta do envio.
- **Claude:** só o `SYSTEM` é usado, como system prompt. `FROM` e `PARAMETER` são do Ollama.

Até 31/10/2026 a biblioteca tinha `analista`, `lexico`, `verificavel` e `performance` com
`qwen2.5-coder:14b` e `deepseek-coder-v2:16b`, e pares `-deepseek-pt`/`-deepseek-en`; os envios
gravados antes disso guardam a cópia do Modelfile usado na pasta de cada envio.

Os arquivos são marcados `-text` no `.gitattributes` para manter os bytes iguais em
qualquer sistema.
