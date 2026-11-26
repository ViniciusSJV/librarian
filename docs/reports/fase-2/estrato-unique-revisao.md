# Estrato `unique` com receptor (antes do 2.4e): folha de revisão UQ-1 a UQ-30

Escrita em 25/11/2026. Healthchecks em `6c05a7a`, ray tracer em `e38254b`, fontes compiladas `3aeb2fa77adc9ec44716bc113822faa14395b71cb7f7217deb46b4e2b87c4726`. Não é passo da Fase 2: só medição; nenhuma regra, código de produção, linha de base, fixture do portão ou fato mudou.

> **Verdade validada pelo usuário em 25/11/2026 (DEC-22),** sem correção: as 30 propostas do agente ficaram como estavam. Como cada uma foi conferida (`verdade_validada_por` na fixture [estrato-unique.json](../../../tools/precisao/fixtures/estrato-unique.json)):
> - **no editor, pelo usuário:** UQ-3, UQ-4, UQ-19 e UQ-23;
> - **pela sessão, com os trechos do código, e validadas pelo usuário:** UQ-6, UQ-28 e UQ-30;
> - **iguais à verdade já validada na fixture do portão:** UQ-1 (HC-26), UQ-13 (HC-143) e UQ-17 (RT-13);
> - **pela justificativa da folha:** as outras 20.
>
> Nenhum pyright, mypy ou rust-analyzer foi consultado, para nada; a regra "o gabarito é ferramenta de teste, não evidência" está no PLANO, 2.4e. Os ids UQ-1 a UQ-30 estão no [docs/IDS.md](../../IDS.md).

## 1. População

**Definição.** As chamadas da população do portão (`pool` em [lib.js](../../../tools/precisao/lib.js): `obj.metodo()` com dono função ou método, fora de testes, sem `self.m()`, com o receptor escrito começando por um nome), nos runs gerados agora pelo binário das fontes `3aeb2fa7…` sobre uma cópia do `.studio` (como o portão faz), cuja resposta de `/api/calls` é `unique` com o alvo um método (`method` ou `trait_method`).

| | Pool do portão | `unique` no pool | alvo método | alvo classe (`modulo.Classe()`) | alvo função (`modulo.f()`) |
|---|---|---|---|---|---|
| Python (Healthchecks) | 1.956 | 325 | **233** | 76 | 16 |
| Rust (ray tracer) | 515 | 94 | **94** (93 + 1 `trait_method`) | 0 | 0 |

O pool e as contagens por `resolution` são os da linha de base (1.956 e 515; `unique` 325 e 94). Os 92 `unique` do Python que não vão para método têm por receptor um módulo do projeto (`forms.WebhookForm()`, `emails.alert()`): a forma só admite as funções e classes do arquivo do módulo (SEARCH.md, "Forma da chamada"). Não são a família de `Ping.get_body`, e ficam fora.

**Diferença para 655 e 245.** Os números do ESTADO vêm de [dados/2.4d/lista.js](dados/2.4d/lista.js) (linhas 16 e 169 de `lista.out`): todo link `unique` de uma chamada com receptor (`method_call_observed`) para um método, de todos os símbolos, com testes. Não é a população do portão. A diferença é só de definição, não de tempo: o 2.4d não mudou a regra de resolução (DEC-46). Decomposição, filtro a filtro, nos runs desta medição:

| Filtro, em ordem | Python | Rust |
|---|---|---|
| Todo link `unique` de chamada com receptor para método, de todos os símbolos | 655 | 257 (245 `method`, como no `lista.js`, + 12 `trait_method`) |
| − arquivo de teste pelo caminho (`tests/`, `test_`, `_test.`) | −309 | 0 |
| − dono com "test" no rótulo (no Rust, as funções de `mod tests` dentro de `src/`) | 0 | −121 |
| − dono que não é função nem método | 0 | 0 |
| − `self.m()` (com um só candidato no grafo, `self.m()` sai `unique`, não `receiver`) | −109 | −29 |
| − receptor escrito que não começa por um nome (`"..".len()`, `x[0].m()`, `(a - b).m()`) | −4 | −13 |
| **População** | **233** | **94** |

