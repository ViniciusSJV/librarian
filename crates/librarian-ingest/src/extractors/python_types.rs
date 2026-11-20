//! Step 2.3a: the written type of a Python method call's receiver, read from one file. Only what
//! is written: an annotated parameter or variable, `self` in a method, a constructor call, a field
//! of a class of the same file. Which class a written name means is not decided here: the resolver
//! reads it at query time from the file's imports. Never inferred as a type checker would.
use crate::resolve::ClassFacts;
use rustpython_ast::{self as ast, text_size::TextRange, Ranged, Visitor};
use std::collections::{BTreeMap, BTreeSet};

/// A receiver whose type is written: its span, the type as written (`T` or `m.T`) and the rule.
pub(crate) struct Typed {
    pub start: usize,
    pub end: usize,
    pub written: String,
    pub rule: &'static str,
}

/// What one binding of a name, or one write of a field, gives.
#[derive(Clone, Debug, PartialEq)]
enum Val {
    /// An annotation: the written type, and whether it is a parameter's.
    Ann(String, bool),
    /// `T(..)` or `m.T(..)`: the written callee.
    Call(String),
    /// The method's own receiver: the class.
    SelfOf(String),
    None,
    /// Another name (`self.x = p`): read through the method's parameter annotations.
    Name(String),
    Unknown,
}

fn span(range: TextRange, offset: usize) -> (usize, usize) {
    (usize::from(range.start()) + offset, usize::from(range.end()) + offset)
}

fn is_none(e: &ast::Expr) -> bool {
    matches!(e, ast::Expr::Constant(c) if matches!(c.value, ast::Constant::None)) || matches!(e, ast::Expr::Name(n) if n.id.as_str() == "None")
}

/// A dotted name as written: `T`, `m.T`, `a.b.T`. None for anything else.
fn dotted(e: &ast::Expr) -> Option<String> {
    match e {
        ast::Expr::Name(n) => Some(n.id.to_string()),
        ast::Expr::Attribute(a) => Some(format!("{}.{}", dotted(&a.value)?, a.attr)),
        _ => None,
    }
}

