# Librarian LLM Engine

O [contrato experimental de afirmações](CLAIMS.md) acrescenta preparação de query
e verificação mecânica de categorias/citações, sem alterar o transporte legado
ou aprovar semanticamente as respostas. Use `claims prepare` e `claims check`.

Preparação e transporte extraídos do renderer, sem mudança de política ou formato.
Este crate não depende do renderer, do Graph Engine ou de um modelo específico.
O adaptador implementado é Ollama HTTP `/api/generate`, não uma API universal de
provedores. Modelo e endpoint são argumentos; Qwen/DeepSeek não são embutidos.

## Usar no Librarian

Na raiz do Librarian:

```powershell
cargo build --locked -p librarian-llm-engine --bins
cargo test --locked -p librarian-llm-engine
cargo run --locked -p librarian-llm-engine --bin prepare_ollama -- query.json NOME_DO_MODELO request-novo.json
cargo run --locked -p librarian-llm-engine --bin send_ollama -- query.json http://127.0.0.1:11434/api/generate NOME_DO_MODELO 300000 1048576 TENTATIVA_01 tentativa-nova
```

Os dois últimos comandos exigem uma query existente com `question`, `evidence`
como objeto e `instructions` como lista de textos. O primeiro prepara sem rede;
o segundo exige servidor/modelo disponíveis e envia. Destinos devem ser novos.
Nenhuma configuração de modelo é criada automaticamente por esses executáveis.
O Modelfile, seleção e política de domínio continuam sendo responsabilidade do consumidor.

## API pública e fronteiras

- `ollama::request(query, model)`: valida a estrutura básica e preserva os bytes
  da query como prompt; envia `model`, `prompt`, `stream=false`, sem overrides.
- `ollama::client::Config`: endpoint, modelo, timeout e limite de bytes explícitos.
- `ollama::client::attempt_linked(query_path, destination, id, &config, link)`:
  preserva consulta, request, retorno bruto, texto e estado. Use `None` para envio
  isolado; não reconfere as fontes nem julga afirmações.
- Os helpers de arquivos/hashes são públicos para o coordenador existente.
  `cli` contém as entradas compartilhadas pelas CLIs; estas podem encerrar o processo.
  Consumidores de biblioteca devem usar a API `ollama`.

`link=Some(...)` é uma declaração confiada ao coordenador: este deve realizar a
conferência e validar origem/seleção/rubrica antes da chamada. O transporte apenas
confere o hash da query contra o vínculo e preserva os metadados; não autentica
um vínculo arbitrário. Não apresentar a API como validação automática de evidência.

`completed` continua significando conclusão estrutural do transporte. Avaliação
semântica fica separada. Falhas, limites, timeouts, ausência de retry e preservação
de diretórios parciais mantêm o comportamento anterior. A prévia da CLI preparadora
inclui um LF final; o request do transporte não inclui esse LF adicional.

## Extração e continuidade

Extraído de `renderer/src/bin/ollama_common` e das CLIs prepare/send em 27/09/2026.
Os 3 testes do preparador e 13 do transporte foram movidos, com um gravador ignorado.
As CLIs antigas do renderer delegam a este crate. PLAYME/TESTME passam a usar os
executáveis do Librarian. `explain_evidence` permanece coordenador do consumidor,
mas importa este transporte. A ponte busca→PreparedQuery não foi implementada.

Os testes HTTP usam servidor simulado, sem Ollama. A extração não faz nova geração
real nem melhora a qualidade semântica por si só. Os artefatos históricos não foram
reescritos. O crate é desenvolvimento local, ainda não uma release publicada.

Para continuar o contrato epistemológico aqui, consulte o
[experimento de política da câmera](../../../renderer/ai/experimentos/26-politica-llm-camera/README.md)
e o [contrato atual](../../../renderer/ai/contratos/engines-v1.md).
As duas respostas reais foram reprovadas: houve interpretação geométrica dentro de
FACT e lacunas indevidas, mesmo sem títulos INFERENCE/HYPOTHESIS. Este é o próximo
problema, não uma correção feita durante a extração. Nenhuma edição automática de
código, mudança de modelo ou nova abstração de provedores foi acrescentada.

## Validação da extração

Em 27/09/2026, no Windows:

- `cargo test --workspace --locked --offline` no Librarian passou; o novo crate
  teve 16 testes aprovados e 1 gravador ignorado, usando HTTP simulado.
- No renderer, os alvos lib, catalog_sources, validate_evidence, explain_evidence,
  prepare_ollama e send_ollama passaram: 285 testes aprovados e 1 gravador ignorado.
  O validador foi compilado antes da integração. A suíte Unix completa não foi executada.
- A query preservada policy-camera-20260927-02 foi preparada pelo executável antigo
  antes do rebuild e pelo novo executável do Librarian: arquivos idênticos por SHA-256
  `6431cffd86d4944b581cea61d5e8d57894d1c508f11a45097ff367c0aecf51de`.
- PLAYME com a pergunta da câmera e `-PrepareOnly` concluiu com código 0 usando
  `librarian/target/debug/prepare_ollama.exe`. Logs locais em
  `renderer/target/llm-extraction-prepare-check`; sem contato com Ollama.
- PLAYME e 15 blocos PowerShell do TESTME passaram na análise sintática.
  Formatação Rust do novo crate e `git diff --check` nos dois repositórios passaram.

Cargo.lock do Librarian incorpora as dependências HTTP; o do renderer apenas
substitui sua dependência direta de reqwest pelo novo crate. Core/Graph Engine
do renderer continuam fixados na revisão Git anterior. A dependência LLM usa
o checkout vizinho por path, assim como ingestão; publicação não foi realizada.