O `lista.out` dá 348 de produção no Python (307 em teste); aqui o filtro de caminho tira 309. Não investiguei a diferença de 2, que não muda a população: ela é medida pelo pool do portão, não por esta tabela.

**O tipo confirmado não tira chamada da população.** Pela regra, "um `unique` que o tipo confirma mantém o rótulo" (SEARCH.md, seção "O que chama", tabela de resoluções e "O tipo derruba um `unique`"); o tipo só dá `type` quando decide entre homônimos. Um tipo que não serve derruba o `unique` para sem candidato, e a chamada também sai da família. Então o estrato `type` não tem nenhuma destas chamadas, e esta população tem as confirmadas e as não confirmadas juntas. Não as separei no sorteio (o pedido é a família `unique`); registrei, para cada chamada, o que o grafo guarda do tipo do receptor: o `receiver_type` que começa na chamada e termina no fim do receptor escrito (como a `receiver_type_node` de `resolve.rs`), a regra (`type_rule`) e as declarações do projeto que o nome alcança.

| | Tipo do projeto, a classe do alvo | Tipo escrito que não nomeia declaração do projeto | Sem tipo escrito no grafo |
|---|---|---|---|
| Python (233) | 53 | 59 | 121 |
| Rust (94) | 36 | 0 | 58 |

Limite: no Python, o tipo pelo retorno anotado e pelo campo de outro arquivo (2.3c) e pela herança (2.4b) é calculado na consulta e não fica no grafo, e `/api/calls` só diz o `type_rule` num link `type` ou `homonym`. Por isso "sem tipo escrito no grafo" não quer dizer que a regra não tenha tipo (UQ-11: o grafo grava a regra do construtor para `_get_rw_check_for_user(..)`, que não é classe, e o retorno anotado `-> Check` está em `hc/front/views.py:140`). A coluna é registro, não confirmação.

**Os 18 links falsos conhecidos** (PLANO, 2.4d): 14 estão na população e entraram no sorteio como as outras chamadas, sem exclusão nem peso; 4 estão fora por definição, não por escolha: `src/count.rs:133` e `:151` (`Groups::len` e `Sorteio::next`, dois em cada linha) estão em funções de teste (`tests_count::each_counted_line_reports_how_many_times_it_ran_and_for_how_long` e `tests_count::a_recursion_counts_every_call_but_the_time_once`), que o pool exclui pelo rótulo do dono, e o receptor escrito não começa por um nome (`"librarian-measure cpu_ms=".len()`, `time[..].split(' ').next()`).

| | Conhecidos na população |
|---|---|
| Python | 4: `Ping.get_body` em `hc/api/management/commands/smtpd.py:37`, `:41`, `hc/front/views.py:800`, `:805` |
| Rust | 10: `Groups::len` em `src/obj.rs:90`, `src/camera.rs:129`; `Sorteio::next` em `src/obj.rs:19`, `:21`, `:22`, `:23`, `:28`, `:63`, `:66`, `:81` |

## 2. Sorteio

Script: [tools/precisao/estrato-unique.js](../../../tools/precisao/estrato-unique.js), rodado uma vez, como o `amostra.js`; o portão não o roda.

```
node tools/precisao/estrato-unique.js --studio .studio --saida docs/reports/fase-2/dados/estrato-unique --fontes 3aeb2fa77adc9ec44716bc113822faa14395b71cb7f7217deb46b4e2b87c4726
```

