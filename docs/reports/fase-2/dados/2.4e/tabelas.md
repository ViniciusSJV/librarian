
## Python (Healthchecks)

Ocorrências de chamada: 16239 (fora de testes: 5243).

### Classificação por gabarito (fora de testes)

| Categoria | execução | tipos | combinado |
| --- | --- | --- | --- |
| concorda | 1104 | 1131 | 1202 |
| concorda_externo | 3342 | 3596 | 3731 |
| concorda_pela_base | 0 | 3 | 0 |
| gabaritos_divergem | 0 | 0 | 13 |
| ligacao_que_falta | 11 | 7 | 13 |
| link_falso_candidato | 13 | 37 | 30 |
| nao_viu | 556 | 317 | 89 |
| nao_viu_duble | 72 | 0 | 0 |
| possivel_com_o_certo | 28 | 29 | 33 |
| possivel_sem_o_certo | 117 | 123 | 132 |

### Populações (Q8)

| População | Ocorrências | Vistas por algum gabarito | Discordâncias (link falso candidato ou polimórfica) | Taxa entre as vistas | IC 95% |
| --- | --- | --- | --- | --- | --- |
| Universo (todas) | 16239 | 16094 | 33 | 0,2% | 0,1% a 0,3% |
| Fora de testes | 5243 | 5154 | 30 | 0,6% | 0,4% a 0,8% |
| Recorte do lista.js (655/245) | 655 | 654 | 33 | 5,0% | 3,5% a 7,0% |
| Recorte do portão (233/94) | 233 | 232 | 30 | 12,9% | 8,9% a 17,9% |

### Tabela 1: alcance de cada workflow

| Workflow | Entradas | Símbolos alcançados | Pelos dois | Só pelo grafo | Só pela execução | Chamadas no alcance | Cobertura execução | Cobertura tipos | Vistas por algum | Marcos | Perda (falhas de ambiente) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A. Cadastro de Check (API) | checks (hc/api/views.py:479); create_check (hc/api/views.py:455) | 70 | 18 | 0 | 52 | 346 | 325 de 346 (93,9%) | 337 de 346 (97,4%) | 344 de 346 (99,4%) | 4/4 | — |
| B. Recebimento de ping | ping (hc/api/views.py:196); ping_by_slug (hc/api/views.py:264); _process_message (hc/api/management/commands/smtpd.py:49) | 33 | 20 | 13 | 0 | 166 | 140 de 166 (84,3%) | 156 de 166 (94,0%) | 164 de 166 (98,8%) | 2/2 | — |
| C. Detecção de atraso | Command.handle (hc/api/management/commands/sendalerts.py:184); Command.handle_going_down (hc/api/management/commands/sendalerts.py:123) | 7 | 6 | 1 | 0 | 51 | 35 de 51 (68,6%) | 48 de 51 (94,1%) | 48 de 51 (94,1%) | 2/2 | — |
| D. Flip e notificações | Command.process_one_flip (hc/api/management/commands/sendalerts.py:91); notify (hc/api/management/commands/sendalerts.py:24) | 180 | 7 | 0 | 153 | 802 | 671 de 802 (83,7%) | 727 de 802 (90,6%) | 791 de 802 (98,6%) | 2/3 (fora: Channel.transport) | signal: 25 de 45 chamadas sem execução |

- A: só pelo grafo, exemplos: —; só pela execução: cors, cors.decorator, authorize.wrapper, authorize_read.wrapper, authorize, Check.to_dict, authorize_read, error

- B: só pelo grafo, exemplos: put_object, Check.prune, Ping.get_body, enc, client, remove_objects, Ping.get_body_bytes, get_object; só pela execução: —

- C: só pelo grafo, exemplos: Command.process_one_flip; só pela execução: —

- D: só pelo grafo, exemplos: —; só pela execução: Channel.notify, Email.is_noop, Notification, Email.notify, Transport.is_noop, Group.notify, Sms.is_noop, Sms.notify


### Tabela 2: lista de lacunas, por família (ordenada pelas chamadas afetadas)