/// The type an annotation names: `T`, `m.T`, `"T"`, `T | None`, `Optional[T]`, `Union[T, None]`,
/// `Annotated[T, ..]`. None for `None`, a union of two types, a collection or `type[T]`.
fn annotation(e: &ast::Expr) -> Option<String> {
    match e {
        ast::Expr::Name(n) if n.id.as_str() == "None" => None,
        ast::Expr::Name(_) | ast::Expr::Attribute(_) => dotted(e),
        ast::Expr::Constant(c) => match &c.value {
            ast::Constant::Str(text) => {
                use rustpython_parser::Parse;
                annotation(&ast::Expr::parse(text, "<annotation>").ok()?)
            }
            _ => None,
        },
        ast::Expr::BinOp(b) if matches!(b.op, ast::Operator::BitOr) => match (is_none(&b.left), is_none(&b.right)) {
            (false, true) => annotation(&b.left),
            (true, false) => annotation(&b.right),
            _ => None,
        },
        ast::Expr::Subscript(s) => {
            let outer = match s.value.as_ref() {
                ast::Expr::Name(n) => n.id.to_string(),
                ast::Expr::Attribute(a) => a.attr.to_string(),
                _ => return None,
            };
            match (outer.as_str(), s.slice.as_ref()) {
                ("Optional", slice) => annotation(slice),
                ("Annotated", ast::Expr::Tuple(t)) => t.elts.first().and_then(annotation),
                ("Union", ast::Expr::Tuple(t)) => {
                    let rest: Vec<&ast::Expr> = t.elts.iter().filter(|x| !is_none(x)).collect();
                    if rest.len() == 1 { annotation(rest[0]) } else { None }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn value(e: &ast::Expr) -> Val {
    match e {
        _ if is_none(e) => Val::None,
        ast::Expr::Name(n) => Val::Name(n.id.to_string()),
        ast::Expr::Call(c) => dotted(&c.func).map_or(Val::Unknown, Val::Call),
        _ => Val::Unknown,
    }
}

/// The names a function binds in its own body (nested scopes apart), and what each binding gives;
/// `outer` are the names it declares `global` or `nonlocal`.
#[derive(Default)]
struct Binds {
    vals: BTreeMap<String, Vec<Val>>,
    outer: BTreeSet<String>,
}

impl Binds {
    fn add(&mut self, name: &str, v: Val) {
        self.vals.entry(name.to_owned()).or_default().push(v);
    }

    fn target(&mut self, t: &ast::Expr, v: Val) {
        match t {
            ast::Expr::Name(n) => self.add(&n.id, v),
            ast::Expr::Tuple(x) => x.elts.iter().for_each(|e| self.target(e, Val::Unknown)),
            ast::Expr::List(x) => x.elts.iter().for_each(|e| self.target(e, Val::Unknown)),
            ast::Expr::Starred(s) => self.target(&s.value, Val::Unknown),
            _ => {}
        }
    }

    fn collect(&mut self, body: &[ast::Stmt]) {
        for s in body {
            match s {
                ast::Stmt::Assign(a) => a.targets.iter().for_each(|t| self.target(t, value(&a.value))),
                ast::Stmt::AnnAssign(a) => self.target(&a.target, annotation(&a.annotation).map_or(Val::Unknown, |t| Val::Ann(t, false))),
                ast::Stmt::AugAssign(a) => self.target(&a.target, Val::Unknown),
                ast::Stmt::For(f) => {
                    self.target(&f.target, Val::Unknown);
                    self.collect(&f.body);
                    self.collect(&f.orelse);
                }
                ast::Stmt::AsyncFor(f) => {
                    self.target(&f.target, Val::Unknown);
                    self.collect(&f.body);
                    self.collect(&f.orelse);
                }
                ast::Stmt::While(w) => {
                    self.collect(&w.body);
                    self.collect(&w.orelse);
                }
                ast::Stmt::If(i) => {
                    self.collect(&i.body);
                    self.collect(&i.orelse);
                }
                ast::Stmt::With(w) => {
                    w.items.iter().filter_map(|i| i.optional_vars.as_deref()).for_each(|t| self.target(t, Val::Unknown));
                    self.collect(&w.body);
                }
                ast::Stmt::AsyncWith(w) => {
                    w.items.iter().filter_map(|i| i.optional_vars.as_deref()).for_each(|t| self.target(t, Val::Unknown));
                    self.collect(&w.body);
                }
                ast::Stmt::Try(t) => self.handlers(&t.body, &t.handlers, &t.orelse, &t.finalbody),
                ast::Stmt::TryStar(t) => self.handlers(&t.body, &t.handlers, &t.orelse, &t.finalbody),
                ast::Stmt::Match(m) => {
                    // A name captured by a case is not read: every case's body is, and the names
                    // its patterns capture lose their type.
                    for case in &m.cases {
                        let mut captured = Captures::default();
                        captured.visit_pattern(case.pattern.clone());
                        captured.0.iter().for_each(|n| self.add(n, Val::Unknown));
                        self.collect(&case.body);
                    }
                }
                ast::Stmt::Import(i) => i.names.iter().for_each(|a| {
                    let bound = a.asname.as_ref().map_or_else(|| a.name.split('.').next().unwrap_or_default().to_owned(), |x| x.to_string());
                    self.add(&bound, Val::Unknown)
                }),
                ast::Stmt::ImportFrom(i) => i.names.iter().for_each(|a| self.add(a.asname.as_ref().unwrap_or(&a.name), Val::Unknown)),
                ast::Stmt::Global(g) => self.outer.extend(g.names.iter().map(|n| n.to_string())),
                ast::Stmt::Nonlocal(g) => self.outer.extend(g.names.iter().map(|n| n.to_string())),
                ast::Stmt::FunctionDef(f) => self.add(&f.name, Val::Unknown),
                ast::Stmt::AsyncFunctionDef(f) => self.add(&f.name, Val::Unknown),
                ast::Stmt::ClassDef(c) => self.add(&c.name, Val::Unknown),
                ast::Stmt::Delete(d) => d.targets.iter().for_each(|t| self.target(t, Val::Unknown)),
                _ => {}
            }
        }
    }

    fn handlers(&mut self, body: &[ast::Stmt], handlers: &[ast::ExceptHandler], orelse: &[ast::Stmt], finalbody: &[ast::Stmt]) {
        self.collect(body);
        for h in handlers {
            let ast::ExceptHandler::ExceptHandler(h) = h;
            if let Some(n) = &h.name {
                self.add(n, Val::Unknown);
            }
            self.collect(&h.body);
        }
        self.collect(orelse);
        self.collect(finalbody);
    }
}

/// Names a `case` pattern captures.
#[derive(Default)]
struct Captures(Vec<String>);

impl Visitor for Captures {
    fn visit_pattern_match_as(&mut self, node: ast::PatternMatchAs) {
        if let Some(n) = &node.name {
            self.0.push(n.to_string());
        }
        self.generic_visit_pattern_match_as(node);
    }
    fn visit_pattern_match_star(&mut self, node: ast::PatternMatchStar) {
        if let Some(n) = &node.name {
            self.0.push(n.to_string());
        }
    }
    fn visit_pattern_match_mapping(&mut self, node: ast::PatternMatchMapping) {
        if let Some(n) = &node.rest {
            self.0.push(n.to_string());
        }
        self.generic_visit_pattern_match_mapping(node);
    }
}

/// What a set of bindings gives, and by which rule: an annotation wins (a parameter's is
/// `parametro`); otherwise every binding must agree (`None` does not count).
fn settle(vals: &[Val]) -> Option<(Val, &'static str)> {
    let anns: Vec<&Val> = vals.iter().filter(|v| matches!(v, Val::Ann(..))).collect();
    if let Some(first) = anns.first() {
        let same = anns.iter().all(|a| matches!((a, first), (Val::Ann(x, _), Val::Ann(y, _)) if x == y));
        let rule = if matches!(first, Val::Ann(_, true)) { "parametro" } else { "anotacao" };
        return same.then(|| ((*first).clone(), rule));
    }
    let rest: Vec<&Val> = vals.iter().filter(|v| !matches!(v, Val::None)).collect();
    let first = *rest.first()?;
    if !rest.iter().all(|v| *v == first) {
        return None;
    }
    match first {
        Val::Call(_) => Some((first.clone(), "construtor")),
        Val::SelfOf(_) => Some((first.clone(), "self")),
        _ => None,
    }
}

/// The fields of the classes of this file, by class name: what each write gives.
#[derive(Default)]
struct Fields(BTreeMap<String, BTreeMap<String, Vec<Val>>>);

impl Fields {
    fn read(&mut self, suite: &[ast::Stmt], classes: &mut BTreeMap<String, ClassFacts>, scope: &str) {
        for s in suite {
            let ast::Stmt::ClassDef(c) = s else { continue };
            let qualified = if scope.is_empty() { c.name.to_string() } else { format!("{scope}.{}", c.name) };
            let last = |e: &ast::Expr| match e {
                ast::Expr::Name(n) => Some(n.id.to_string()),
                ast::Expr::Attribute(a) => Some(a.attr.to_string()),
                ast::Expr::Subscript(s) => match s.value.as_ref() {
                    ast::Expr::Name(n) => Some(n.id.to_string()),
                    ast::Expr::Attribute(a) => Some(a.attr.to_string()),
                    _ => None,
                },
                _ => None,
            };
            let facts = classes.entry(qualified.clone()).or_default();
            facts.based |= c.bases.iter().any(|b| last(b).as_deref() != Some("object"));
            facts.protocol |= c.bases.iter().any(|b| last(b).as_deref() == Some("Protocol"));
            let writes = self.0.entry(c.name.to_string()).or_default();
            for item in &c.body {
                match item {
                    ast::Stmt::AnnAssign(a) => {
                        let classvar = matches!(a.annotation.as_ref(), ast::Expr::Subscript(s) if matches!(s.value.as_ref(), ast::Expr::Name(n) if n.id.as_str() == "ClassVar"));
                        if let (ast::Expr::Name(n), Some(t), false) = (a.target.as_ref(), annotation(&a.annotation), classvar) {
                            writes.entry(n.id.to_string()).or_default().push(Val::Ann(t, false));
                        }
                    }
                    ast::Stmt::FunctionDef(f) => {
                        facts.getattr |= matches!(f.name.as_str(), "__getattr__" | "__getattribute__");
                        method_writes(&f.name, &f.args, &f.body, writes);
                    }
                    ast::Stmt::AsyncFunctionDef(f) => method_writes(&f.name, &f.args, &f.body, writes),
                    _ => {}
                }
            }
            self.read(&c.body, classes, &qualified);
        }
    }
}

/// The writes of `self.<field>` in one method. Only `__init__` gives a type by a constructor or
/// by a parameter; any other write in another method, except `None`, takes the type away.
fn method_writes(name: &str, args: &ast::Arguments, body: &[ast::Stmt], writes: &mut BTreeMap<String, Vec<Val>>) {
    let params: BTreeMap<String, String> = args
        .posonlyargs
        .iter()
        .chain(&args.args)
        .chain(&args.kwonlyargs)
        .filter_map(|a| Some((a.def.arg.to_string(), annotation(a.def.annotation.as_deref()?)?)))
        .collect();
    let field = |e: &ast::Expr| match e {
        ast::Expr::Attribute(a) if matches!(a.value.as_ref(), ast::Expr::Name(n) if n.id.as_str() == "self") => Some(a.attr.to_string()),
        _ => None,
    };
    let mut stack: Vec<&ast::Stmt> = body.iter().collect();
    while let Some(s) = stack.pop() {
        match s {
            ast::Stmt::Assign(a) => {
                for t in &a.targets {
                    let Some(f) = field(t) else { continue };
                    let v = match (name, value(&a.value)) {
                        (_, Val::None) => Val::None,
                        ("__init__", Val::Name(p)) => params.get(&p).map_or(Val::Unknown, |t| Val::Ann(t.clone(), false)),
                        ("__init__", v @ Val::Call(_)) => v,
                        _ => Val::Unknown,
                    };
                    writes.entry(f).or_default().push(v);
                }
            }
            ast::Stmt::AnnAssign(a) => {
                if let (Some(f), Some(t)) = (field(&a.target), annotation(&a.annotation)) {
                    writes.entry(f).or_default().push(Val::Ann(t, false));
                }
            }
            ast::Stmt::AugAssign(a) => {
                if let Some(f) = field(&a.target) {
                    writes.entry(f).or_default().push(Val::Unknown);
                }
            }
            ast::Stmt::If(i) => stack.extend(i.body.iter().chain(&i.orelse)),
            ast::Stmt::For(f) => stack.extend(f.body.iter().chain(&f.orelse)),
            ast::Stmt::While(f) => stack.extend(f.body.iter().chain(&f.orelse)),
            ast::Stmt::With(f) => stack.extend(f.body.iter()),
            ast::Stmt::Try(t) => {
                stack.extend(t.body.iter().chain(&t.orelse).chain(&t.finalbody));
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    stack.extend(h.body.iter());
                }
            }
            _ => {}
        }
    }
}

/// One function's bindings (a lambda's or a comprehension's: its parameters or targets, unknown).
struct Scope(Binds);

struct Walk<'a> {
    offset: usize,
    module: Binds,
    scopes: Vec<Scope>,
    /// Enclosing classes, and how many function scopes were open when each was entered.
    classes: Vec<(String, usize)>,
    fields: &'a Fields,
    out: Vec<Typed>,
}

impl Walk<'_> {
    fn name_type(&self, name: &str) -> Option<(Val, &'static str)> {
        for s in self.scopes.iter().rev() {
            if s.0.outer.contains(name) {
                break;
            }
            if let Some(v) = s.0.vals.get(name) {
                return settle(v);
            }
        }
        self.module.vals.get(name).and_then(|v| settle(v))
    }

    /// The type a receiver is read as: the class name or written type, and the rule.
    fn type_of(&self, e: &ast::Expr) -> Option<(String, &'static str)> {
        let typed = |v: Val, rule| match v {
            Val::Ann(t, _) | Val::Call(t) => Some((t, rule)),
            Val::SelfOf(c) => Some((c, "self")),
            _ => None,
        };
        match e {
            ast::Expr::Name(n) => {
                let (v, rule) = self.name_type(n.id.as_str())?;
                typed(v, rule)
            }
            // A field of a class of this file: `self.x`, or `a.x` with `a` of such a class.
            ast::Expr::Attribute(a) => {
                let (base, _) = self.type_of(&a.value)?;
                if base.contains('.') {
                    return None;
                }
                let (v, _) = settle(self.fields.0.get(&base)?.get(a.attr.as_str())?)?;
                typed(v, "campo")
            }
            ast::Expr::Call(c) => typed(Val::Call(dotted(&c.func)?), "construtor"),
            _ => None,
        }
    }

    fn function(&mut self, args: &ast::Arguments, body: &[ast::Stmt], decorators: &[ast::Expr]) {
        for d in decorators {
            self.visit_expr(d.clone());
        }
        let params: Vec<&ast::ArgWithDefault> = args.posonlyargs.iter().chain(&args.args).chain(&args.kwonlyargs).collect();
        for p in &params {
            if let Some(d) = &p.default {
                self.visit_expr(*d.clone());
            }
        }
        let method = self.classes.last().is_some_and(|(_, depth)| *depth == self.scopes.len());
        let bound_first = !decorators.iter().any(|d| matches!(d, ast::Expr::Name(n) if matches!(n.id.as_str(), "staticmethod" | "classmethod")));
        let mut binds = Binds::default();
        for (i, p) in params.iter().enumerate() {
            let v = match p.def.annotation.as_deref().and_then(annotation) {
                Some(t) => Val::Ann(t, true),
                None if i == 0 && method && bound_first => Val::SelfOf(self.classes.last().map(|(c, _)| c.clone()).unwrap_or_default()),
                None => Val::Unknown,
            };
            binds.add(&p.def.arg, v);
        }
        for a in args.vararg.iter().chain(&args.kwarg) {
            binds.add(&a.arg, Val::Unknown);
        }
        binds.collect(body);
        self.scopes.push(Scope(binds));
        for s in body {
            self.visit_stmt(s.clone());
        }
        self.scopes.pop();
    }

    /// A lambda or a comprehension: its names are its own, with no type.
    fn opaque(&mut self, names: Vec<String>, visit: impl FnOnce(&mut Self)) {
        let mut binds = Binds::default();
        names.iter().for_each(|n| binds.add(n, Val::Unknown));
        self.scopes.push(Scope(binds));
        visit(self);
        self.scopes.pop();
    }

    fn comprehension(&mut self, generators: Vec<ast::Comprehension>, elements: Vec<ast::Expr>) {
        let mut names = Binds::default();
        for g in &generators {
            names.target(&g.target, Val::Unknown);
        }
        self.opaque(names.vals.into_keys().collect(), |w| {
            for g in generators {
                w.visit_expr(g.iter);
                g.ifs.into_iter().for_each(|i| w.visit_expr(i));
            }
            elements.into_iter().for_each(|e| w.visit_expr(e));
        });
    }
}

impl Visitor for Walk<'_> {
    fn visit_stmt_function_def(&mut self, node: ast::StmtFunctionDef) {
        self.function(&node.args, &node.body, &node.decorator_list);
    }
    fn visit_stmt_async_function_def(&mut self, node: ast::StmtAsyncFunctionDef) {
        self.function(&node.args, &node.body, &node.decorator_list);
    }
    fn visit_stmt_class_def(&mut self, node: ast::StmtClassDef) {
        for e in node.decorator_list.into_iter().chain(node.bases) {
            self.visit_expr(e);
        }
        self.classes.push((node.name.to_string(), self.scopes.len()));
        for s in node.body {
            self.visit_stmt(s);
        }
        self.classes.pop();
    }
    fn visit_expr_call(&mut self, node: ast::ExprCall) {
        if let ast::Expr::Attribute(a) = node.func.as_ref() {
            if let Some((written, rule)) = self.type_of(&a.value) {
                let (start, end) = span(a.value.range(), self.offset);
                self.out.push(Typed { start, end, written, rule });
            }
        }
        self.generic_visit_expr_call(node);
    }
    fn visit_arguments(&mut self, node: ast::Arguments) {
        for a in node.posonlyargs.into_iter().chain(node.args).chain(node.kwonlyargs) {
            if let Some(d) = a.default {
                self.visit_expr(*d);
            }
        }
    }
    fn visit_expr_lambda(&mut self, node: ast::ExprLambda) {
        let names = node.args.posonlyargs.iter().chain(&node.args.args).chain(&node.args.kwonlyargs).map(|a| a.def.arg.to_string()).collect();
        self.opaque(names, |w| w.visit_expr(*node.body));
    }
    fn visit_expr_list_comp(&mut self, node: ast::ExprListComp) {
        self.comprehension(node.generators, vec![*node.elt]);
    }
    fn visit_expr_set_comp(&mut self, node: ast::ExprSetComp) {
        self.comprehension(node.generators, vec![*node.elt]);
    }
    fn visit_expr_generator_exp(&mut self, node: ast::ExprGeneratorExp) {
        self.comprehension(node.generators, vec![*node.elt]);
    }
    fn visit_expr_dict_comp(&mut self, node: ast::ExprDictComp) {
        self.comprehension(node.generators, vec![*node.key, *node.value]);
    }
    fn visit_withitem(&mut self, node: ast::WithItem) {
        self.visit_expr(node.context_expr);
        if let Some(t) = node.optional_vars {
            self.visit_expr(*t);
        }
    }
    fn visit_match_case(&mut self, node: ast::MatchCase) {
        if let Some(g) = node.guard {
            self.visit_expr(*g);
        }
        for s in node.body {
            self.visit_stmt(s);
        }
    }
    fn visit_keyword(&mut self, node: ast::Keyword) {
        self.visit_expr(node.value);
    }
}

/// The typed receivers of one file and the facts of its classes.
pub(crate) fn read(suite: &[ast::Stmt], offset: usize) -> (Vec<Typed>, BTreeMap<String, ClassFacts>) {
    let mut classes = BTreeMap::new();
    let mut fields = Fields::default();
    fields.read(suite, &mut classes, "");
    let mut module = Binds::default();
    module.collect(suite);
    let mut walk = Walk { offset, module, scopes: vec![], classes: vec![], fields: &fields, out: vec![] };
    for s in suite {
        walk.visit_stmt(s.clone());
    }
    (walk.out, classes)
}