- **Semente:** `estrato-unique-2026-11-25`. A ordem é o SHA-256 de `semente|arquivo@início-fim`, crescente; ficam as 15 primeiras de cada linguagem. O `arquivo@início-fim` é o mesmo em qualquer run do mesmo código-fonte, então o sorteio não depende do run.
- Rodado duas vezes (a segunda só mudou um rótulo da coluna de tipo): as mesmas 30 chamadas. Runs da segunda: `20261125-234413` (Healthchecks) e `20261125-234657` (ray tracer), na cópia. O grafo novo difere do run de referência de 18/11 nos dois projetos, como na linha de base (`grafo_novo_igual_ao_de_referencia: false`): o perfil mudou desde então.
- Esqueleto, com a ordem de cada chamada, o hash dos binários e o do `.studio` antes e depois: [dados/estrato-unique/estrato-unique.esqueleto.json](dados/estrato-unique/estrato-unique.esqueleto.json).
- Saiu **1 dos 14 conhecidos**: UQ-19 (`ordered.len()`), marcado como conhecido.
- Três chamadas sorteadas já estão na amostra do portão, com a verdade validada: UQ-1 = HC-26, UQ-13 = HC-143, UQ-17 = RT-13. A verdade proposta repete a validada.
- `.studio` original: `healthchecks-25597e41` 932 arquivos, `61cd25c1…`; `renderer-9b78f3a9` 42 arquivos, `612cdbf6…`, iguais antes e depois de cada execução.

## 3. Folha

Colunas: na coluna "Editor", "sim" é conferida no editor pelo usuário, e "trechos", conferida pela sessão com os trechos do código e validada pelo usuário; o texto da chamada é a primeira linha do rótulo; "Alvo" é o que o Librarian liga (`unique`, 1 candidato em todas); "Receptor" é o receptor escrito e o que o grafo guarda do tipo dele (seção 1). "Verdade" é o corpo que roda; `externo: X` quer dizer que roda X, de fora do projeto, e então o link para o projeto é `link_falso`, como na HC-27. Nenhuma ficou indecisa.

### Python (Healthchecks)