| # | Família | Chamadas (produção / teste) | Tipos de discordância | Por gabarito | Cai num workflow | Link falso candidato em workflow | Exemplos |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | sem família: `homonym|nome|externo|possivel_sem_o_certo` | 2166 (132 / 2034) | possivel_sem_o_certo 3272 | execução 2119, tipos 1153 | C, D, B, A | 0 | hc/api/management/commands/sendalerts.py:86 `self.stdout.write(logs)` → LogSink.write, NullSink.write; execucao externo OutputWrapper.write; tipos —<br>hc/api/management/commands/sendalerts.py:215 `self.executor.shutdown(wait=True)` → MockSocket.shutdown, MockSocket.shutdown; execucao —; tipos externo concurrent.futures._base.Executor.shutdown<br>hc/api/management/commands/prunepingsslow.py:29 `self.stdout.write(f"Pruning: {check.code}")` → LogSink.write, NullSink.write; execucao externo OutputWrapper.write; tipos — |
| 2 | sem família: `above_candidate_limit|nome|projeto|ligacao_que_falta` | 344 (7 / 337) | ligacao_que_falta 686 | execução 342, tipos 344 | D | 0 | hc/api/management/commands/sendalerts.py:43 `ch.notify(flip)` → above_candidate_limit; execucao Channel.notify; tipos Channel.notify<br>hc/front/views.py:1333 `channel.notify(dummy_flip, is_test=True)` → above_candidate_limit; execucao Channel.notify; tipos Channel.notify<br>hc/front/views.py:1339 `channel.notify(dummy_flip, is_test=True)` → above_candidate_limit; execucao Channel.notify; tipos Channel.notify |
| 3 | FM-1 Dublê de teste ligado por código de produção | 17 (17 / 0) | link_falso_candidato 17 | tipos 17 | D | 17 | hc/lib/curl.py:139 `c.setopt(pycurl.NOSIGNAL, 1)` → FakeCurl.setopt; execucao —; tipos externo pycurl._pycurl.Curl.setopt<br>hc/lib/curl.py:140 `c.setopt(pycurl.PROTOCOLS, pycurl.PROTO_HTTP | pycurl.PROTO_` → FakeCurl.setopt; execucao —; tipos externo pycurl._pycurl.Curl.setopt<br>hc/lib/curl.py:141 `c.setopt(pycurl.OPENSOCKETFUNCTION, opensocket)` → FakeCurl.setopt; execucao —; tipos externo pycurl._pycurl.Curl.setopt |
| 4 | FM-2 Método de biblioteca com o nome de um método do projeto | 16 (13 / 3) | link_falso_candidato 23 | execução 16, tipos 7 | D, B | 4 | hc/api/transports.py:117 `q.last()` → ChannelsAdmin.last; execucao externo QuerySet.last; tipos externo django.db.models.query.QuerySet.last<br>hc/api/management/commands/smtpd.py:37 `message.get_body(("plain",))` → Ping.get_body; execucao externo MIMEPart.get_body; tipos externo email.message.MIMEPart.get_body<br>hc/api/management/commands/smtpd.py:41 `message.get_body(("html",))` → Ping.get_body; execucao externo MIMEPart.get_body; tipos externo email.message.MIMEPart.get_body |
| 5 | sem família: `no_candidate_in_project|simples|projeto|ligacao_que_falta` | 7 (6 / 1) | ligacao_que_falta 7 | execução 7 | D, A | 0 | hc/api/models.py:1150 `cls` → no_candidate_in_project; execucao Email|Group|Sms|Webhook|Apprise|Call|Discord|GitHub|GoogleChat|Gotify|Matrix|Mattermost|MsTeamsWorkflow|Ntfy|Opsgenie|PagerTree|PagerDuty|Pushover|Pushbullet|RocketChat|Shell|Signal|Slack|Spike|Telegram|Trello|VictorOps|WhatsApp|Zulip; tipos —<br>hc/api/decorators.py:91 `f` → no_candidate_in_project; execucao get_checks|badges|get_check|get_check_by_unique_key|flips_by_uuid|flips_by_unique_key; tipos —<br>hc/api/decorators.py:108 `f` → no_candidate_in_project; execucao checks|badge|check_badge|single|badges|get_check_by_unique_key|flips_by_uuid|flips_by_unique_key|ping_body|pings|channels|pause|resume; tipos — |
| 6 | sem família: `homonym|self|externo|possivel_sem_o_certo` | 7 (0 / 7) | possivel_sem_o_certo 14 | execução 7, tipos 7 | — | 0 | hc/api/tests/test_get_pings.py:84 `self.get().json()` → Response.json, Channel.json; execucao externo functools.partial; tipos externo django.test.client._MonkeyPatchedWSGIResponse.json<br>hc/api/tests/test_get_pings.py:97 `self.get().json()` → Response.json, Channel.json; execucao externo functools.partial; tipos externo django.test.client._MonkeyPatchedWSGIResponse.json<br>hc/api/tests/test_get_pings.py:105 `self.get().json()` → Response.json, Channel.json; execucao externo functools.partial; tipos externo django.test.client._MonkeyPatchedWSGIResponse.json |
| 7 | sem família: `homonym|expressão|externo|possivel_sem_o_certo` | 1 (0 / 1) | possivel_sem_o_certo 1 | execução 1 | D | 0 | hc/lib/tests/test_curl.py:37 `self.opts[pycurl.WRITEDATA].write(b"hello world")` → LogSink.write, NullSink.write; execucao externo builtins.BytesIO.write; tipos — |

