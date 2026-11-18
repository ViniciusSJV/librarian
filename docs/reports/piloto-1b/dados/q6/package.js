// Q6: the prepared package against the original, before sending.
const fs = require('fs');
const O = 'C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/q6';
const P = O + '/root/.studio/projects/healthchecks-25597e41/runs';
const dirs = { original: P + '/20261118-061046/turns/14', nova: P + '/20261118-205150/turns/01' };
const facts = o => { const out = []; const walk = (x, path) => { if (Array.isArray(x)) x.forEach((y, i) => walk(y, `${path}[${i}]`)); else if (x && typeof x === 'object') { if (/^F\d+$/.test(x.id ?? '') && (x.statement || x.text)) out.push(`${x.id} @ ${path}`); for (const [k, v] of Object.entries(x)) walk(v, `${path}.${k}`); } }; walk(o, ''); return out; };
(async () => {
  for (const [name, d] of Object.entries(dirs)) {
    const q = JSON.parse(fs.readFileSync(d + '/question.json'));
    const cq = fs.readFileSync(d + '/claims/query.json');
    console.log(`== ${name}: gathered.reviewed_facts ${JSON.stringify(q.gathered.reviewed_facts)}`);
    console.log(`   claims/query.json ${cq.length} B; prepared/bundle/query.json ${fs.statSync(d + '/prepared/bundle/query.json').size} B; estimativa a 3,96 B/token (razão da original): ${Math.round(cq.length / (46220 / 11667))}`);
    const f = facts(JSON.parse(cq));
    console.log(`   fatos na query enviada: ${f.length} -> ${f.map(x => x.split(' @ ')[0]).join(' ')}`);
    console.log(`   onde: ${[...new Set(f.map(x => x.split(' @ ')[1].replace(/\[\d+\]/g, '[]')))].join(', ')}`);
  }
  const ev = await (await fetch('http://127.0.0.1:8790/api/evidence?key=healthchecks-25597e41&run=20261118-205150&turn=01')).json();
  fs.writeFileSync(O + '/evidence-before-send.json', JSON.stringify(ev, null, 1));
  const brief = JSON.stringify(ev, (k, v) => (typeof v === 'string' && v.length > 120 ? v.slice(0, 120) + '…' : v));
  console.log('\n/api/evidence:', brief.slice(0, 2500));
})();
