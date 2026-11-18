# Biblioteca de modelos

Os modelos que o Librarian usa e o que cada um recebe ficam num catálogo só,
[`catalog.json`](catalog.json), junto com os textos das tarefas em [`system/`](system). Quem lê o
catálogo e monta cada Modelfile é o `librarian-llm-engine` (`src/models.rs`); o
[Librarian Studio](../STUDIO.md#5-modelos) só escolhe e mostra, sem editar.

Um Modelfile não é mais um arquivo guardado. Ele é montado no envio a partir de **tarefa × perfil
× idioma**. A cópia usada fica gravada na pasta do envio (`Modelfile.requested`), com o hash.

## Tarefas

A tarefa vem do que é enviado, nunca de uma escolha na tela:

| Tarefa | Quando | O que pede |
| --- | --- | --- |
| `verificavel` | Pergunta em claims sem mudança permitida | Responder em claims: só o JSON pedido, cada item com a própria prova |
| `performance` | Pergunta em claims com `allow_edits` ou `allow_patch` (modo Melhorar) | Propor mudança: uma HYPOTHESIS por mudança, com `edits` e plano de medição |
| `lexico` | Sugerir léxico | Grupos de sinônimos para a busca (JSON) |
| `analista` | Pergunta em **prosa**, um legado fora do fluxo padrão | Explicar só as evidências (FACT, LACUNA). Sem conferência mecânica |

Cada tarefa tem **um texto por idioma** (`system/<tarefa>.<idioma>.txt`), o mesmo para todos os
perfis. Há uma exceção registrada: o `verificavel` em inglês tem duas traduções independentes do
mesmo texto em português, feitas em 26/10 (Claude) e em 01/11 (qwen), que diferem só na redação.
Nenhuma das duas tem validação registrada. **Decisão de 18/11/2026: a exceção fica, temporariamente.**
Cada perfil mantém a sua tradução: o Claude usa `system/verificavel.en.txt`, e o qwen usa
`system/verificavel.en.qwen3coder.txt`, por `system_by_profile`. Nenhum texto foi reescrito, e os
hashes fixados no teste de compatibilidade não mudaram. Unificar as duas é uma decisão para depois,
de preferência com uma validação em inglês, porque muda os bytes de um dos dois Modelfiles.

## Perfis

| Perfil | Provedor | Modelo | Parâmetros |
| --- | --- | --- | --- |
| `qwen3coder`, local, o preferido | Ollama | `qwen3-coder:30b` | `temperature 0`, `seed 42`, `num_ctx 32768`, e o `num_predict` de cada tarefa |
| `claude`, remoto | Claude Code CLI | `sonnet`, `opus` ou `haiku` (padrão `sonnet`) | Só o `SYSTEM` vale; `FROM claude` existe porque um Modelfile exige `FROM` |

As versões dos modelos e os parâmetros se mudam no catálogo. No Ollama, o perfil local precisa de
`ollama pull qwen3-coder:30b`.

## Idioma

Um lugar só decide o idioma: o campo **Idioma** das configurações do projeto (`language`: vazio ou
`pt` para português, `en` para inglês). Ele escolhe o contrato de claims, o texto de cada tarefa e o
idioma do pedido do léxico. O nome de um arquivo não decide mais nada.

## Padrão de cada ação

`actions` diz qual perfil cada ação usa quando a pessoa não escolheu outro:

| Ação | Padrão |
| --- | --- |
| localizar, melhorar, cadeia, léxico, lote de fatos | `qwen3coder` |
| escada | `claude`, com `haiku` |

A página guarda, por tipo de ação, a escolha da pessoa. Uma escolha guardada antes da biblioteca
(provedor e Modelfile) é trocada pelo padrão.

## Skills

`tasks.<tarefa>.skills` diz quais skills (`../skills/<nome>/SKILL.md`) uma tarefa aceita. Um envio
recusa uma skill que a sua tarefa não aceita, e a página não oferece nenhuma fora disso. Hoje nenhuma
tarefa aceita skill.

A Ponytail continua em `../skills/ponytail/`, para uma futura especialização em desempenho. O
resultado registrado dela é de 01/11, com ferramentas e acesso ao código, e não se aplica aos envios
JSON do Studio.

## Compatibilidade

Os 14 Modelfiles que existiam como arquivos são montados de novo **byte a byte**. O teste
`the_catalog_renders_the_files_it_replaced_to_the_byte` (`librarian-llm-engine`) fixa o hash de
cada um. Isso inclui os três validados:

- `performance-claude`: `fb60d59c…`
- `verificavel-qwen3coder`: `64bf8c2d…`
- `lexico-qwen3coder`: `e1b71e78…`

Os envios gravados antes guardam a própria cópia do Modelfile usado.

Até 17/11/2026, a biblioteca tinha 22 arquivos, `<tarefa>-<modelo>[-en].Modelfile`, com
`qwen3coder`, `gemma4`, `gptoss` e `claude`. Os de gemma4 e gpt-oss saíram: não eram padrão de
nenhuma ação, e nenhum resultado registrado deles passou (R3 e R5). Junto saiu a diretiva
`# librarian: structured-output off`, que só eles usavam. Até 31/10/2026, havia também
`qwen2.5-coder:14b` e `deepseek-coder-v2:16b`.

Os arquivos são marcados `-text` no `.gitattributes`, para manter os bytes iguais em qualquer sistema.