### Tabela 3: para fixar o X

| Família | Chamadas | Acumulado | Link falso candidato dentro de um workflow? |
| --- | --- | --- | --- |
| `homonym|nome|externo|possivel_sem_o_certo` | 2166 | 2166 | não |
| `above_candidate_limit|nome|projeto|ligacao_que_falta` | 344 | 2510 | não |
| FM-1 | 17 | 2527 | sim (17) |
| FM-2 | 16 | 2543 | sim (4) |
| `no_candidate_in_project|simples|projeto|ligacao_que_falta` | 7 | 2550 | não |
| `homonym|self|externo|possivel_sem_o_certo` | 7 | 2557 | não |
| `homonym|expressão|externo|possivel_sem_o_certo` | 1 | 2558 | não |

- 2 famílias acima do X: X entre 17 e 343
- 3 famílias acima do X: X entre 16 e 16
- 4 famílias acima do X: X entre 7 e 15


### Tabela 4: para fixar o teto (fora de testes; grupo provisório, Q9)

| Grupo provisório | Ligações | Vistas por algum gabarito | Vistas pela execução | Só pelos tipos | Discordâncias (candidatas) | Gabaritos divergem | Taxa entre as vistas | IC 95% | Piso do UQ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| outras | 686 | 686 | 613 | 73 | 0 | 13 | 0,0% | 0,0% a 0,5% | — |
| provavel pelo nome | 442 | 441 | 403 | 38 | 30 | 0 | 6,8% | 4,6% a 9,6% | ≥ 21 de 233 |
| provavel pelo tipo | 121 | 118 | 101 | 17 | 0 | 0 | 0,0% | 0,0% a 3,1% | — |

### Tabela 5: cobertura por família e por workflow

| Conjunto | Ocorrências | Vistas execução | Vistas tipos | Vistas por algum | 3 ÷ vistas (menor teto verificável com 0 discordâncias) |
| --- | --- | --- | --- | --- | --- |
| workflow A | 346 | 325 | 337 | 344 | 0,9% |
| workflow B | 166 | 140 | 156 | 164 | 1,8% |
| workflow C | 51 | 35 | 48 | 48 | 6,3% |
| workflow D | 802 | 671 | 727 | 791 | 0,4% |

### Tabela 6: teto do caminho

| Workflow | Profundidade (união) | Nós reais (união) | Passa de 6 camadas? | Passa de 300 nós? | Passa de 120 nós (o Studio)? | Profundidade só pelo grafo | Nós só pelo grafo |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A | 6 | 70 | não | não | não | 3 | 18 |
| B | 5 | 33 | não | não | não | 5 | 33 |
| C | 2 | 7 | não | não | não | 2 | 7 |
| D | 8 | 180 | sim | não | sim | 4 | 7 |

### Calibração: o gabarito contra a verdade validada (fixtures)

| Gabarito | Itens | Não viu | Concorda com a verdade | Discorda da verdade |
| --- | --- | --- | --- | --- |
| execução | 184 | 31 | 153 | 0 |
| tipos | 184 | 37 | 144 | 3 |

- tipos, discordâncias: HC-79 hc/api/management/commands/smtpd.py:122: verdade ["NullSink.write (hc/api/tests/test_smtpd.py:43)","Django: OutputWrapper.write"]; gabarito LogSink.write (hc/api/management/commands/smtpd.py:29); HC-113 hc/settings.py:69: verdade "envsecret (hc/settings.py:49)"; gabarito envsecret (hc/settings.py:41); HC-114 hc/settings.py:344: verdade "envsecret (hc/settings.py:49)"; gabarito envsecret (hc/settings.py:41)


### Conhecidos (PLANO, 2.4d; UQ)

