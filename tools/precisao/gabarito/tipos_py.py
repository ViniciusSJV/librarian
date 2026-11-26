# Gabarito de tipos do Python (passo 2.4e, DEC-48): para cada chamada dos arquivos do projeto, a declaracao
# que o mypy (com o plugin do django-stubs, pelo mypy.ini do projeto) da como alvo. E so regua: nunca entra no
# grafo nem no acervo (DEC-48, item 6). Mesma logica do `dmypy inspect --show definition`
# (mypy/inspections.py, `collect_nodes`), numa passada so, sem cache.
#
#   <python do virtualenv> tipos_py.py <raiz do projeto> <saida.jsonl> <alvo do mypy, ex.: hc> [opcoes do mypy...]
#   ex.: ... tipos_py.py C:/.../healthchecks C:/.../tipos-hc.jsonl hc --strict --platform linux
#
# Cada linha: {"arquivo", "chamada": [linha, coluna, linha_fim, coluna_fim], "chamado": [...] (a expressao
# chamada), "texto" (nome chamado), "alvos": [{"arquivo", "linha", "linha_def", "nome", "fullname", "tipo"}],
# "motivo"} — sem alvo, `motivo` diz por que ("any", "sem_tipo", "nao_ref", "sem_no"). Linhas 1-based, colunas
# 0-based em bytes UTF-8 (as do ast do Python).
import sys, os, json, time

ROOT = os.path.normcase(os.path.abspath(sys.argv[1]))
OUT = sys.argv[2]
ARGS = sys.argv[3:]
os.chdir(ROOT)
sys.path.insert(0, ROOT)

from mypy import build
from mypy.main import process_options
from mypy.nodes import (CallExpr, MemberExpr, NameExpr, RefExpr, Decorator, FuncDef, OverloadedFuncDef, TypeInfo, Var,
                        MypyFile)
from mypy.types import get_proper_type, AnyType, FunctionLike
from mypy.server.subexpr import get_subexpressions
from mypy.inspections import get_instance_fallback, find_node, find_module_by_fullname
from mypy.typevars import fill_typevars_with_any

t0 = time.time()
sources, options = process_options(ARGS)
options.export_types = True
options.preserve_asts = True
options.incremental = False
options.cache_dir = os.environ.get("GABARITO_MYPY_CACHE", os.path.join(os.path.dirname(os.path.abspath(OUT)), "mypy-cache"))
res = build.build(sources, options)
all_types = res.manager.all_types
graph = res.graph

def rel(p):
    if not p:
        return None
    a = os.path.normcase(os.path.abspath(p))
    return a[len(ROOT) + 1:].replace("\\", "/") if a.startswith(ROOT + os.sep) else a

def nodes_of(e):
    """O que a expressao chamada pode referir (inspections.py, collect_nodes)."""
    node = e.node
    if node is not None:
        return [node], None
    if isinstance(e, MemberExpr) and e.kind is None:
        base = all_types.get(e.expr)
        if base is None:
            return [], "sem_tipo"
        base = get_proper_type(base)
        if isinstance(base, AnyType):
            return [], "any"
        out = [n for i in get_instance_fallback(base) for n in [find_node(e.name, i.type)] if n]
        if not out and isinstance(base, FunctionLike) and base.is_type_obj():
            out = [n for i in get_instance_fallback(fill_typevars_with_any(base.type_object())) for n in [find_node(e.name, i.type)] if n]
        return out, (None if out else "sem_no")
    return [], "sem_no"

