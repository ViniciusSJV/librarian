// Deriva tela-24d0.js de ../2.4c/tela-24c.js e escreve seeds.json.   node prepara-tela.js <dump-dir>
'use strict';
const fs = require('fs'), path = require('path');
const g = JSON.parse(fs.readFileSync(path.join(process.argv[2], 'healthchecks-25597e41.graph.json'), 'utf8'));
const out = JSON.parse(fs.readFileSync(path.join(__dirname, '../2.4c/seeds.json'), 'utf8'));
// Dois casos seguidos não podem ter o mesmo dono: a página não refaz a consulta da mesma semente.
const NEW = [
  ['hc/api/models.py:1479', /^TokenBucket\.authorize\(/, 'TokenBucket.authorize (função solta homônima)', 'TokenBucket.authorize'],
  ['hc/api/models.py:1293', /^PdConf\.load/, 'PdConf.load (mesmo módulo)', 'PdConf.load'],
  ['hc/integrations/telegram/views.py:57', /^TelegramCallback\.load/, 'TelegramCallback.load', 'TelegramCallback.load'],
  ['hc/integrations/signal/views.py:119', /^Signal\.send/, 'Signal.send (importada)', 'Signal.send'],
  ['hc/integrations/telegram/views.py:82', /^Telegram\.send/, 'Telegram.send (HC-156)', 'Telegram.send'],
];
const cases = NEW.map(([where, re, name, to]) => {
  const f = g.edges.filter(e => e.kind === 'observes' && `${e.evidence.path}:${e.evidence.start_line}` === where && re.test(g.nodes[e.to].label) && g.nodes[e.to].kind === 'method_call_observed');
  if (f.length !== 1) throw new Error(where + ' ' + f.length);
  out[where] = { seed: f[0].from, text: g.nodes[f[0].to].label, dono: g.nodes[f[0].from].label };
  return `  { name: ${JSON.stringify(name)}, root: HC, where: '${where}', res: 'type', to: /^${to.replace('.', '\\.')}$/, basis: 'type', relation: 'calls', rule: 'classe' },\n`;
});
fs.writeFileSync(path.join(__dirname, 'seeds.json'), JSON.stringify(out, null, 1) + '\n');
let s = fs.readFileSync(path.join(__dirname, '../2.4c/tela-24c.js'), 'utf8').replace(/\r\n/g, '\n').replace(/tela-24c/g, 'tela-24d0');
const rep = (a, b) => { if (!s.includes(a)) throw new Error(a); s = s.replace(a, b); };
rep('// DEC-30 check of step 2.4c (from tela-24d0.js)', '// DEC-30 check of step 2.4d-0 (from ../2.4c/tela-24c.js): the calls by the class\'s name, and what the 2.4c checked');
rep('const CASES = [\n', 'const CASES = [\n' + cases.join(''));
rep("      ok(lang.toUpperCase() + ': só o texto do abstrato", "      ok(lang.toUpperCase() + ': o texto de type diz o nome da classe', /Classe\\.m\\(\\)|Class\\.m\\(\\)/.test(await said(p, 'path.resolution.type')));\n      ok(lang.toUpperCase() + ': só o texto do abstrato");
rep("{ ...v, name: v.name + ', abstrato', w: 'hc/api/models.py:1173', abstract: true }]", "{ ...v, name: v.name + ', abstrato', w: 'hc/api/models.py:1173', abstract: true }, { ...v, name: v.name + ', classe', w: 'hc/api/models.py:1479', classe: true }]");
rep('v.abstract ? "abstrato" : "noop"', 'v.classe ? "classe" : v.abstract ? "abstrato" : "noop"');
rep("r.detail?.includes('roda o corpo da classe do objeto') && (v.abstract", "v.classe ? /seguida: método do tipo escrito.*Classe\\.m\\(\\)/.test(r.detail ?? '') : r.detail?.includes('roda o corpo da classe do objeto') && (v.abstract");
fs.writeFileSync(path.join(__dirname, 'tela-24d0.js'), s);