| Local | Texto | Alvo do Librarian | Teste? | execução | tipos |
| --- | --- | --- | --- | --- | --- |
| hc/api/transports.py:38 | `ping.get_body_bytes()` | Ping.get_body_bytes | não | concorda | concorda |
| hc/api/transports.py:29 | `ping.get_body_bytes()` | Ping.get_body_bytes | não | concorda | concorda |
| hc/lib/curl.py:139 | `c.setopt(pycurl.NOSIGNAL, 1)` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:140 | `c.setopt(pycurl.PROTOCOLS, pycurl.PROTO_HTTP | pyc` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:141 | `c.setopt(pycurl.OPENSOCKETFUNCTION, opensocket)` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:142 | `c.setopt(pycurl.FOLLOWLOCATION, True)` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:143 | `c.setopt(pycurl.MAXREDIRS, 3)` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:145 | `c.setopt(pycurl.TIMEOUT, timeout)` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:149 | `c.setopt(pycurl.URL, url.encode())` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:152 | `c.setopt(pycurl.USERPWD, "%s:%s" % auth)` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:165 | `c.setopt(pycurl.HTTPHEADER, headers_list)` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:169 | `c.setopt(pycurl.POSTFIELDS, urlencode(data))` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:175 | `c.setopt(pycurl.UPLOAD, 1)` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:176 | `c.setopt(pycurl.INFILESIZE, len(data))` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:177 | `c.setopt(pycurl.READDATA, BytesIO(data))` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:179 | `c.setopt(pycurl.CUSTOMREQUEST, method.upper())` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:182 | `c.setopt(pycurl.WRITEDATA, buffer)` | FakeCurl.setopt | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:185 | `c.perform()` | FakeCurl.perform | não | nao_viu_duble | link_falso_candidato |
| hc/lib/curl.py:203 | `c.getinfo(pycurl.RESPONSE_CODE)` | FakeCurl.getinfo | não | nao_viu_duble | link_falso_candidato |
| hc/api/management/commands/smtpd.py:37 | `message.get_body(("plain",))` | Ping.get_body | não | link_falso_candidato | link_falso_candidato |
| hc/api/management/commands/smtpd.py:41 | `message.get_body(("html",))` | Ping.get_body | não | link_falso_candidato | link_falso_candidato |
| hc/front/views.py:769 | `ping.get_body_bytes()` | Ping.get_body_bytes | não | concorda | concorda |
| hc/front/views.py:800 | `parsed.get_body(("plain",))` | Ping.get_body | não | link_falso_candidato | link_falso_candidato |
| hc/front/views.py:805 | `parsed.get_body(("html",))` | Ping.get_body | não | link_falso_candidato | link_falso_candidato |
| hc/front/views.py:819 | `ping.get_body_bytes()` | Ping.get_body_bytes | não | concorda | concorda |
| hc/api/views.py:655 | `ping.get_body_bytes()` | Ping.get_body_bytes | não | concorda | concorda |
| hc/api/models.py:774 | `self.get_body_bytes()` | Ping.get_body_bytes | não | nao_viu | concorda |
| hc/integrations/rocketchat/transport.py:49 | `ping.get_body_size()` | Ping.get_body_size | não | concorda | concorda |

### Só do gabarito e extras

```
{
 "so_do_gabarito": {
  "execucao": {
   "total": 982,
   "projeto": 71,
   "exemplos": [
    "hc/settings.py:153 → envbool",
    "hc/accounts/views.py:579 → require_sudo_mode",
    "hc/accounts/views.py:602 → require_sudo_mode",
    "hc/accounts/views.py:684 → require_sudo_mode",
    "hc/accounts/views.py:719 → require_sudo_mode",
    "hc/accounts/views.py:755 → require_sudo_mode",
    "hc/accounts/views.py:790 → require_sudo_mode",
    "hc/accounts/views.py:804 → require_sudo_mode",
    "hc/api/views.py:430 → authorize_read",
    "hc/api/views.py:455 → authorize",
    "hc/api/views.py:490 → authorize",
    "hc/api/views.py:497 → authorize_read",
    "hc/api/views.py:508 → authorize_read",
    "hc/api/views.py:516 → authorize",
    "hc/api/views.py:537 → authorize"
   ]
  },
  "tipos": {
   "total": 1135,
   "projeto": 9,
   "exemplos": [
    "hc/front/templatetags/hc_extras.py:199 name_then_code → Check.name_then_code",
    "hc/settings.py:153 envbool → envbool",
    "hc/settings.py:229 envsecret → envsecret",
    "hc/settings.py:230 envint → envint",
    "hc/settings.py:250 envsecret → envsecret",
    "hc/settings.py:327 envint → envint",
    "hc/settings.py:328 envbool → envbool",
    "hc/settings.py:329 envbool → envbool",
    "hc/settings.py:331 envsecret → envsecret"
   ]
  }
 },
 "fora_da_edition": 0,
 "extras": {
  "pares_em_outra_thread": 8
 },
 "por_resolucao": {
  "no_candidate_in_project (teste)": 7147,
  "unique (teste)": 817,
  "homonym (teste)": 2083,
  "type (teste)": 167,
  "no_candidate_in_project": 3780,
  "unique": 1081,
  "above_candidate_limit": 26,
  "homonym": 188,
  "above_candidate_limit (teste)": 377,
  "type": 121,
  "receiver (teste)": 313,
  "import": 16,
  "receiver": 17,
  "scope": 14,
  "import (teste)": 48,
  "scope (teste)": 44
 }
}
```

## Rust (ray tracer)

Ocorrências de chamada: 2545 (fora de testes: 1106).

