// Acha, no dump do 2.3b, a ocorrência (path@sb-eb), o dono e a resposta de cada chamada proposta.
'use strict';
const fs = require('fs');
const dir = 'C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/1d90cf65-301d-42c6-a8af-99e9259cf401/scratchpad/dump';
const want = {
  hc: [
    ['HC-98', 'hc/integrations/slack/views.py', 105, 'result.json('],
    ['HC-99', 'hc/integrations/discord/views.py', 71, 'result.json('],
    ['HC-100', 'hc/front/views.py', 533, 'project.num_checks_available('],
    ['HC-101', 'hc/front/views.py', 1072, 'target_project.num_checks_available('],
    ['HC-102', 'hc/front/views.py', 425, 'request.profile.projects('],
    ['HC-103', 'hc/integrations/telegram/views.py', 131, 'request.profile.projects('],
    ['HC-104', 'hc/api/views.py', 465, 'request.project.num_checks_available('],
    ['HC-105', 'hc/front/views.py', 640, 'check.going_down_after('],
    ['HC-106', 'hc/integrations/googlechat/transport.py', 44, 'ping.formatted_kind_created('],
    ['HC-107', 'hc/accounts/views.py', 181, 'profile.send_instant_login_link('],
    ['HC-108', 'hc/api/management/commands/smtpd.py', 91, 'check.ping('],
    ['HC-109', 'hc/accounts/backends.py', 38, 'profile.check_token('],
    ['HC-110', 'hc/accounts/views.py', 512, 'tr.can_accept('],
    ['HC-111', 'hc/integrations/slack/views.py', 106, 'doc.get("ok")'],
    ['HC-112', 'hc/integrations/discord/views.py', 72, 'doc.get("code")'],
  ],
  rt: [
    ['RT-66', 'src/bounds.rs', 52, 'result.add_point('],
    ['RT-67', 'src/world.rs', 282, 'w.color_at('],
  ],
};
const out = {};
for (const [lang, key] of [['hc', 'healthchecks-25597e41'], ['rt', 'renderer-9b78f3a9']]) {
  const d = JSON.parse(fs.readFileSync(`${dir}/${key}.calls.json`, 'utf8'));
  const all = [];
  for (const r of Object.values(d.res)) {
    for (const l of r.links) all.push({ ...l, owner: r.owner.label, res: l.resolution, to: `${l.to.label} (${l.to.at})` });
    for (const u of r.unresolved) all.push({ ...u, owner: r.owner.label, res: u.reason, to: null });
  }
  for (const [id, p, line, text] of want[lang]) {
    const hits = all.filter(x => x.path === p && x.line === line && x.text.replace(/\s+/g, '').startsWith(text.replace(/\s+/g, '')));
    const occ = [...new Set(hits.map(h => `${h.path}@${h.sb}-${h.eb}`))];
    if (occ.length !== 1) { console.log(id, 'ACHADAS', occ.length, hits.map(h => h.text)); continue; }
    const h = hits[0];
    out[id] = { ocorrencia: occ[0], local: `${p}:${line}`, texto: h.text, dono: h.owner, linha_de_base: { resolution: h.res, candidates: h.candidates, alvos: hits.map(x => x.to).filter(Boolean).sort() } };
  }
}
fs.writeFileSync(process.argv[2], JSON.stringify(out, null, 1));
for (const [id, o] of Object.entries(out)) console.log(id, o.ocorrencia, o.dono, '|', o.linha_de_base.resolution, o.linha_de_base.candidates, o.linha_de_base.alvos.join(';'));
