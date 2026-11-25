// Passo 2.4, Parada 1 (diagnóstico). Cruza as mudanças previstas (py.json, rs.json, locais.json)
// com a amostra do portão (fixtures), pela ocorrência arquivo@início-fim.
'use strict';
const fs = require('fs');
const { load } = require('./load.js');
const FIX = 'C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/fixtures';
const py = require('./py.json'), rs = require('./rs.json'), locais = require('./locais.json');
const dir = process.argv[2] ?? '../dump';
for (const [key, file, rows] of [
  ['healthchecks-25597e41', 'amostra-hc.json', [...py.n0.map(x => ({ ...x, frente: 'N0' })), ...py.n5.map(x => ({ ...x, frente: 'N5' })), ...locais.map(x => ({ ...x, frente: 'N0-local', mudanca: x.now !== 'no_candidate_in_project' ? 'link->sem (ligação local)' : 'igual' }))]],
  ['renderer-9b78f3a9', 'amostra-rt.json', rs.map(x => ({ ...x, frente: 'N0' }))],
]) {
  const P = load(dir, key);
  const at = new Map();
  for (const c of P.calls.values()) at.set(`${c.path}@${c.sb}-${c.eb}`, c);
  const byLine = new Map();
  for (const r of rows) (byLine.get(`${r.at}|${r.text.slice(0, 40)}`) ?? byLine.set(`${r.at}|${r.text.slice(0, 40)}`, []).get(`${r.at}|${r.text.slice(0, 40)}`)).push(r);
  const sample = JSON.parse(fs.readFileSync(`${FIX}/${file}`, 'utf8')).chamadas;
  console.log(`== ${file}: ${sample.length} chamadas`);
  for (const s of sample) {
    const c = at.get(s.ocorrencia);
    if (!c) { console.log(s.id, 'ocorrência não achada'); continue; }
    const hits = byLine.get(`${c.path}:${c.line}|${c.text.slice(0, 40)}`) ?? [];
    for (const h of hits) if (!/^igual|confirma/.test(h.mudanca)) console.log(s.id, s.local, s.texto, '|', h.frente, h.mudanca, '|', h.after ?? h.binding?.to, '| verdade:', JSON.stringify(s.verdade));
    for (const h of hits) if (/confirma/.test(h.mudanca)) console.log(s.id, 'confirma', h.frente);
  }
}
