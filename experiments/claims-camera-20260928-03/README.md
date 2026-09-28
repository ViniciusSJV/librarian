# Mapa de linhas — câmera, série 03

Três tentativas previstas em protocol-before.md. Mantidos pergunta, contexto,
schema, modelo e checker. A query acrescenta evidence.line_map, gerado linha a
linha do trecho, com número original e texto completo, e instrução para usá-lo.
Não inclui implementação adicional, interpretação geométrica ou gabarito.

Resultado: 3 transportes completed e 3 aprovações mecânicas. Todas as respostas
identificam src/camera.rs:86 e citam exatamente
`        let direction = (pixel - origin).normalize();`, incluindo indentação.
Não há interpretação geométrica, inferência, hipótese, execução, performance ou
lacuna extra. Os textos são idênticos entre si.

A resposta ainda omite o nome ray_from_pixel exigido pela rubrica. Portanto a
localização apresentada é sustentada, mas a resposta não é integralmente aceita.
O protocolo anterior à geração já esclarecia que o nome escrito no trecho e o
arquivo bastam, sem inventar o escopo Camera ausente da declaração selecionada.

evaluation.json contém hashes/conferências e avaliação posterior por tentativa.
Os relatórios mecânicos mantêm semantic_status=pending e accepted=false. A série
04 tenta tornar explícita a exigência do nome sem alterar a evidência ou o checker.
