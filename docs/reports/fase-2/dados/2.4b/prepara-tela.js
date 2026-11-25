// Deriva tela-24b.js de tela-24a.js e escreve seeds.json.   node prepara-tela.js <dump-dir>
'use strict';
const fs = require('fs'), path = require('path');
const g = JSON.parse(fs.readFileSync(path.join(process.argv[2], 'healthchecks-25597e41.graph.json'), 'utf8'));
const CASES = [['hc/api/models.py:1173', /^self\.transport\.notify/], ['hc/api/models.py:1153', /^self\.transport\.is_noop/], ['hc/integrations/telegram/transport.py:65', /^cls\.post/], ['hc/integrations/telegram/transport.py:15', /^super\(\)\.__init__/], ['hc/integrations/mattermost/transport.py:15', /^self\.payload/]];
const out = {};
for (const [where, re] of CASES) {
  const f = g.edges.filter(e => e.kind === 'observes' && `${e.evidence.path}:${e.evidence.start_line}` === where && re.test(g.nodes[e.to].label) && g.nodes[e.to].kind === 'method_call_observed');
  if (f.length !== 1) throw new Error(where + ' ' + f.length);
  out[where] = { seed: f[0].from, text: g.nodes[f[0].to].label, dono: g.nodes[f[0].from].label };
}
fs.writeFileSync(path.join(__dirname, 'seeds.json'), JSON.stringify(out, null, 1) + '\n');
let s = fs.readFileSync(path.join(__dirname, '../2.4a/tela-24a.js'), 'utf8').replace(/\r\n/g, '\n');
const cut = (a, b, text) => { const i = s.indexOf(a), j = s.indexOf(b, i); if (i < 0 || j < 0) throw new Error(a); s = s.slice(0, i) + text + s.slice(j); };
cut('// DEC-30 check', 'const path =', `// DEC-30 check of step 2.4b (from tela-24a.js): inherited, cls, super() and @property links (\`type\`), and the 30 overrides of
// an abstract method as one box whose detail has the whole list and the choice.   node tela-24b.js
`);
s = s.replace(/tela-24a/g, 'tela-24b');
cut('const CASES = [', '\n(async', `const CASES = [
  { name: 'is_noop (@property)', root: HC, where: 'hc/api/models.py:1153', res: 'type', to: /^Transport\\.is_noop$/, basis: 'type', relation: 'calls', edgeNull: true, rule: 'propriedade' },
  { name: 'cls.post (herdado)', root: HC, where: 'hc/integrations/telegram/transport.py:65', res: 'type', to: /^HttpTransport\\.post$/, basis: 'type', relation: 'calls', rule: 'cls' },
  { name: 'super().__init__', root: HC, where: 'hc/integrations/telegram/transport.py:15', res: 'type', to: /^TransportError\\.__init__$/, basis: 'type', relation: 'calls', edgeNull: true, rule: 'super' },
  { name: 'self.payload (herdado)', root: HC, where: 'hc/integrations/mattermost/transport.py:15', res: 'type', to: /^Slackalike\\.payload$/, basis: 'type', relation: 'calls', rule: 'self' },
];
`);
s = s.replace("ok(`${tag}: relation", "ok(`${tag}: type_rule ${c.rule}`, r.link?.type_rule === c.rule, r.link?.type_rule);\n        ok(`${tag}: relation");
cut("      const step = await said(p, 'inventory.step.scope')", '      ok(`${lang.toUpperCase()}: nenhum erro de página`', `      { // the abstract method: one box, not 30
        const sd = SEEDS['hc/api/models.py:1173'], tag = lang.toUpperCase() + ' F23';
        const r = await edge(p, sd.seed, sd.text, 'hc/api/models.py:1173');
        const d = await p.evaluate(() => ({ boxes: [...document.querySelectorAll('#pathCanvas g.node')].map(n => n.dataset.id), texts: [...document.querySelectorAll('#pathCanvas g.node text')].map(x => x.textContent), items: [...document.querySelectorAll('#pathDetail .many-list li')].map(li => li.innerText), dash: document.querySelectorAll('#pathCanvas polyline.ambiguous').length }));
        ok(tag + ': uma caixa só para as 30', d.boxes.filter(b => b.startsWith('many:')).length === 1 && !d.boxes.some(b => b.includes('transport') && /notify/.test(b) && !b.startsWith('many:')), d.boxes.length);
        ok(tag + ': a caixa diz 30', d.texts.some(x => /^30 (candidatas|candidates)$/.test(x)), d.texts.join(' | '));
        ok(tag + ': uma linha tracejada', d.dash === 1, d.dash);
        ok(tag + ': o detalhe lista as 30, com arquivo e botão', d.items.length === 30 && d.items.every(x => /\\.notify .*transport\\.py:\\d+/.test(x.replace(/\\n/g, ' '))), d.items.slice(0, 3).join(' || '));
        ok(tag + ': o texto da resolução', r.detail?.includes(await said(p, 'path.resolution.overrides', 30)), r.detail?.slice(0, 300));
        ok(tag + ': nenhuma chave crua', !/path\\.|undefined|\\{n\\}/.test(r.detail ?? ''), r.detail?.slice(0, 200));
        await p.screenshot({ path: path.join(OUT, lang + '-f23.png') });
        // The box itself, clicked, opens the same list.
        await p.evaluate(() => { document.querySelector('#pathDetail').hidden = true; });
        const box = p.locator('#pathCanvas g.node[data-id^="many:"]');
        await box.click();
        ok(tag + ': clicar na caixa abre a lista', await p.evaluate(() => !document.querySelector('#pathDetail').hidden && document.querySelectorAll('#pathDetail .many-list li').length === 30));
        // Choosing one follows it.
        const ans = p.waitForResponse(x => new URL(x.url()).pathname === '/api/calls');
        await p.locator('#pathDetail .many-list li', { hasText: 'Slackalike.notify' }).locator('button').click();
        const body = await (await ans).json();
        await p.waitForTimeout(900);
        const ch = body.links.filter(l => l.resolution === 'chosen');
        ok(tag + ': a escolha vira um link chosen para Slackalike.notify', ch.length === 1 && (await labelOf(p, ch[0].to)) === 'Slackalike.notify', JSON.stringify(ch.map(x => x.to)));
        ok(tag + ': a caixa some e a linha fica cheia', await p.evaluate(() => !document.querySelector('#pathCanvas g.node[data-id^="many:"]') && !document.querySelectorAll('#pathCanvas polyline.ambiguous').length));
        await p.click('#pathResetChoices');
        await p.waitForTimeout(900);
        ok(tag + ': limpar as escolhas volta à caixa', await p.evaluate(() => !!document.querySelector('#pathCanvas g.node[data-id^="many:"]')));
      }
`);
s = s.replace(/const c = CASES\[0\];\n\s+const r = await edge\(p, SEEDS\[c\.where\]\.seed, SEEDS\[c\.where\]\.text, c\.where\);/, "const w = 'hc/api/models.py:1173';\n      const r = await edge(p, SEEDS[w].seed, SEEDS[w].text, w);");
s = s.replace("pt-escopo-", "pt-f23-").replace("o detalhe aparece com o texto do escopo`, r.detail?.includes('seguida pelo escopo')", "o detalhe aparece com o texto do abstrato`, r.detail?.includes('o método do tipo é abstrato')");
// A dashed line answers only on its dashes: a second point, a little ahead, when the first is in a gap.
s = s.replace('  await p.mouse.click(at.x, at.y);', "  for (const dx of [0, 2, 4, 6]) { await p.mouse.click(at.x + dx, at.y); if (await p.evaluate(() => !document.querySelector('#pathDetail').hidden)) break; }");
fs.writeFileSync(path.join(__dirname, 'tela-24b.js'), s);
console.log(Object.values(out).map(x => x.dono).join(', '));