| Id | Ocorrência | Linha | Quem chama | Chamada | Alvo do Librarian | Receptor e tipo no grafo | Verdade proposta | Classe | Certeza | Justificativa | Editor |
|---|---|---|---|---|---|---|---|---|---|---|---|
| UQ-1 | `hc/integrations/pagertree/transport.py@1094-1116` | 29 | `PagerTree.notify` | `flip.owner.tags_list()` | `Check.tags_list (hc/api/models.py:413)` | `flip.owner`; sem tipo escrito no grafo | `Check.tags_list (hc/api/models.py:413)` | correta | alta | = HC-26, validada (18/11). `flip: Flip` (`pagertree/transport.py:10`); `Flip.owner = ForeignKey(Check)` (`hc/api/models.py:1362`); nenhuma subclasse de `Check` no projeto |  |
| UQ-2 | `hc/integrations/spike/views.py@891-918` | 28 | `add_spike` | `channel.assign_all_checks()` | `Channel.assign_all_checks (hc/api/models.py:1067)` | `channel`; `Channel` (construtor), a classe do alvo | `Channel.assign_all_checks (hc/api/models.py:1067)` | correta | alta | `channel = Channel(project=project, kind="spike")` (`spike/views.py:24`); nenhuma subclasse de `Channel` |  |
| UQ-3 | `hc/integrations/signal/transport.py@2647-2666` | 91 | `Signal.send` | `reply.get_results()` | `Signal.Reply.get_results (hc/integrations/signal/transport.py:48)` | `reply`; regra do construtor, nome que não é declaração do projeto | `Signal.Reply.get_results (hc/integrations/signal/transport.py:48)` | correta | média | `reply = Signal.Reply.model_validate_json(reply_bytes)` (`signal/transport.py:80`); `Signal.Reply(BaseModel)` (`:44`). `model_validate_json` é o método de classe do pydantic que devolve uma instância da classe: semântica de fora do projeto. É a mesma leitura da HC-143 (`ZulipConf.model_validate_json`, `hc/api/models.py:1315`), validada. `get_results` só é declarado aqui no projeto | sim |
| UQ-4 | `hc/lib/curl.py@4405-4433` | 139 | `request` | `c.setopt(pycurl.NOSIGNAL, 1)` | `FakeCurl.setopt (hc/lib/tests/test_curl.py:18)` | `c`; regra do construtor, nome que não é declaração do projeto | externo: `pycurl.Curl.setopt` | **link_falso** | alta | `c = pycurl.Curl()` (`hc/lib/curl.py:138`); `FakeCurl` é o dublê que só os testes injetam. Mesma leitura da HC-27 (`c.getinfo`, mesma função), validada: dublê de teste não é alvo de código de produção (DEC-33). Não está entre os 18 conhecidos | sim |
| UQ-5 | `hc/front/views.py@2850-2867` | 88 | `_tags_counts` | `check.tags_list()` | `Check.tags_list (hc/api/models.py:413)` | `check`; sem tipo escrito no grafo | `Check.tags_list (hc/api/models.py:413)` | correta | alta | `checks: Iterable[Check]` (`hc/front/views.py:77`), `for check in checks` (`:82`) |  |
| UQ-6 | `hc/integrations/whatsapp/transport.py@1896-1919` | 59 | `WhatsApp.notify` | `profile.authorize_sms()` | `Profile.authorize_sms (hc/accounts/models.py:310)` | `profile`; sem tipo escrito no grafo | `Profile.authorize_sms (hc/accounts/models.py:310)` | correta | alta | `profile = self.channel.project.owner_profile` (`whatsapp/transport.py:58`); `Transport.__init__(self, channel: Channel)` (`hc/api/transports.py:63-64`); `Channel.project = ForeignKey(Project)` (`hc/api/models.py:1021`); `@property owner_profile -> Profile` (`hc/accounts/models.py:467-468`); `WhatsApp(HttpTransport)` (`whatsapp/transport.py:17`) não redefine `__init__` nem `channel` | trechos |
| UQ-7 | `hc/integrations/matrix/views.py@1202-1229` | 35 | `add` | `channel.assign_all_checks()` | `Channel.assign_all_checks (hc/api/models.py:1067)` | `channel`; `Channel` (construtor), a classe do alvo | `Channel.assign_all_checks (hc/api/models.py:1067)` | correta | alta | `channel = Channel(project=project, kind="matrix")` (`matrix/views.py:25`) |  |
| UQ-8 | `hc/integrations/webhook/transport.py@2534-2576` | 73 | `Webhook.notify` | `self.channel.webhook_spec(flip.new_status)` | `Channel.webhook_spec (hc/api/models.py:1203)` | `self.channel`; sem tipo escrito no grafo | `Channel.webhook_spec (hc/api/models.py:1203)` | correta | alta | `Webhook(HttpTransport)` (`webhook/transport.py:13`), `HttpTransport(Transport)` (`hc/api/transports.py:132`), `self.channel = channel` com `channel: Channel` no `Transport.__init__` (`:63-64`); nem `Webhook` nem `HttpTransport` redefinem `__init__` |  |
| UQ-9 | `hc/accounts/views.py@5785-5826` | 180 | `login` | `Profile.objects.for_user(magic_form.user)` | `ProfileManager.for_user (hc/accounts/models.py:62)` | `Profile.objects`; sem tipo escrito no grafo | `ProfileManager.for_user (hc/accounts/models.py:62)` | correta | alta | `objects = ProfileManager()` (`hc/accounts/models.py:115`); o gerenciador que o Django entrega por `Profile.objects` é desta classe (`ProfileManager(models.Manager["Profile"])`, `:61`) |  |
| UQ-10 | `hc/integrations/pd/transport.py@1072-1089` | 32 | `PagerDuty.notify` | `check.tags_list()` | `Check.tags_list (hc/api/models.py:413)` | `check`; sem tipo escrito no grafo | `Check.tags_list (hc/api/models.py:413)` | correta | alta | `check = flip.owner` (`pd/transport.py:18`), `flip: Flip` (`:14`), `Flip.owner = ForeignKey(Check)` (`hc/api/models.py:1362`) |  |
| UQ-11 | `hc/front/views.py@29679-29727` | 867 | `resume` | `check.create_flip("new", mark_as_processed=True)` | `Check.create_flip (hc/api/models.py:652)` | `check`; regra do construtor, nome que não é declaração do projeto | `Check.create_flip (hc/api/models.py:652)` | correta | alta | `check = _get_rw_check_for_user(request, code)` (`hc/front/views.py:863`), `-> Check` (`:140`) |  |
| UQ-12 | `hc/front/views.py@12002-12036` | 371 | `_get_project_summary` | `profile.checks_from_all_projects()` | `Profile.checks_from_all_projects (hc/accounts/models.py:207)` | `profile`; `Profile` (parâmetro), a classe do alvo | `Profile.checks_from_all_projects (hc/accounts/models.py:207)` | correta | alta | `def _get_project_summary(profile: Profile)` (`hc/front/views.py:367`) |  |
| UQ-13 | `hc/integrations/zulip/transport.py@1527-1560` | 48 | `Zulip.notify` | `self.channel.zulip.formatted_to()` | `ZulipConf.formatted_to (hc/api/models.py:958)` | `self.channel.zulip`; sem tipo escrito no grafo | `ZulipConf.formatted_to (hc/api/models.py:958)` | correta | alta | = HC-143, validada (25/11). `@property zulip -> ZulipConf` (`hc/api/models.py:1313-1315`) |  |
| UQ-14 | `hc/integrations/email/views.py@3829-3849` | 108 | `unsubscribe` | `channel.make_token()` | `Channel.make_token (hc/api/models.py:1071)` | `channel`; regra do construtor, nome que não é declaração do projeto | `Channel.make_token (hc/api/models.py:1071)` | correta | alta | `channel = get_object_or_404(Channel, code=code, kind="email")` (`email/views.py:107`): o Django devolve uma instância do modelo passado |  |
| UQ-15 | `hc/api/views.py@13150-13161` | 371 | `_update` | `spec.kind()` | `Spec.kind (hc/api/views.py:134)` | `spec`; `Spec` (parâmetro), a classe do alvo | `Spec.kind (hc/api/views.py:134)` | correta | alta | `def _update(check: Check, spec: Spec, v: int)` (`hc/api/views.py:332`); `class Spec(BaseModel)` (`:61`), sem subclasse |  |