### Classificação por gabarito (fora de testes)

| Categoria | execução | tipos | combinado |
| --- | --- | --- | --- |
| casou_ambiguo | 93 | 0 | 0 |
| concorda | 370 | 524 | 524 |
| concorda_externo | 0 | 456 | 456 |
| concorda_pela_base | 0 | 3 | 0 |
| ligacao_que_falta | 6 | 24 | 24 |
| link_falso_candidato | 0 | 12 | 12 |
| nao_viu | 617 | 13 | 13 |
| possivel_com_o_certo | 20 | 70 | 73 |
| possivel_sem_o_certo | 0 | 4 | 4 |

### Populações (Q8)

| População | Ocorrências | Vistas por algum gabarito | Discordâncias (link falso candidato ou polimórfica) | Taxa entre as vistas | IC 95% |
| --- | --- | --- | --- | --- | --- |
| Universo (todas) | 2545 | 2503 | 12 | 0,5% | 0,2% a 0,8% |
| Fora de testes | 1106 | 1093 | 12 | 1,1% | 0,6% a 1,9% |
| Recorte do lista.js (655/245) | 245 | 241 | 12 | 5,0% | 2,6% a 8,5% |
| Recorte do portão (233/94) | 94 | 94 | 12 | 12,8% | 6,8% a 21,2% |

### Tabela 1: alcance de cada workflow

| Workflow | Entradas | Símbolos alcançados | Pelos dois | Só pelo grafo | Só pela execução | Chamadas no alcance | Cobertura execução | Cobertura tipos | Vistas por algum | Marcos | Arestas perdidas (rayon) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CR-1. Render de uma cena | impl Camera::render (src/camera.rs:95) | 112 | 4 | 53 | 0 | 380 | 178 de 380 (46,8%) | 380 de 380 (100,0%) | 380 de 380 (100,0%) | 6/6 | 3: impl Camera::ray_from_pixel (src/camera.rs:81), impl World::color_at (src/world.rs:109), impl Canvas::set_pixel_color (src/canvas.rs:23) |
| CR-2. Malha OBJ com BVH | load_bvh (src/obj.rs:164) | 35 | 24 | 1 | 8 | 110 | 48 de 110 (43,6%) | 110 de 110 (100,0%) | 110 de 110 (100,0%) | 2/2 | 0 |
| CR-3. Gravação da imagem | impl Canvas::to_png (src/canvas.rs:111); impl Canvas::to_ppm (src/canvas.rs:84) | 9 | 9 | 0 | 0 | 53 | 8 de 53 (15,1%) | 53 de 53 (100,0%) | 53 de 53 (100,0%) | 2/2 | 0 |

- CR-1: só pelo grafo, exemplos: impl Camera::ray_from_pixel, impl World::color_at, impl Canvas::set_pixel_color, impl Tuple::point, impl Ray::new, impl Tuple::normalize, impl World::intersect_world, impl Intersection::prepare_computations; só pela execução: —

- CR-2: só pelo grafo, exemplos: impl Sorteio::next; só pela execução: impl Intersectable for Object::bounds, impl ops::Sub for Tuple::sub, impl From<Triangle> for Object::from, impl Groups::bounds, impl Bounds::transformed, impl Tuple::new, impl ops::Mul<Tuple> for Matrix<4>::mul, impl ops::Div<f64> for Tuple::div

- CR-3: só pelo grafo, exemplos: —; só pela execução: —


### Tabela 2: lista de lacunas, por família (ordenada pelas chamadas afetadas)