def describe(n, module_path):
    tipo = type(n).__name__
    line = n.line
    linha_def = None
    if isinstance(n, Decorator):
        linha_def = n.func.line
        line = min([d.line for d in n.decorators] + [n.func.line])
    elif isinstance(n, OverloadedFuncDef) and n.items:
        first = n.items[0]
        linha_def = first.func.line if isinstance(first, Decorator) else first.line
    elif isinstance(n, FuncDef):
        linha_def = n.line
    elif isinstance(n, TypeInfo):
        line = n.line
    mod = find_module_by_fullname(n.fullname, graph)
    path = rel(mod.path) if mod is not None else module_path
    out = {"arquivo": path, "linha": line, "linha_def": linha_def, "nome": n.name, "fullname": n.fullname, "tipo": tipo}
    info = getattr(n.func, "info", None) if isinstance(n, Decorator) else getattr(n, "info", None)
    # A module-level function has a FakeInfo (a TypeInfo that raises when read): only a real class has overrides.
    try:
        cls_name = info.fullname if isinstance(info, TypeInfo) and not isinstance(n, TypeInfo) else None
    except AttributeError:
        cls_name = None
    if cls_name:
        out["sobrescritas"] = overrides(info, n.name)
    return out

# The project's classes (top level and nested), for "concorda pela base" (Parada 1, Q6): the methods of the
# project that override the one mypy gives, by the MRO mypy computed.
def _infos():
    out = []
    for state in graph.values():
        p = rel(state.path) if state.path else None
        if not p or os.path.isabs(p) or state.tree is None:
            continue
        stack = list(state.tree.names.values())
        seen = set()
        while stack:
            sym = stack.pop()
            ti = sym.node
            if isinstance(ti, TypeInfo) and ti.fullname not in seen and ti.module_name == state.id:
                seen.add(ti.fullname)
                out.append((p, ti))
                stack.extend(ti.names.values())
    return out
PROJECT_INFOS = _infos()
_over = {}
def overrides(info, name):
    k = (info.fullname, name)
    if k not in _over:
        res_ = []
        for p, ti in PROJECT_INFOS:
            if ti is info or info not in ti.mro or name not in ti.names:
                continue
            m = ti.names[name].node
            if m is None:
                continue
            line = m.func.line if isinstance(m, Decorator) else getattr(m, "line", None)
            res_.append({"arquivo": p, "linha": line, "nome": name, "classe": ti.fullname})
        _over[k] = res_
    return _over[k]

def record(path, o, f, tm):
    e = o.callee
    rec = {"arquivo": path, "chamada": [o.line, o.column, o.end_line, o.end_column],
           "chamado": [e.line, e.column, e.end_line, e.end_column], "texto": getattr(e, "name", None), "alvos": []}
    if isinstance(e, RefExpr):
        ns, why = nodes_of(e)
        for n in ns:
            try:
                rec["alvos"].append(describe(n, path))
            except Exception as ex:  # never let one call stop the pass
                rec.setdefault("erros", []).append(repr(ex))
        if not rec["alvos"]:
            rec["motivo"] = why or "sem_no"
    else:
        rec["motivo"] = "nao_ref"
        t = tm.get(e)
        if t is not None and isinstance(get_proper_type(t), AnyType):
            rec["motivo"] = "any"
    f.write(json.dumps(rec, ensure_ascii=False) + "\n")

total = 0
files = 0
with open(OUT, "w", encoding="utf-8") as f:
    for mid, state in sorted(graph.items()):
        p = rel(state.path) if state.path else None
        if not p or os.path.isabs(p) or state.tree is None:
            continue
        # Every call expression of the module (mypy/server/subexpr.py); the types come from the whole build.
        tm = all_types
        calls = sorted((e for e in get_subexpressions(state.tree) if isinstance(e, CallExpr)), key=lambda e: (e.line, e.column, e.end_line or 0, e.end_column or 0))
        for o in calls:
            record(p, o, f, tm)
        total += len(calls)
        files += 1
errors = [l for l in res.errors if ": error:" in l]
print(f"tipos: {files} arquivos, {total} chamadas, {len(errors)} erros do mypy, {time.time() - t0:.1f} s", file=sys.stderr)
for l in errors[:20]:
    print("  " + l, file=sys.stderr)