### Rust (ray tracer)

| Id | Ocorrência | Linha | Quem chama | Chamada | Alvo do Librarian | Receptor e tipo no grafo | Verdade proposta | Classe | Certeza | Justificativa | Editor |
|---|---|---|---|---|---|---|---|---|---|---|---|
| UQ-16 | `src/triangle.rs@1797-1824` | 56 | `impl Intersectable for Triangle::local_intersect` | `p1_to_origin.cross(self.e1)` | `impl Tuple::cross (src/tuple.rs:131)` | `p1_to_origin`; sem tipo escrito no grafo | `impl Tuple::cross (src/tuple.rs:131)` | correta | alta | `let p1_to_origin = local_ray.origin - self.p1;` (`src/triangle.rs:49`); `local_ray: Ray` (`:38`), `Ray.origin: Tuple` (`src/ray.rs:6`), `Triangle.p1: Tuple` (`src/triangle.rs:11`), `impl ops::Sub for Tuple` com `Output = Self` (`src/tuple.rs:66-67`) |  |
| UQ-17 | `src/object.rs@5613-5642` | 161 | `impl Intersectable for Object::bounds` | `bounds.add_point(triangle.p1)` | `impl Bounds::add_point (src/bounds.rs:23)` | `bounds`; `Bounds` (caminho), a classe do alvo | `impl Bounds::add_point (src/bounds.rs:23)` | correta | alta | = RT-13, validada (18/11). `let mut bounds = Bounds::empty();` (`src/object.rs:160`) |  |
| UQ-18 | `src/bin/cena_video.rs@4615-4642` | 109 | `main` | `sorteio.between(0.15, 0.32)` | `impl Sorteio::between (src/bin/cena_video.rs:33)` | `sorteio`; sem tipo escrito no grafo | `impl Sorteio::between (src/bin/cena_video.rs:33)` | correta | alta | `let mut sorteio = Sorteio(2026);` (`src/bin/cena_video.rs:104`), struct de tupla (`:26`) |  |
| UQ-19 | `src/camera.rs@4218-4231` | 129 | `impl Camera::benchmark_render` | `ordered.len()` | `impl Groups::len (src/groups.rs:60)` | `ordered`; sem tipo escrito no grafo | externo: `Vec::len` | **link_falso** (conhecido) | alta | Um dos 18 conhecidos (PLANO, 2.4d). `let mut timings = Vec::with_capacity(samples);` (`src/camera.rs:111`), `timings.push(start.elapsed().as_nanos())` (`:121`), `let mut ordered = timings.clone();` (`:125`): `Vec<u128>` | sim |
| UQ-20 | `src/obj.rs@5781-5804` | 143 | `build_bvh` | `right.bounds().center()` | `impl Bounds::center (src/bounds.rs:39)` | `right.bounds()`; sem tipo escrito no grafo | `impl Bounds::center (src/bounds.rs:39)` | correta | alta | `fn build_bvh(mut objects: Vec<Object>)` (`src/obj.rs:123`), `objects.sort_by(\|left, right\| ..)` (`:141`): `right: &Object`; `Object::bounds(&self) -> Bounds` (`src/object.rs:157`) |  |
| UQ-21 | `src/camera.rs@1583-1602` | 55 | `impl Camera::new` | `transform.inverse()` | `impl Matrix<4>::inverse (src/matrix.rs:193)` | `transform`; `Matrix` (caminho), a classe do alvo | `impl Matrix<4>::inverse (src/matrix.rs:193)` | correta | alta | `let transform = Matrix::identity();` (`src/camera.rs:54`); `identity` só existe em `impl Matrix<4>` (`src/matrix.rs:70-71`) |  |
| UQ-22 | `src/bin/cena_video.rs@4661-4687` | 110 | `main` | `sorteio.between(-1.8, 2.5)` | `impl Sorteio::between (src/bin/cena_video.rs:33)` | `sorteio`; sem tipo escrito no grafo | `impl Sorteio::between (src/bin/cena_video.rs:33)` | correta | alta | Como UQ-18 (`src/bin/cena_video.rs:104`) |  |
| UQ-23 | `src/bin/cena_video.rs@1517-1544` | 42 | `hue` | `k.min(4. - k).clamp(0., 1.)` | `impl Color::clamp (src/color.rs:24)` | `k.min(4. - k)`; sem tipo escrito no grafo | externo: `f64::clamp` | **link_falso** (novo) | alta | `fn hue(h: f64)` (`src/bin/cena_video.rs:39`), `\|offset: f64\|` (`:40`), `let k = (h * 6. + offset) % 6.;` (`:41`): `k` é `f64`, `f64::min` devolve `f64`, e `.clamp(0., 1.)` é o `f64::clamp` da biblioteca padrão. `Color::clamp` é de `Color` (`src/color.rs:24`). Não está entre os 18 conhecidos | sim |
| UQ-24 | `src/intersection.rs@1158-1189` | 41 | `impl Computations::from` | `ray.direction.reflect(normal_v)` | `impl Tuple::reflect (src/tuple.rs:33)` | `ray.direction`; sem tipo escrito no grafo | `impl Tuple::reflect (src/tuple.rs:33)` | correta | alta | `fn from(intersection: Intersection, ray: Ray)` (`src/intersection.rs:22`), `Ray.direction: Tuple` (`src/ray.rs:7`) |  |
| UQ-25 | `src/camera.rs@7484-7532` | 207 | `render_external_benchmark` | `camera.benchmark_render(world, warmups, samples)` | `impl Camera::benchmark_render (src/camera.rs:110)` | `camera`; sem tipo escrito no grafo | `impl Camera::benchmark_render (src/camera.rs:110)` | correta | alta | `let (camera, world) = external_benchmark_scene(width, height);` (`src/camera.rs:206`), `-> (Camera, World)` (`:162`) |  |
| UQ-26 | `src/transformations.rs@345-363` | 12 | `impl Transform for Tuple::view_transform` | `forward.cross(upn)` | `impl Tuple::cross (src/tuple.rs:131)` | `forward`; sem tipo escrito no grafo | `impl Tuple::cross (src/tuple.rs:131)` | correta | alta | `impl Transform for Tuple` (`src/transformations.rs:8`), `fn view_transform(self, to: Tuple, ..)` (`:9`), `let forward = (to - self).normalize();` (`:10`); `Sub` dá `Tuple` (`src/tuple.rs:66-67`), `normalize(&self) -> Self` (`:123`) |  |
| UQ-27 | `src/tuple.rs@1244-1260` | 54 | `impl ops::Add for Tuple::add` | `other.is_point()` | `impl Tuple::is_point (src/tuple.rs:29)` | `other`; `Tuple` (parâmetro), a classe do alvo | `impl Tuple::is_point (src/tuple.rs:29)` | correta | alta | `impl ops::Add for Tuple` (`src/tuple.rs:50`), `fn add(self, other: Self)` (`:53`) |  |
| UQ-28 | `src/intersection.rs@719-755` | 27 | `impl Computations::from` | `intersection.object.normal_at(point)` | `Intersectable::normal_at (src/object.rs:35)` | `intersection.object`; sem tipo escrito no grafo | `Intersectable::normal_at (src/object.rs:35)` | correta | alta | `Intersection.object: Object` (`src/intersection.rs:68`); `impl Intersectable for Object` (`src/object.rs:90`) não define `normal_at`, e nenhum outro `impl` o define (só `src/object.rs:35`), nem há `impl Object { .. }` solto: roda o método padrão do trait | trechos |
| UQ-29 | `src/intersection.rs@624-652` | 25 | `impl Computations::from` | `ray.position(intersection.t)` | `impl Ray::position (src/ray.rs:18)` | `ray`; `Ray` (parâmetro), a classe do alvo | `impl Ray::position (src/ray.rs:18)` | correta | alta | `ray: Ray` (`src/intersection.rs:22`) |  |
| UQ-30 | `src/object.rs@1394-1418` | 40 | `Intersectable::normal_at` | `world_normal.normalize()` | `impl Tuple::normalize (src/tuple.rs:123)` | `world_normal`; sem tipo escrito no grafo | `impl Tuple::normalize (src/tuple.rs:123)` | correta | alta | `let mut world_normal = self.transform().inverse().transpose() * local_normal;` (`src/object.rs:38`); `fn transform(&self) -> Matrix<4>` (`:18`); `inverse` e `transpose` devolvem `Self` (`src/matrix.rs:193`, `:81`); `impl ops::Mul<Tuple> for Matrix<4>` com `Output = Tuple` (`:276-277`) | trechos |

