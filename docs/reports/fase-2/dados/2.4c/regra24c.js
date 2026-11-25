// Passo 2.4c, Parada 1: a regra do despacho polimórfico, aplicada a cada chamada com tipo estático.
'use strict';
module.exports = get => {
  const all = [];
  let X;
  const f = (c, p, t, r, text, at) => {
    X ??= get();
    const { N, info, lookup, descendants, show, R, up, last } = X;
    if (!r.found) { all.push({ c, p, t, r, text, at }); return; }
    const desc = descendants(t.cls);
    // O corpo que roda em cada subclasse (a busca na ordem dela), não só o que ela declara.
    const bodies = new Set(), abertas = [], declaram = [];
    for (const d of desc) {
      const x = lookup(d, p.name);
      if (x.found) x.found.forEach(m => bodies.add(m));
      else abertas.push(N[d].label + ' ' + JSON.stringify(x));
      if (info[d].methods[p.name]) declaram.push(d);
    }
    const abs = m => R.abstract(m) && !/fora da regra/.test(R.abstract(m));
    const base = r.found;
    const extra = [...bodies].filter(m => !base.includes(m) && !abs(m));
    // Avaliado à parte: subclasses que também sobrescrevem quem chama (self/cls) só chegam aqui por super().
    let menos = null;
    if (extra.length && ['self', 'cls'].includes(t.rule)) {
      const m = up(c.owner, ['method']), n = last(N[m].label);
      menos = desc.filter(d => { const x = lookup(d, n); return x.found && !x.found.includes(m); }).map(d => N[d].label);
    }
    const super_ = !!t.skip, exact = t.rule === 'construtor';
    let cand = null;
    if (!super_ && !exact && extra.length) cand = [...(base.length === 1 && abs(base[0]) ? [] : base), ...extra];
    all.push({ c, p, t, r, text, at, extra: extra.map(show).sort(), cand: cand && cand.map(show).sort(), exact, super_, abertas, menos, ndesc: desc.length, declaram: declaram.map(d => N[d].label), baseAbs: base.length === 1 && !!abs(base[0]), alvo: base.map(show), multi: base.length > 1 });
  };
  f.all = all;
  return f;
};
