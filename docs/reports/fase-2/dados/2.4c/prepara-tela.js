// Deriva tela-24c.js de ../2.4b/tela-24b.js e escreve seeds.json.   node prepara-tela.js <dump-dir>
'use strict';
const fs = require('fs'), path = require('path');
const g = JSON.parse(fs.readFileSync(path.join(process.argv[2], 'healthchecks-25597e41.graph.json'), 'utf8'));
const CASES = [['hc/api/models.py:1173', /^self\.transport\.notify/], ['hc/api/models.py:1153', /^self\.transport\.is_noop/], ['hc/api/transports.py:162', /^cls\.raise_for_response/], ['hc/integrations/slack/transport.py:72', /^self\.fix_asterisks/], ['hc/integrations/call/views.py:26', /^form\.get_json/], ['hc/integrations/telegram/transport.py:65', /^cls\.post/], ['hc/integrations/mattermost/transport.py:15', /^self\.payload/]];
const out = {};
for (const [where, re] of CASES) {
  const f = g.edges.filter(e => e.kind === 'observes' && `${e.evidence.path}:${e.evidence.start_line}` === where && re.test(g.nodes[e.to].label) && g.nodes[e.to].kind === 'method_call_observed');
  if (f.length !== 1) throw new Error(where + ' ' + f.length);
  out[where] = { seed: f[0].from, text: g.nodes[f[0].to].label, dono: g.nodes[f[0].from].label };
}
fs.writeFileSync(path.join(__dirname, 'seeds.json'), JSON.stringify(out, null, 1) + '\n');
let s = fs.readFileSync(path.join(__dirname, '../2.4b/tela-24b.js'), 'utf8').replace(/\r\n/g, '\n');
const cut = (a, b, text) => { const i = s.indexOf(a), j = s.indexOf(b, i); if (i < 0 || j < 0) throw new Error(a); s = s.slice(0, i) + text + s.slice(j); };
cut('// DEC-30 check', 'const path =', `// DEC-30 check of step 2.4c (from tela-24b.js): the bodies that may run (2 lines, or one box above 8), the choice, the
// text that never says "abstract", and links that stay.   node tela-24c.js
`);
s = s.replace(/tela-24b/g, 'tela-24c');
cut('const CASES = [', '\n(async', `const CASES = [
  { name: 'form.get_json (construtor)', root: HC, where: 'hc/integrations/call/views.py:26', res: 'type', to: /^PhoneNumberForm\\.get_json$/, basis: 'type', relation: 'calls', rule: 'construtor' },
  { name: 'cls.post (herdado)', root: HC, where: 'hc/integrations/telegram/transport.py:65', res: 'type', to: /^HttpTransport\\.post$/, basis: 'type', relation: 'calls', rule: 'cls' },
  { name: 'self.payload (herdado)', root: HC, where: 'hc/integrations/mattermost/transport.py:15', res: 'type', to: /^Slackalike\\.payload$/, basis: 'type', relation: 'calls', rule: 'self' },
];
// One box each: the count, the body to choose, and whether the static type's own is in the list.
const MANY = [
  { name: 'notify (abstrato)', where: 'hc/api/models.py:1173', n: 30, rule: 'propriedade', pick: 'Slackalike.notify', has: 'Webhook.notify', hasNot: 'Transport.notify' },
  { name: 'raise_for_response', where: 'hc/api/transports.py:162', n: 12, rule: 'cls', pick: 'Discord.raise_for_response', has: 'HttpTransport.raise_for_response' },
  { name: 'is_noop', where: 'hc/api/models.py:1153', n: 14, rule: 'propriedade', pick: 'Webhook.is_noop', has: 'Transport.is_noop' },
];
const NO_ABSTRACT = x => !/abstrat|abstract/i.test(x ?? '');
`);
cut('      { // the abstract method: one box, not 30', '      ok(`${lang.toUpperCase()}: nenhum erro de página`', `      for (const m of MANY) { // one box
        const sd = SEEDS[m.where], tag = lang.toUpperCase() + ' ' + m.name;
        const r = await edge(p, sd.seed, sd.text, m.where);
        if (r.error) { ok(tag, false, r.error); continue; }
        const word = new RegExp('^' + m.n + ' (candidatas|candidates)$');
        const d = await p.evaluate(() => ({ boxes: [...document.querySelectorAll('#pathCanvas g.node[data-id^="many:"]')].map(n => n.textContent), items: [...document.querySelectorAll('#pathDetail .many-list li')].map(li => li.innerText.replace(/\\n/g, ' ')) }));
        const label = x => x.split(' ')[0];
        ok(tag + ': uma caixa com a contagem', d.boxes.filter(b => b.includes(m.n + ' ' + (lang === 'pt' ? 'candidatas' : 'candidates'))).length === 1, d.boxes.join(' | '));
        ok(tag + ': linha tracejada', r.dashed, r.dashed);
        ok(tag + ': o detalhe lista todas, com arquivo', d.items.length === m.n && d.items.every(x => /transports?\\.py:\\d+/.test(x)), d.items.slice(0, 3).join(' || '));
        ok(tag + ': traz ' + m.has, d.items.some(x => label(x) === m.has), d.items.map(label).join());
        if (m.hasNot) ok(tag + ': não traz ' + m.hasNot, !d.items.some(x => label(x) === m.hasNot));
        ok(tag + ': homonym, basis type, type_rule ' + m.rule + ', candidates ' + m.n, r.link?.resolution === 'homonym' && r.link.basis === 'type' && r.link.type_rule === m.rule && r.link.candidates === m.n && r.link.candidate_edge === null, JSON.stringify(r.link).slice(0, 300));
        const text = m.hasNot ? await p.evaluate(([n, method]) => __t.t('path.resolution.overridesAbstract', { n, method }), [m.n, m.hasNot]) : await said(p, 'path.resolution.overrides', m.n);
        ok(tag + (m.hasNot ? ': o texto diz que ' + m.hasNot + ' é abstrato, não roda e saiu da lista' : ': o texto da resolução, sem dizer abstrato'), r.detail?.includes(text) && (m.hasNot ? r.detail.includes(m.hasNot) && /saiu da lista|not in the list/.test(r.detail) : NO_ABSTRACT(r.detail)), r.detail?.slice(0, 400));
        ok(tag + ': abstract_method ' + (m.hasNot ?? 'ausente'), m.hasNot ? (await labelOf(p, r.link?.abstract_method)) === m.hasNot : r.link && !('abstract_method' in r.link), r.link?.abstract_method);
        ok(tag + ': nenhuma chave crua', !/path\\.|undefined|\\{n\\}/.test(r.detail ?? ''), r.detail?.slice(0, 200));
        await p.screenshot({ path: path.join(OUT, lang + '-' + m.n + '.png') });
        await p.evaluate(() => { document.querySelector('#pathDetail').hidden = true; });
        await p.locator('#pathCanvas g.node[data-id^="many:"]', { hasText: word }).click().catch(async () => p.locator('#pathCanvas g.node[data-id^="many:"]', { hasText: m.n + ' ' }).first().click());
        ok(tag + ': clicar na caixa abre a lista', await p.evaluate(n => !document.querySelector('#pathDetail').hidden && document.querySelectorAll('#pathDetail .many-list li').length === n, m.n));
        const ans = p.waitForResponse(x => new URL(x.url()).pathname === '/api/calls');
        await p.locator('#pathDetail .many-list li').filter({ has: p.locator('code', { hasText: new RegExp('^' + m.pick.replace('.', '\\\\.') + '$') }) }).locator('button').click();
        const body = await (await ans).json();
        await p.waitForTimeout(900);
        const ch = body.links.filter(l => l.resolution === 'chosen');
        ok(tag + ': a escolha vira um link chosen para ' + m.pick, ch.length === 1 && (await labelOf(p, ch[0].to)) === m.pick, JSON.stringify(ch.map(x => x.to)));
        await p.click('#pathResetChoices');
        await p.waitForTimeout(900);
        ok(tag + ': limpar as escolhas volta à caixa', await p.evaluate(n => [...document.querySelectorAll('#pathCanvas g.node[data-id^="many:"]')].some(x => x.textContent.includes(n + ' ')), m.n));
      }
      { // two bodies: two dashed lines, each with the choice
        const w = 'hc/integrations/slack/transport.py:72', sd = SEEDS[w], tag = lang.toUpperCase() + ' fix_asterisks';
        const r = await edge(p, sd.seed, sd.text, w);
        const lines = await p.evaluate(w => [...document.querySelectorAll('#pathCanvas polyline.edge')].filter(x => x.querySelector('title')?.textContent.startsWith('self.fix_asterisks') && x.querySelector('title').textContent.includes(w)).map(x => x.classList.contains('ambiguous')), w);
        ok(tag + ': duas linhas tracejadas', lines.length === 2 && lines.every(Boolean), JSON.stringify(lines));
        ok(tag + ': homonym, basis type, self, 2, com aresta', r.link?.resolution === 'homonym' && r.link.basis === 'type' && r.link.type_rule === 'self' && r.link.candidates === 2 && typeof r.link.candidate_edge === 'string', JSON.stringify(r.link).slice(0, 300));
        ok(tag + ': o texto da resolução, sem dizer abstrato', r.detail?.includes(await said(p, 'path.resolution.overrides', 2)) && NO_ABSTRACT(r.detail) && !('abstract_method' in r.link), r.detail);
        await p.screenshot({ path: path.join(OUT, lang + '-fix.png') });
        const ans = p.waitForResponse(x => new URL(x.url()).pathname === '/api/calls');
        await p.locator('#pathDetail button', { hasText: await said(p, 'path.choose') }).first().click();
        const ch = (await (await ans).json()).links.filter(l => l.resolution === 'chosen');
        ok(tag + ': a escolha vira um link chosen', ch.length === 1 && /fix_asterisks$/.test(await labelOf(p, ch[0].to)), JSON.stringify(ch.map(x => x.to)));
        await p.waitForTimeout(900);
        await p.click('#pathResetChoices');
        await p.waitForTimeout(600);
      }
      ok(lang.toUpperCase() + ': só o texto do abstrato diz "abstrato"', await p.evaluate(() => JSON.stringify(Object.entries(__t.STRINGS).filter(([k, v]) => k.startsWith('path.') && /abstrat|abstract/i.test(JSON.stringify(v))).map(([k]) => k)) === '["path.resolution.overridesAbstract"]'));
`);
s = s.replace("line.classList.contains('dashed')", "line.classList.contains('ambiguous') || line.classList.contains('dashed')");
s = s.replace("for (const v of [{ name: 'tema escuro', width: 1440, theme: 'dark' }, { name: 'tema claro', width: 1440, theme: 'light' }, { name: 'tela estreita (390 px)', width: 390, height: 844, theme: null }]) {", "for (const v of [{ name: 'tema escuro', width: 1440, theme: 'dark' }, { name: 'tema claro', width: 1440, theme: 'light' }, { name: 'tela estreita (390 px)', width: 390, height: 844, theme: null }].flatMap(v => [{ ...v, w: 'hc/api/models.py:1153' }, { ...v, name: v.name + ', abstrato', w: 'hc/api/models.py:1173', abstract: true }])) {");
if (!s.includes("w: 'hc/api/models.py:1153' }, { ...v")) throw new Error('temas');
s = s.split("const w = 'hc/api/models.py:1173';\n").join("const w = v.w;\n");
s = s.replace('pt-f23-${', 'pt-${v.abstract ? "abstrato" : "noop"}-${').replace("o detalhe aparece com o texto do abstrato`, r.detail?.includes('o método do tipo é abstrato')", "o detalhe aparece com o texto certo`, r.detail?.includes('roda o corpo da classe do objeto') && (v.abstract ? /Transport\\.notify, do tipo escrito, é abstrato/.test(r.detail) : NO_ABSTRACT(r.detail))");
fs.writeFileSync(path.join(__dirname, 'tela-24c.js'), s);
console.log(Object.values(out).map(x => x.dono).join(', '));