**Indecisas:** nenhuma.

## 4. Taxa de link falso

> **Sobre a verdade validada pelo usuário em 25/11/2026.** Com 15 chamadas por linguagem, o intervalo é largo demais para comparar com o limite da condição da Fase 2B, que, pelo PLANO, é medido pelo gabarito do 2.4e em todas as chamadas, não em amostra. Esta é só uma primeira estimativa.

Intervalo exato de Clopper-Pearson, 95%:

| | `link_falso` / sorteadas | Taxa | IC 95% | Em chamadas da população (só ilustração) |
|---|---|---|---|---|
| Python | 1 / 15 (UQ-4) | 6,7% | 0,2% a 31,9% | ~0 a ~74 de 233 |
| Rust | 2 / 15 (UQ-19 conhecido, UQ-23 novo) | 13,3% | 1,7% a 40,5% | ~2 a ~38 de 94 |
| Rust sem o conhecido sorteado | 1 / 15 | 6,7% | 0,2% a 31,9% | — |

Não somei as duas linguagens: as populações têm tamanhos diferentes (233 e 94), e uma soma simples pesaria o Rust demais.

**Piso pela contagem, não pela amostra.** Os falsos já identificados dão um mínimo exato em cada população, maior que a estimativa pontual do Python:
- **Python, pelo menos 21 de 233 (9,0%), verificado:** os 4 de `Ping.get_body` e os 17 links de `hc/lib/curl.py` para o dublê `FakeCurl`. Conferência: 17 chamadas em `c`, todas dentro de `request` (da linha 55 até antes do `def get`, na linha 210): `c.setopt` nas linhas 139, 140, 141, 142, 143, 145, 149, 152, 165, 169, 175, 176, 177, 179 e 182; `c.perform` na 185; `c.getinfo` na 203 (HC-27); um único `c =`, na 138 (`pycurl.Curl()`); os alvos `FakeCurl.setopt`, `.perform` e `.getinfo` em `hc/lib/tests/test_curl.py:18`, `:21` e `:39`. Conferido pela sessão com a saída do comando (`Select-String` sobre os dois arquivos) e validado pelo usuário em 25/11/2026.
- **Rust, pelo menos 11 de 94 (11,7%):** os 10 conhecidos e a UQ-23. Os 10 vêm do 2.4d, e só a UQ-19 foi revista agora. Os outros 2 `Color::clamp` da população (`src/canvas.rs:50` e `:97`, `pixel.clamp(..)`) não foram conferidos; no `:97`, o grafo dá ao receptor o tipo `Color` (a conferir no 2.4e).

