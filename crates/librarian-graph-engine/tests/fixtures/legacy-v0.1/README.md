# Exportações legadas de dossiês v0.1

Saídas de `prepare_query` geradas uma única vez em 19/10/2026, antes da etapa G3,
a partir de [dossier.json](dossier.json). Cobrem `kind` legado `rust_source`
(código: omite `executed`), `test_run` com e sem `execution`, texto comum e
conferência de captura. Testadas por [`legacy_exports.rs`](../../legacy_exports.rs).

**Não regenere estes arquivos para fazer um teste passar.** Dossiês antigos sem
`content` declarado devem continuar produzindo estes bytes.
