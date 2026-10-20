# Librarian LLM Engine

O [contrato experimental de afirmações](CLAIMS.md) acrescenta preparação de query
e verificação mecânica de categorias/citações, sem alterar o transporte legado
ou aprovar semanticamente as respostas. Use `claims prepare` e `claims check`.

Preparação e transporte Ollama do Librarian. Este crate não depende do renderer, do Graph Engine ou de um modelo específico.
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

## Estado atual

O [Librarian Studio](../../STUDIO.md) envia ao Ollama o `query.json` de um bundle
produzido por `prepare` e conferido por `verify-prepared` imediatamente antes do
envio, usando `attempt_linked` deste crate. O transporte preserva query, request,
retorno bruto, texto e estado.

- **Explicação ainda não aprovada.** Com `prepare.numbered_lines`, cada linha do
  contexto leva a linha original do arquivo. Numa execução, a resposta da câmera
  passou a citar a linha 86 de `src/camera.rs`; antes citava "linha 11", a posição
  dentro do trecho. Ela ainda não nomeia a função e copia o objeto
  `{number, text}` como evidência. A avaliação semântica está pendente.
- **O contrato de afirmações ainda não está na trilha do studio.** As
  [séries de experimentos](../../experiments/) continuam reprovadas pelo checker.
- **O vínculo do bundle no registro do envio ainda não é usado.**
  `attempt_linked` existe, mas `send_ollama` e o studio passam `None`.

Esses pontos seguem na fase L (limites do LLM Engine). Origem do crate,
validação da extração (27/09/2026) e experimentos anteriores estão no
[HISTORICO](../../HISTORICO.md).