| # | Família | Chamadas (produção / teste) | Tipos de discordância | Por gabarito | Cai num workflow | Link falso candidato em workflow | Exemplos |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | FM-5 `T::from` com vários `impl From<X>` | 95 (15 / 80) | ligacao_que_falta 129 | execução 34, tipos 95 | CR-1, CR-2 | 0 | src/bin/cena_video.rs:91 `Patterns::from` → above_candidate_limit; execucao —; tipos impl From<Checkers> for Patterns::from<br>src/bin/cena_video.rs:155 `Object::from` → above_candidate_limit; execucao —; tipos impl From<Sphere> for Object::from<br>src/cylinder.rs:39 `Object::from` → above_candidate_limit; execucao impl From<Cylinder> for Object::from; tipos impl From<Cylinder> for Object::from |
| 2 | FM-2 Método de biblioteca com o nome de um método do projeto | 12 (12 / 0) | link_falso_candidato 12 | tipos 12 | CR-2, CR-1 | 6 | src/bin/cena_video.rs:42 `k.min(4. - k).clamp(0., 1.)` → impl Color::clamp; execucao —; tipos externo core https://github.com/rust-lang/rust/library/core f64/impl#[f64]clamp().<br>src/obj.rs:63 `f.next()` → impl Sorteio::next; execucao —; tipos externo core https://github.com/rust-lang/rust/library/core str/iter/impl#[`SplitWhitespace<'a>`][Iterator]next().<br>src/obj.rs:66 `fields.next()` → impl Sorteio::next; execucao —; tipos externo core https://github.com/rust-lang/rust/library/core str/iter/impl#[`SplitWhitespace<'a>`][Iterator]next(). |
| 3 | FM-4 Despacho pelo método padrão de trait | 5 (5 / 0) | ligacao_que_falta 6 | tipos 5, execução 1 | CR-1 | 0 | src/patterns.rs:15 `self.transform()` → above_candidate_limit; execucao impl Intersectable for Object::transform|impl Incuse for Stripe::transform|impl Incuse for Checkers::transform; tipos Incuse::transform<br>src/object.rs:36 `self.transform()` → above_candidate_limit; execucao impl Intersectable for Object::transform|impl Intersectable for Sphere::transform|impl Intersectable for Triangle::transform; tipos Intersectable::transform<br>src/object.rs:38 `self.transform()` → above_candidate_limit; execucao impl Intersectable for Object::transform|impl Intersectable for Sphere::transform|impl Intersectable for Triangle::transform; tipos Intersectable::transform |
| 4 | sem família: `above_candidate_limit|nome|projeto|ligacao_que_falta` | 3 (3 / 0) | ligacao_que_falta 4 | tipos 3, execução 1 | — | 0 | src/groups.rs:68 `child.set_transform(self.transform * child.transform())` → above_candidate_limit; execucao —; tipos impl Intersectable for Object::set_transform<br>src/groups.rs:68 `child.transform()` → above_candidate_limit; execucao —; tipos impl Intersectable for Object::transform<br>src/world.rs:21 `o.transform()` → above_candidate_limit; execucao impl Intersectable for Object::transform; tipos impl Intersectable for Object::transform |
| 5 | sem família: `homonym|nome|externo|possivel_sem_o_certo` | 2 (2 / 0) | possivel_sem_o_certo 2 | tipos 2 | CR-1 | 0 | src/intersection.rs:134 `self.data.into_iter()` → impl IntoIterator for Intersections::into_iter, impl IntoIterator for Groups::into_iter; execucao —; tipos externo alloc https://github.com/rust-lang/rust/library/alloc vec/impl#[`Vec<T, A>`][IntoIterator]into_iter().<br>src/groups.rs:42 `self.children.into_iter()` → impl IntoIterator for Intersections::into_iter, impl IntoIterator for Groups::into_iter; execucao —; tipos externo alloc https://github.com/rust-lang/rust/library/alloc vec/impl#[`Vec<T, A>`][IntoIterator]into_iter(). |
| 6 | sem família: `homonym|expressão|externo|possivel_sem_o_certo` | 2 (2 / 0) | possivel_sem_o_certo 2 | tipos 2 | — | 0 | src/count.rs:90 `sites` → impl IntoIterator for Intersections::into_iter, impl IntoIterator for Groups::into_iter; execucao —; tipos externo alloc https://github.com/rust-lang/rust/library/alloc vec/impl#[`Vec<T, A>`][IntoIterator]into_iter().<br>src/groups.rs:65 `self.children` → impl IntoIterator for Intersections::into_iter, impl IntoIterator for Groups::into_iter; execucao —; tipos externo alloc https://github.com/rust-lang/rust/library/alloc vec/impl#[`Vec<T, A>`][IntoIterator]into_iter(). |
| 7 | sem família: `above_candidate_limit|caminho|projeto|ligacao_que_falta` | 1 (1 / 0) | ligacao_que_falta 2 | execução 1, tipos 1 | — | 0 | src/camera.rs:192 `crate::object::Object::from` → above_candidate_limit; execucao impl From<Plane> for Object::from|impl From<Sphere> for Object::from; tipos impl From<Sphere> for Object::from |

### Tabela 3: para fixar o X

| Família | Chamadas | Acumulado | Link falso candidato dentro de um workflow? |
| --- | --- | --- | --- |
| FM-5 | 95 | 95 | não |
| FM-2 | 12 | 107 | sim (6) |
| FM-4 | 5 | 112 | não |
| `above_candidate_limit|nome|projeto|ligacao_que_falta` | 3 | 115 | não |
| `homonym|nome|externo|possivel_sem_o_certo` | 2 | 117 | não |
| `homonym|expressão|externo|possivel_sem_o_certo` | 2 | 119 | não |
| `above_candidate_limit|caminho|projeto|ligacao_que_falta` | 1 | 120 | não |

- 2 famílias acima do X: X entre 5 e 11
- 3 famílias acima do X: X entre 3 e 4
- 4 famílias acima do X: X entre 2 e 2


