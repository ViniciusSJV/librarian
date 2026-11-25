// Passo 2.4c, Parada 1: as contagens (incluído pelo py24c.js).
'use strict';
const fs = require('fs');
module.exports = (R4C, rows, check, outFile, X) => {
  const { P, N, info, show, parts, slice, named, declared, classes, descendants, lookup, last, sym, parent } = X;
  const A = R4C.all, cnt = (xs, f) => { const m = {}; xs.forEach(x => { const k = f(x); m[k] = (m[k] ?? 0) + 1; }); return Object.fromEntries(Object.entries(m).sort()); };
  const PT = t => t ? 'T' : 'P';
  console.log('conferência com o binário:', check.igual, 'iguais', check.dif.length, 'diferentes', check.semTipo.length, 'sem tipo');
  console.log('chamadas com tipo estático do projeto:', A.length, '; com método achado:', A.filter(x => x.alvo).length);
  for (const t of [false, true]) console.log(t ? 'teste' : 'produção', cnt(A.filter(x => x.alvo && x.c.test === t), x => `${x.t.rule} | subclasses ${x.ndesc ? 'sim' : 'não'} | sobrescrita ${x.extra.length ? 'sim' : 'não'} | aberta ${x.abertas.length ? 'sim' : 'não'}`));
  console.log('== com sobrescrita (qualquer origem, inclusive as exceções)');
  for (const x of A.filter(x => x.extra?.length)) console.log(PT(x.c.test), x.at, x.text.replace(/\s+/g, ' ').slice(0, 60), '|', x.t.rule, N[x.t.cls].label, '| hoje', x.c.res, x.c.targets.length, '| exato', x.exact, 'super', x.super_, '| extra', x.extra.length, '| cand', x.cand?.length, '| sobrescrevem quem chama', JSON.stringify(x.menos), '| baseAbs', x.baseAbs, '\n     ', x.extra.join('; '));
  console.log('== método achado e subclasse aberta (base de fora antes, nome ligado a atributo...)');
  for (const x of A.filter(x => x.alvo && x.abertas.length)) console.log(PT(x.c.test), x.at, x.text.slice(0, 50), x.t.rule, x.c.res, x.abertas.slice(0, 2), x.abertas.length);
  console.log('== vários corpos na própria classe', A.filter(x => x.multi).length);
  const porNome = [];
  for (const c of P.calls.values()) {
    if (c.lang !== 'python' || !c.path) continue;
    const text = slice(c.path, c.sb, c.eb), p = parts(text) ?? parts(text + '()');
    const recv = p?.receiver ?? null;
    if (!recv || !/^[A-Za-z_][\w.]*$/.test(recv) || /^(self|cls)\b/.test(recv)) continue;
    const n = named(c.path, recv);
    if (n.kind !== 'project' || !info[n.id]) continue;
    const l = lookup(n.id, p.name);
    const ov = l.found ? descendants(n.id).filter(d => info[d].methods[p.name]).length : 0;
    porNome.push({ at: c.path + ':' + c.line, test: c.test, text: text.slice(0, 60), res: c.res, alvos: c.targets.map(show), l: l.found ? 'achado' : l.external ? 'fora' : JSON.stringify(l), ov });
  }
  console.log('== T.m() pelo nome da classe', porNome.length, cnt(porNome, x => `${x.test ? 'teste' : 'prod'} ${x.res} ${x.l} sobrescritas ${x.ov ? 'sim' : 'não'}`));
  porNome.filter(x => x.l === 'achado').slice(0, 40).forEach(x => console.log('  ', x.at, x.text, x.res, x.alvos.join(), 'sobrescritas', x.ov));
  const ov = [];
  for (const c of classes) for (const [n, ms] of Object.entries(info[c].methods)) { const ds = descendants(c).filter(d => info[d].methods[n]); if (ds.length) ov.push({ m: show(ms[0]), n, ds: ds.length, sub: ds.slice(0, 3).map(d => N[d].label) }); }
  console.log('== métodos sobrescritos no projeto', ov.length); ov.forEach(x => console.log('  ', x.m, x.ds, x.sub.join()));
  const nomes = new Set(ov.map(x => x.n)), vistos = new Set(A.map(x => x.c.occ));
  console.log('== x.m() com m sobrescrito em algum lugar e sem tipo estático', cnt([...P.calls.values()].filter(c => c.lang === 'python' && c.kind === 'method_call_observed' && !vistos.has(c.occ) && nomes.has(parts(c.text)?.name ?? '')), c => `${c.test ? 'teste' : 'prod'} ${parts(c.text).name} ${c.res}`));
  const ms = new Set(ov.map(x => x.m));
  console.log('== links únicos para um método sobrescrito');
  for (const c of P.calls.values()) if (c.lang === 'python' && c.targets.length === 1 && ms.has(show(c.targets[0])) && c.res !== 'homonym') console.log('  ', PT(c.test), c.path + ':' + c.line, c.text.replace(/\s+/g, ' ').slice(0, 60), c.res, c.rule, show(c.targets[0]));
  const mn = {};
  for (const id in N) if (sym(id) && N[id].kind === 'method' && N[id].language === 'python') mn[last(N[id].label)] = (mn[last(N[id].label)] ?? 0) + 1;
  const need = A.filter(x => x.alvo && !x.exact && !x.super_), dois = need.filter(x => mn[x.p.name] > 1), hoje = A.filter(x => x.super_ || x.baseAbs || !x.alvo);
  const sy = xs => new Set(xs.map(x => x.c.owner)).size;
  console.log('== custo: não exatas com método achado', need.length, 'chamadas em', sy(need), 'símbolos; com o nome em mais de um método', dois.length, 'em', sy(dois), 'símbolos; (super, abstrato ou não achado: já precisam hoje, no máximo)', hoje.length, 'em', sy(hoje));
  const outra = x => classes.some(k => k !== x.t.cls && info[k].bases.length && (info[k].methods[x.p.name] || info[k].other.has(x.p.name)));
  const multiBase = classes.filter(k => info[k].bases.length > 1).length;
  const pagam = new Set(hoje.map(x => x.c.owner)), so = xs => new Set(xs.map(x => x.c.owner).filter(o => !pagam.has(o))).size, f2 = need.filter(outra);
  console.log('== custo com filtros: novos símbolos que pagariam, sem filtro', so(need), '; nome em mais de um método', so(dois), '; nome declarado em outra classe com base', so(f2), '(classes com mais de uma base:', multiBase + ')');
  console.log('== subclasses existem, sem sobrescrita');
  for (const x of A.filter(x => x.alvo && x.ndesc && !x.extra.length)) console.log('  ', PT(x.c.test), x.at, N[x.c.owner].label, x.text.replace(/\s+/g, ' ').slice(0, 60), x.t.rule, N[x.t.cls].label, x.ndesc, x.c.res, x.alvo[0], `${x.c.path}@${x.c.sb}-${x.c.eb}`);
  console.log('== nome também declarado em outra classe, sem sobrescrita (irmãs)');
  for (const x of A.filter(x => x.alvo && !x.extra.length && !x.super_ && !x.exact && /fix_asterisks|get_json|is_noop|raise_for_response|payload|send$/.test(x.p.name))) console.log('  ', PT(x.c.test), x.at, N[x.c.owner].label, x.text.replace(/\s+/g, ' ').slice(0, 50), x.t.rule, N[x.t.cls].label, x.c.res, x.alvo[0], `${x.c.path}@${x.c.sb}-${x.c.eb}`);
  console.log('== construtor exato com homônimos');
  for (const x of A.filter(x => x.exact && x.alvo && /get_json|TestServer|fix_/.test(x.text + x.p.name))) console.log('  ', PT(x.c.test), x.at, N[x.c.owner].label, x.text.slice(0, 50), N[x.t.cls].label, x.c.res, x.alvo[0], `${x.c.path}@${x.c.sb}-${x.c.eb}`);
  fs.writeFileSync(outFile, JSON.stringify(A.filter(x => x.extra?.length).map(x => ({ at: x.at, occ: `${x.c.path}@${x.c.sb}-${x.c.eb}`, text: x.text, rule: x.t.rule, hoje: x.c.res, alvos: x.c.targets.map(show), cand: x.cand, extra: x.extra, exact: x.exact, super_: x.super_, sobrescrevem_quem_chama: x.menos })), null, 1) + '\n');
};
