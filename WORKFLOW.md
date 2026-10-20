# Workflow: Librarian e projetos analisados

O Librarian executa toda a trilha da consulta. O projeto analisado (renderer, xadrez ou
outro) fornece, no máximo, um perfil `librarian.json`: fontes, vocabulário, instruções
de explicação e critérios prévios. O Librarian não conhece o domínio.

**O uso normal é pelo [Librarian Studio](STUDIO.md)**: página web com projeto, grafo,
perguntas e envio a LLMs. Lá estão como iniciar, o formato do perfil e onde ficam os dados.
Este documento descreve a mesma trilha pela CLI, para automação e conferência.

## Trilha pela CLI

Na raiz do Librarian, com `PROJ` = pasta do projeto analisado:

```powershell
cargo run --release --locked -p librarian-ingest --bin librarian_ingest -- generate $PROJ sources.json edition
cargo run --release --locked -p librarian-ingest --bin librarian_ingest -- verify edition $PROJ
cargo run --release --locked -p librarian-ingest --bin librarian_ingest -- graph edition graph-v2.json v2
cargo run --release --locked -p librarian-ingest --bin librarian_ingest -- verify-graph edition graph-v2.json
cargo run --release --locked -p librarian-ingest --bin librarian_ingest -- prepare edition request.json prepared
cargo run --release --locked -p librarian-ingest --bin librarian_ingest -- verify-prepared edition prepared
cargo run --release --locked -p librarian-llm-engine --bin send_ollama -- prepared/bundle/query.json http://127.0.0.1:11434/api/generate MODELO 300000 1048576 TENTATIVA_01 tentativa
```

O formato de `request.json` (pergunta, léxico, `top`, `depth`, `hits`, `radius`,
`numbered_lines`, instruções) está em [SEARCH.md](crates/librarian-ingest/SEARCH.md#da-busca-ao-preparedquery).
Saída 2 indica diagnósticos na edition, não falha. O modelo do Ollama é criado antes com
`ollama create MODELO -f modelfiles/<nome>.Modelfile`.

## Como ler o fluxo

1. O projeto escolhe arquivos; o Librarian preserva bytes e confere o acervo. O grafo
   registra pertencimento, tipos escritos, chamadas observadas e candidatos por nome,
   com intervalos e hashes de origem. Não há resolução por compilador.
2. A pergunta é normalizada e expandida pelo léxico explícito. O ranking encontra
   símbolos; o grafo amplia a vizinhança do primeiro resultado. `search` deriva esse
   grafo em memória dos snapshots; `graph-v2.json` é exportação, não banco.
3. Sem candidato não há evidência para enviar; diagnósticos na edition também
   interrompem a trilha antes do envio, salvo `accept_diagnostics`.
4. `prepare` transforma os primeiros resultados em fichas estruturais (onde está cada
   declaração, linhas a–b), confere as fontes contra os snapshots e exporta um
   `PreparedQuery`. `verify-prepared` refaz tudo e compara byte a byte imediatamente
   antes do envio. **O grafo e os demais resultados não entram na query.**
5. O Ollama recebe o Modelfile por `ollama create`; editar o arquivo não aplica nada.
6. `completed` é conclusão de transporte. A avaliação semântica é separada e manual,
   contra critérios gravados antes da resposta. Nenhuma resposta, hipótese ou relação
   candidata vira fato do acervo.

## Fronteira atual

O cliente HTTP não reconfere fontes no envio (`evidence_rechecked=false`); a conferência
ocorre antes, por `verify-prepared`. O vínculo do bundle no registro do envio
(`attempt_linked`) ainda não é usado. O checker de afirmações
([CLAIMS.md](crates/librarian-llm-engine/CLAIMS.md)) ainda não faz parte da trilha; seus
experimentos ficam em `experiments/`. A
[referência formal de dossiês 0.1](crates/librarian-graph-engine/WORKFLOW-V0.1.md)
preserva as regras de PreparedQuery como documentação técnica separada.