### Tabela 4: para fixar o teto (fora de testes; grupo provisório, Q9)

| Grupo provisório | Ligações | Vistas por algum gabarito | Vistas pela execução | Só pelos tipos | Discordâncias (candidatas) | Gabaritos divergem | Taxa entre as vistas | IC 95% | Piso do UQ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| outras | 210 | 210 | 150 | 60 | 0 | 0 | 0,0% | 0,0% a 1,7% | — |
| provavel pelo tipo | 190 | 190 | 121 | 69 | 0 | 0 | 0,0% | 0,0% a 1,9% | — |
| provavel pelo nome | 136 | 136 | 99 | 37 | 12 | 0 | 8,8% | 4,6% a 14,9% | ≥ 11 de 94 |

### Tabela 5: cobertura por família e por workflow

| Conjunto | Ocorrências | Vistas execução | Vistas tipos | Vistas por algum | 3 ÷ vistas (menor teto verificável com 0 discordâncias) |
| --- | --- | --- | --- | --- | --- |
| workflow CR-1 | 380 | 178 | 380 | 380 | 0,8% |
| workflow CR-2 | 110 | 48 | 110 | 110 | 2,7% |
| workflow CR-3 | 53 | 8 | 53 | 53 | 5,7% |

### Tabela 6: teto do caminho

| Workflow | Profundidade (união) | Nós reais (união) | Passa de 6 camadas? | Passa de 300 nós? | Passa de 120 nós (o Studio)? | Profundidade só pelo grafo | Nós só pelo grafo |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CR-1 | 9 | 112 | sim | não | não | 6 | 57 |
| CR-2 | 6 | 35 | não | não | não | 6 | 25 |
| CR-3 | 3 | 9 | não | não | não | 3 | 9 |

### Calibração: o gabarito contra a verdade validada (fixtures)

| Gabarito | Itens | Não viu | Concorda com a verdade | Discorda da verdade |
| --- | --- | --- | --- | --- |
| execução | 83 | 41 | 42 | 0 |
| tipos | 83 | 1 | 82 | 0 |


### Conhecidos (PLANO, 2.4d; UQ)

| Local | Texto | Alvo do Librarian | Teste? | execução | tipos |
| --- | --- | --- | --- | --- | --- |
| src/color.rs:147 | `color.clamp(0.0, 1.0)` | impl Color::clamp | sim | concorda | concorda |
| src/bin/cena_video.rs:42 | `k.min(4. - k).clamp(0., 1.)` | impl Color::clamp | não | nao_viu | link_falso_candidato |
| src/bin/cena_video.rs:34 | `self.next()` | impl Sorteio::next | não | nao_viu | concorda |
| src/obj.rs:63 | `f.next()` | impl Sorteio::next | não | nao_viu | link_falso_candidato |
| src/obj.rs:66 | `fields.next()` | impl Sorteio::next | não | nao_viu | link_falso_candidato |
| src/obj.rs:81 | `parts.next()` | impl Sorteio::next | não | nao_viu | link_falso_candidato |
| src/obj.rs:90 | `face.len()` | impl Groups::len | não | nao_viu | link_falso_candidato |
| src/obj.rs:28 | `field.split('/').next()` | impl Sorteio::next | não | nao_viu | link_falso_candidato |
| src/obj.rs:19 | `fields.next()` | impl Sorteio::next | não | nao_viu | link_falso_candidato |
| src/obj.rs:21 | `fields.next()` | impl Sorteio::next | não | nao_viu | link_falso_candidato |
| src/obj.rs:22 | `fields.next()` | impl Sorteio::next | não | nao_viu | link_falso_candidato |
| src/obj.rs:23 | `fields.next()` | impl Sorteio::next | não | nao_viu | link_falso_candidato |
| src/canvas.rs:50 | `pixel.clamp(0.0, 1.0)` | impl Color::clamp | não | concorda | concorda |
| src/canvas.rs:97 | `pixel.clamp(0.0, 1.0)` | impl Color::clamp | não | concorda | concorda |
| src/count.rs:133 | `time["librarian-measure cpu_ms=".len()..].split(' ` | impl Sorteio::next | sim | nao_viu | nao_viu |
| src/count.rs:133 | `"librarian-measure cpu_ms=".len()` | impl Groups::len | sim | nao_viu | nao_viu |
| src/count.rs:151 | `time["librarian-measure cpu_ms=".len()..].split(' ` | impl Sorteio::next | sim | nao_viu | nao_viu |
| src/count.rs:151 | `"librarian-measure cpu_ms=".len()` | impl Groups::len | sim | nao_viu | nao_viu |
| src/camera.rs:129 | `ordered.len()` | impl Groups::len | não | nao_viu | link_falso_candidato |

