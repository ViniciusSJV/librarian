# Gabarito de execucao do Python (passo 2.4e, DEC-48): roda um programa (a suite de testes do projeto)
# gravando, com sys.monitoring (Python 3.12+), cada par diferente (local da chamada, alvo) que rodou.
# E so regua: nunca entra no grafo nem no acervo (DEC-48, item 6). Nao muda o projeto.
#
#   <python do virtualenv> execucao_py.py <raiz do projeto> <saida.jsonl> <programa> [argumentos...]
#   ex.: ... execucao_py.py C:/.../healthchecks C:/.../execucao-hc.jsonl manage.py test
#
# Rode com PYTHONDONTWRITEBYTECODE=1 e sem `python -I` (o -I ignora essa variavel e grava .pyc no projeto).
#
# Cada linha da saida e um par:
#   tipo "chamada": chamador dentro do projeto. `pos` = [linha, linha_fim, coluna, coluna_fim] da chamada
#     (co_positions; colunas em bytes UTF-8 a partir de 0), `funcao` = co_qualname de quem chama.
#   tipo "raiz": chamador fora do projeto (Django, unittest, concurrent.futures) e alvo dentro dele.
#   `alvo` = lista: o objeto chamado e, se houver, a cadeia `__wrapped__` dos decoradores. Cada elemento:
#     {"arquivo", "linha", "nome"} (funcao Python), {"classe", "arquivo", "init"} (instanciacao),
#     {"duble": "mock", "nome"} (unittest.mock), ou {"externo": modulo, "nome"} (sem codigo Python).
#   `fio` = nome da thread em que o par apareceu primeiro; `outra_thread` = se apareceu fora da principal.
import sys, os, json, runpy, time, threading, types

ROOT = os.path.normcase(os.path.abspath(sys.argv[1]))
OUT = sys.argv[2]
PREFIX = ROOT + os.sep
mon = sys.monitoring
TOOL = mon.PROFILER_ID
mon.use_tool_id(TOOL, "gabarito-2.4e")
MAIN = threading.main_thread()

_files = {}
def rel(fn):
    """Caminho relativo a raiz, com '/', ou None se fora do projeto ou do virtualenv."""
    r = _files.get(fn, 0)
    if r != 0:
        return r
    r = None
    if fn and not fn.startswith("<"):
        p = os.path.normcase(os.path.abspath(fn))
        if p.startswith(PREFIX):
            r = p[len(PREFIX):].replace("\\", "/")
            if "site-packages/" in r or r.startswith((".venv", "venv")):
                r = None
    _files[fn] = r
    return r

_pos = {}
def position(code, offset):
    ps = _pos.get(code)
    if ps is None:
        ps = _pos[code] = list(code.co_positions())
    i = offset // 2
    return list(ps[i]) if i < len(ps) else None

def code_of(f):
    f = getattr(f, "__func__", f)
    co = getattr(f, "__code__", None)
    return co if isinstance(co, types.CodeType) else None

def element(f):
    f = getattr(f, "__func__", f)
    co = code_of(f)
    if co is not None:
        return {"arquivo": rel(co.co_filename) or co.co_filename, "linha": co.co_firstlineno, "nome": co.co_qualname}
    t = type(f)
    if t.__module__ == "unittest.mock":
        return {"duble": "mock", "nome": t.__qualname__}
    if isinstance(f, type):
        mod = sys.modules.get(f.__module__)
        out = {"classe": f.__module__ + "." + f.__qualname__, "arquivo": rel(getattr(mod, "__file__", "") or "")}
        for k in f.__mro__:
            init = k.__dict__.get("__init__")
            if init is not None:
                ic = code_of(init)
                if ic is not None:
                    out["init"] = {"arquivo": rel(ic.co_filename) or ic.co_filename, "linha": ic.co_firstlineno, "nome": ic.co_qualname}
                break
        return out
    owner = getattr(f, "__self__", None)
    name = getattr(f, "__qualname__", None) or getattr(f, "__name__", None) or t.__qualname__
    if owner is not None and not isinstance(owner, type(sys)) and "." not in name:
        name = type(owner).__qualname__ + "." + name
    mod = getattr(f, "__module__", None) or (type(owner).__module__ if owner is not None else t.__module__)
    return {"externo": mod or "", "nome": name}

def chain(c):
    out, f, n = [], c, 0
    while f is not None and n < 10:
        out.append(element(f))
        f = getattr(getattr(f, "__func__", f), "__wrapped__", None)
        n += 1
    return out

seen = {}
roots = {}

def on_call(code, offset, callable, arg0):
    caller = rel(code.co_filename)
    if caller is None:
        tc = code_of(callable)
        if tc is None or rel(tc.co_filename) is None:
            return
        key = (code, id(tc))
        r = roots.get(key)
        if r is None:
            roots[key] = r = {"tipo": "raiz", "chamador_externo": code.co_filename, "funcao": code.co_qualname,
                              "alvo": chain(callable), "fio": threading.current_thread().name, "outra_thread": False}
        if not r["outra_thread"] and threading.current_thread() is not MAIN:
            r["outra_thread"] = True
        return
    key = (code, offset, id(getattr(callable, "__func__", callable)) if not isinstance(callable, type) else id(callable))
    r = seen.get(key)
    if r is None:
        seen[key] = r = {"tipo": "chamada", "chamador": caller, "pos": position(code, offset), "funcao": code.co_qualname,
                         "alvo": chain(callable), "fio": threading.current_thread().name, "outra_thread": False}
    if not r["outra_thread"] and threading.current_thread() is not MAIN:
        r["outra_thread"] = True

mon.register_callback(TOOL, mon.events.CALL, on_call)
sys.argv = sys.argv[3:]
sys.path.insert(0, ROOT)
os.chdir(ROOT)
t = time.time()
status = 0
mon.set_events(TOOL, mon.events.CALL)
try:
    runpy.run_path(sys.argv[0], run_name="__main__")
except SystemExit as e:
    status = e.code if isinstance(e.code, int) else (0 if e.code is None else 1)
finally:
    mon.set_events(TOOL, 0)
    with open(OUT, "w", encoding="utf-8") as f:
        for v in list(seen.values()) + list(roots.values()):
            f.write(json.dumps(v, ensure_ascii=False) + "\n")
    print(f"gabarito: {len(seen)} chamadas, {len(roots)} raizes, {time.time() - t:.1f} s, saida {status}", file=sys.stderr)
sys.exit(status)
