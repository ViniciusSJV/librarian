// P1 real validation against a Studio on a COPY of .studio. Usage: node validate.js <port> <root> <out.json>
const fs = require('fs');
const [port, root, out] = process.argv.slice(2);
const base = `http://127.0.0.1:${port}`;
const key = 'healthchecks-25597e41', run = '20261118-061046';
const turns = `${root}/.studio/projects/${key}/runs/${run}/turns`;
const views = 'src:627a48233053768114a51eadccf97aadd78ac0fc450648e2b5da871ba305777e:d0121156422dcefcd2035b4c7f2222f23eba84765c7abf28520153d051ca673d';
const ping = `${views}:function:6860:9238`, pingBySlug = `${views}:function:9244:10677`;

async function call(method, path, body) {
  const r = await fetch(base + path, { method, headers: { 'content-type': 'application/json' }, body: body && JSON.stringify(body) });
  const text = await r.text();
  if (!r.ok) throw new Error(`${method} ${path}: ${r.status} ${text.slice(0, 300)}`);
  return JSON.parse(text);
}

(async () => {
  const result = {};
  const facts = await call('GET', `/api/facts?key=${key}&run=${run}`);
  result.facts = facts.facts.map(f => ({ id: f.id, symbol: f.symbol_name, declaration_line: f.declaration_line, status: f.status }));
  for (const [name, id] of [['ping', ping], ['ping_by_slug', pingBySlug]]) {
    const s = await call('POST', '/api/envelope/suggest', { key, run, nodes: [id] });
    const item = s.items.find(i => i.id === id);
    result[`suggest_${name}`] = { facts: item.facts, items: s.items.map(i => ({ id: i.id.split(':').slice(-3).join(':'), facts: i.facts })) };
  }
  // Q6's declarations and text, with_facts: a new question in the copy, prepared and never sent.
  const q6 = JSON.parse(fs.readFileSync(`${turns}/14/question.json`, 'utf8'));
  const asked = await call('POST', '/api/gathered-question', { key, run, question: q6.asked?.original ?? q6.question, nodes: q6.gathered.nodes, with_facts: true });
  result.q6_with_facts = { turn: asked.id, prepare_error: asked.prepare_error, reviewed_facts: asked.gathered.reviewed_facts,
    targets: asked.request?.targets, query_bytes: asked.query ? Buffer.byteLength(asked.query) : null };
  result.q6_recorded = { reviewed_facts: q6.gathered.reviewed_facts };
  // Questions 01 to 14 as they were recorded: H1 to H3 checked now, H4 made again now.
  result.evidence = {};
  for (let n = 1; n <= 14; n++) {
    const t = String(n).padStart(2, '0');
    const e = await call('GET', `/api/evidence?key=${key}&run=${run}&turn=${t}`);
    result.evidence[t] = { h1: e.checks?.h1?.status ?? e.h1?.status, h2: e.checks?.h2?.status ?? e.h2?.status, h3: e.checks?.h3?.status ?? e.h3?.status,
      h4: e.h4 ? (e.h4.status ?? e.h4.result ?? JSON.stringify(e.h4).slice(0, 120)) : null, raw_keys: Object.keys(e) };
  }
  fs.writeFileSync(out, JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ suggest_ping: result.suggest_ping.facts, suggest_ping_by_slug: result.suggest_ping_by_slug.facts,
    q6: result.q6_with_facts, evidence: Object.fromEntries(Object.entries(result.evidence).map(([t, e]) => [t, `${e.h1}/${e.h2}/${e.h3} h4=${e.h4}`])) }, null, 1));
})().catch(e => { console.error(e); process.exit(1); });
