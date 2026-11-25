// As sementes da verificação de tela (tela-24a.js): para cada caso, o símbolo que escreve a chamada e o texto
// da ocorrência, lidos do grafo de um run do binário do 2.4a.   node seeds24a.js <dump-dir>
'use strict';
const fs = require('fs'), path = require('path');
const dir = process.argv[2];
const CASES = {
  'healthchecks-25597e41': [['hc/lib/curl.py:206', 'Response'], ['hc/integrations/signal/tests/test_signal_captcha.py:66', 'setup_mock'], ['hc/api/tests/test_sendalerts.py:116', 'notify'], ['hc/api/views.py:463', '_lookup']],
  'renderer-9b78f3a9': [['src/world.rs:192', 'create_default_world']],
};
const out = {};
for (const [key, cases] of Object.entries(CASES)) {
  const g = JSON.parse(fs.readFileSync(path.join(dir, `${key}.graph.json`), 'utf8'));
  for (const [where, text] of cases) {
    const found = g.edges.filter(e => e.kind === 'observes' && `${e.evidence.path}:${e.evidence.start_line}` === where && g.nodes[e.to].label === text && g.nodes[e.to].kind === 'call_observed' && g.nodes[e.from].category === 'symbol');
    if (found.length !== 1) throw new Error(`${where} ${text}: ${found.length}`);
    out[where] = { seed: found[0].from, text, dono: g.nodes[found[0].from].label };
  }
}
fs.writeFileSync(path.join(__dirname, 'seeds.json'), JSON.stringify(out, null, 1) + '\n');
console.log(Object.values(out).map(x => x.dono).join(', '));
