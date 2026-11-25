// Passo 2.4b, Parada 1 refeita: as regras 1 (método abstrato) e 2 (candidato fora da cadeia), incluído pelo
// py24b.js com o contexto dele.   (não roda sozinho)
'use strict';
module.exports = ctx => {
  const { N, P, info, mro, lookup, descendants, slice, declared, show, decorators, balanced, paramsAt, retOf, classes } = ctx;
  // O corpo, sem a docstring e os comentários.
  function body(m) {
    const d = declared[m], s = slice(d.path, d.start_byte, d.end_byte), c = balanced(s, paramsAt(s));
    const ret = s.slice(c + 1).match(/^\s*(->[^:]+(\[[^\]]*\])?)?\s*:/);
    if (!ret) return null;
    let b = s.slice(c + 1 + ret[0].length).split('\n').map(l => l.replace(/^\s*#.*$/, '')).join('\n').trim();
    b = b.replace(/^[rRuU]?("""[\s\S]*?"""|'''[\s\S]*?'''|"[^"\n]*"|'[^'\n]*')\s*/, '');
    return b.trim();
  }
  const why = {};
  function abstract(m) {
    if (m in why) return why[m];
    const b = body(m);
    let r = null;
    if (decorators(m).some(d => /(^|\.)abstract(method|property|classmethod|staticmethod)$/.test(d))) r = '@abstractmethod';
    else if (b !== null && /^raise\s+NotImplementedError(\([^\n]*\))?\s*(#.*)?$/.test(b)) r = 'raise NotImplementedError';
    else if (b === 'pass' || b === '...') r = b;
    else if (b === '') r = 'só docstring (fora da regra)';
    return why[m] = r;
  }
  // As sobrescritas que podem rodar para um receptor de tipo cls: nas subclasses do projeto, não abstratas.
  function overrides(cls, name) {
    return descendants(cls).flatMap(d => (info[d].methods[name] ?? [])).filter(m => !abstract(m) || /fora da regra/.test(abstract(m)));
  }
  // Regra 2: as classes do projeto de onde o método pode vir: a cadeia do receptor e a de cada subclasse.
  function reachable(cls, skip) {
    const out = new Set(), m = mro(cls);
    if (!m) return null;
    m.slice(skip).forEach(k => out.add(k));
    for (const d of descendants(cls)) {
      const md = mro(d);
      if (!md) return null;
      md.forEach(k => { if (!(skip && (k === cls || md.indexOf(k) < md.indexOf(cls)))) out.add(k); });
    }
    if (skip) out.delete(cls);
    return out;
  }
  // Uma base que a regra não sabe o que é (expressão, condicional, só em TYPE_CHECKING): pode trazer métodos do projeto.
  // Um decorador de classe ou uma metaclasse pode pôr métodos: só pesa nas classes alcançáveis (um super() pula a
  // própria classe e as subclasses), e o __getattr__ não vale num super().
  function opaqueUnknown(cls, skip) {
    const chain = [cls, ...descendants(cls)].flatMap(c => mro(c) ?? []).filter(k => info[k]);
    if (chain.some(k => info[k].bases.some(b => b.kind !== 'project' && (b.key ?? '').startsWith('expr:') || b.risk))) return true;
    return [...reachable(cls, skip)].filter(k => info[k]).some(k => info[k].meta.length || info[k].decos.length || info[k].protocol || (!skip && info[k].getattr));
  }
  return { abstract, overrides, reachable, opaqueUnknown, body };
};