A estimativa do Python (6,7%) fica abaixo do piso (9,0%), porque os falsos do Python estão concentrados num arquivo: é o efeito de 15 chamadas, não uma contradição.

## 5. Decisões do usuário (25/11/2026)

As propostas da Parada 1 foram decididas assim:

- **P1. Fixture, sem o portão ler.** Gravada em [tools/precisao/fixtures/estrato-unique.json](../../../tools/precisao/fixtures/estrato-unique.json): UQ-1 a UQ-30, com a verdade, a certeza, a justificativa e o `verdade_validada_por`. O portão **não** lê o arquivo nesta parada; `portao.js` e `lib.js` não mudaram. Ligar a fixture ao portão (só medir, tabela à parte, com prova de veredito igual) fica para a Parada 1 do 2.4e ou do 2.4f (registrado no [README do portão](../../../tools/precisao/README.md)). As três que já estão na amostra (UQ-1 = HC-26, UQ-13 = HC-143, UQ-17 = RT-13) não podem contar duas vezes quando isso for feito.
- **P2. Fora da estimativa ponderada.** Esta população entra junto com o estrato `type`, refazendo os estratos de uma vez, antes da Fase 2B (pendência no ESTADO).
- **P3. PLANO, 2.4e:** (a) as duas contagens, dizendo qual é qual: 233 e 94 pela definição do portão; 655 e 245 com testes e `self.m()`; (b) duas famílias a medir: o dublê de teste ligado por código de produção (17 de `FakeCurl`, com a HC-27) e o método da biblioteca padrão do Rust com nome de método do projeto (`len`, `next` e `clamp`, este pela UQ-23); (c) medir a taxa nesta mesma população e comparar com o estrato; (d) a conferir, não links falsos conhecidos: os dois `pixel.clamp` de `src/canvas.rs:50` e `:97`, e os nomes `first`, `last` e `get` chamados sobre `Vec` (ex.: `src/camera.rs`, em `benchmark_render`), se algum tiver homônimo no projeto.
- **P4. PLANO, 2.4f, registrado sem decidir:** a opção (d), não ligar uma chamada de produção a um método declarado em arquivo de teste, que só pode ser escolhida com a contagem de quantas corretas ela derruba; e `clamp` na lista de nomes da opção (b).
- **P5. A condição da 2B não muda.** O piso (Python pelo menos 21 de 233, 9,0%, verificado; Rust pelo menos 11 de 94, 11,7%) fica no ESTADO como referência para o teto do "só pelo nome", fixado depois do 2.4e.

## 6. O que mudou e o que não mudou

- **Novos:** o script [estrato-unique.js](../../../tools/precisao/estrato-unique.js), a fixture [estrato-unique.json](../../../tools/precisao/fixtures/estrato-unique.json), o esqueleto do sorteio em [dados/estrato-unique/](dados/estrato-unique/) e esta folha.
- **Documentos atualizados:** [docs/IDS.md](../../IDS.md) (prefixo UQ), o [ESTADO](../../../ESTADO-ATUAL.md) (pendência "Antes do 2.4e" fechada, próximo passo, estimativa ponderada e o piso), o [PLANO](../../../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md) (2.4e e 2.4f), o [README do portão](../../../tools/precisao/README.md) e uma seção datada no [CTX](../../../CONTEXTO-CONTINUIDADE.md).
- **Não mudou:** nenhuma regra de resolução, código de `crates/`, `portao.js`, `lib.js`, linha de base, fixture que o portão lê ou fato do acervo. As fontes continuam `3aeb2fa7…`, e o `.studio` original não mudou.