### Só do gabarito e extras

```
{
 "so_do_gabarito": {
  "tipos": {
   "total": 638,
   "por_tipo": {
    "use (produção)": 4,
    "operador do projeto (produção)": 255,
    "dentro de macro (produção)": 43,
    "referência como valor (produção)": 2,
    "use (teste)": 2,
    "dentro de macro (teste)": 272,
    "referência como valor (teste)": 1,
    "operador do projeto (teste)": 59
   },
   "exemplos": {
    "use": [
     "src/bin/librarian_measure_mesh.rs:16 use renderer::obj::load_bvh;",
     "src/bin/cena_video.rs:16 use renderer::obj::load_bvh_smooth;",
     "src/bin/bench_render.rs:1 use renderer::camera::render_benchmark;",
     "src/bin/bench_external_render.rs:1 use renderer::camera::render_external_benchmark;",
     "src/obj.rs:176 use crate::obj::{build_bvh, parse_obj};"
    ],
    "operador do projeto": [
     "src/bin/librarian_measure_mesh.rs:52 Matrix::translation(Tuple::vector(-1.5, 0., 0.)) * Matrix::rotation_y(PI / 6.) *",
     "src/bin/librarian_measure_mesh.rs:52 Matrix::translation(Tuple::vector(-1.5, 0., 0.)) * Matrix::rotation_y(PI / 6.) *",
     "src/bin/librarian_measure_mesh.rs:52 Matrix::translation(Tuple::vector(-1.5, 0., 0.)) * Matrix::rotation_y(PI / 6.) *",
     "src/bin/librarian_measure_mesh.rs:52 Matrix::translation(Tuple::vector(-1.5, 0., 0.)) * Matrix::rotation_y(PI / 6.) *",
     "src/bin/librarian_measure_mesh.rs:52 Matrix::translation(Tuple::vector(-1.5, 0., 0.)) * Matrix::rotation_y(PI / 6.) *"
    ],
    "dentro de macro": [
     "src/bin/librarian_measure_mesh.rs:72 let lights = vec![Light::point_light(Tuple::point(-10., 10., -10.), Color::new(1",
     "src/bin/librarian_measure_mesh.rs:72 let lights = vec![Light::point_light(Tuple::point(-10., 10., -10.), Color::new(1",
     "src/bin/librarian_measure_mesh.rs:72 let lights = vec![Light::point_light(Tuple::point(-10., 10., -10.), Color::new(1",
     "src/bin/librarian_measure_mesh.rs:73 let mut world = World::new(vec![Object::from(floor), Object::from(wall)], lights",
     "src/bin/librarian_measure_mesh.rs:73 let mut world = World::new(vec![Object::from(floor), Object::from(wall)], lights"
    ],
    "referência como valor": [
     "src/count.rs:132 let time = lines.iter().find(|l| l.starts_with(\"librarian-measure cpu_ms=\") && l",
     "src/count.rs:150 let time = lines.iter().find(|l| l.starts_with(\"librarian-measure cpu_ms=\") && l",
     "src/obj.rs:205 group.subgroups.iter().flat_map(leaves).collect()"
    ]
   }
  }
 },
 "fora_da_edition": 0,
 "extras": {
  "disco": {
   "iguais": 32,
   "diferentes": []
  },
  "sem_declaracao": 0,
  "scip_encoding": [
   1
  ],
  "rayon": [
   "src/camera.rs:81:ray_from_pixel [sem nome]",
   "src/world.rs:109:color_at [sem nome]",
   "src/canvas.rs:23:set_pixel_color [sem nome]",
   "src/camera.rs:81:ray_from_pixel [sem nome]",
   "src/world.rs:109:color_at [sem nome]",
   "src/canvas.rs:23:set_pixel_color [sem nome]",
   "src/camera.rs:81:ray_from_pixel [sem nome]",
   "src/world.rs:109:color_at [sem nome]",
   "src/canvas.rs:23:set_pixel_color [sem nome]",
   "src/camera.rs:81:ray_from_pixel [sem nome]",
   "src/world.rs:109:color_at [sem nome]",
   "src/canvas.rs:23:set_pixel_color [sem nome]"
  ]
 },
 "por_resolucao": {
  "unique": 325,
  "no_candidate_in_project": 449,
  "above_candidate_limit (teste)": 129,
  "type (teste)": 440,
  "unique (teste)": 703,
  "type": 183,
  "homonym": 77,
  "trait_default": 7,
  "above_candidate_limit": 44,
  "trait_default (teste)": 33,
  "no_candidate_in_project (teste)": 107,
  "receiver": 21,
  "homonym (teste)": 3,
  "scope (teste)": 24
 }
}
```
