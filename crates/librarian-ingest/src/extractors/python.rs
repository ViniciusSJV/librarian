//! Python adapter (rustpython-parser). Only syntactic declarations and occurrences.
//! No import resolution, dynamic dispatch, decorator evaluation or runtime behavior.
use super::{Extraction, Extractor, Occurrence, OwnerSyntax};
use crate::{hash, Chunk, Diagnostic, Source, Symbol};
use rustpython_ast::{self as ast, text_size::TextRange, Ranged, Visitor};
use rustpython_parser::Parse;

/// `g6` adds fields, attribute accesses, and calls and bases as candidates (profile v3).
/// `v5` adds module variables, names used as values or written, keyword arguments, loops and
/// `with` blocks (profile v5). `v6` adds parameters, `x in name` comparisons and names bound to
/// a list in a function (profile v6).
/// Without them, records and occurrences are exactly those of the v2 adapter.
pub(crate) struct Python {
    g6: bool,
    v5: bool,
    v6: bool,
}

pub(crate) static PYTHON: Python = Python { g6: false, v5: false, v6: false };
pub(crate) static PYTHON_V3: Python = Python { g6: true, v5: false, v6: false };
pub(crate) static PYTHON_V5: Python = Python { g6: true, v5: true, v6: false };
pub(crate) static PYTHON_V6: Python = Python { g6: true, v5: true, v6: true };

impl Extractor for Python {
    fn extract(&self, source: &Source, bytes: &[u8]) -> Extraction {
        let failure = |code: &str, message: String| Extraction {
            symbols: vec![],
            chunks: vec![],
            diagnostics: vec![Diagnostic {
                source_id: source.id.clone(),
                code: code.into(),
                message,
            }],
        };
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => return failure("invalid_utf8", error.to_string()),
        };
        let (suite, offset) = match parse(text) {
            Ok(parsed) => parsed,
            Err(error) => return failure("python_parse_error", error.to_string()),
        };
        let mut walker = Walker::new(self, Some(source), bytes, offset);
        for stmt in suite {
            walker.visit_stmt(stmt);
        }
        Extraction {
            symbols: walker.symbols,
            chunks: walker.chunks,
            diagnostics: vec![],
        }
    }

    fn observe(&self, bytes: &[u8]) -> Vec<Occurrence> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return vec![];
        };
        let Ok((suite, offset)) = parse(text) else {
            return vec![];
        };
        let mut walker = Walker::new(self, None, bytes, offset);
        for stmt in suite {
            walker.visit_stmt(stmt);
        }
        walker.found
    }

    fn candidate_kinds(&self) -> &'static [&'static str] {
        &["class"]
    }

    fn candidate_kinds_for(&self, occurrence: &str) -> &'static [&'static str] {
        match (self.v5, occurrence) {
            // A module variable can name a type (`SessionDep = Annotated[...]`).
            (true, "type_reference") => return &["class", "variable"],
            (true, "name_reference" | "name_write") => return &["variable", "function", "class"],
            (true, "keyword_argument") => return &["field"],
            _ => {}
        }
        match (self.g6, occurrence) {
            // Calling a class constructs it, so classes are candidates of plain calls too.
            (true, "call_observed") => &["function", "method", "class"],
            (true, "method_call_observed") => &["method", "function"],
            (true, "attribute_access") => &["field"],
            (true, "base_class") => &["class"],
            _ => self.candidate_kinds(),
        }
    }

    /// A class spans all its methods, like a Rust impl; its methods are the answers.
    fn navigation_kinds(&self) -> &'static [&'static str] {
        &["class"]
    }

    fn owner_syntax(&self) -> Option<&'static OwnerSyntax> {
        self.g6.then_some(&OWNER)
    }

    fn call_kinds(&self) -> &'static [&'static str] {
        &["call_observed", "method_call_observed"]
    }

    fn matched_only(&self) -> &'static [&'static str] {
        if self.v5 {
            &["name_reference", "name_write", "keyword_argument"]
        } else {
            &[]
        }
    }

    /// A name that is not local is the module's own, when the module declares it.
    fn file_scoped(&self) -> &'static [&'static str] {
        if self.v5 {
            &["name_reference", "name_write"]
        } else {
            &[]
        }
    }
}

/// A method's owner is the class that contains it. The class spans its methods, so the writes
/// inside it are already in its declaration; writes are not a separate occurrence kind.
static OWNER: OwnerSyntax = OwnerSyntax {
    method_kinds: &["method"],
    owner_containers: &["class"],
    naming_container: None,
    owner_kinds: &[],
    receiver: "self",
    read: "attribute_access",
    write: None,
    literal: None,
};

/// The parser does not skip a UTF-8 BOM; the offset restores original bytes.
fn parse(text: &str) -> Result<(ast::Suite, usize), rustpython_parser::ParseError> {
    let offset = if text.starts_with('\u{feff}') { 3 } else { 0 };
    Ok((ast::Suite::parse(&text[offset..], "<source>")?, offset))
}

/// One pass collects declarations (when a source is given) and occurrences.
struct Walker<'a> {
    g6: bool,
    v5: bool,
    v6: bool,
    source: Option<&'a Source>,
    bytes: &'a [u8],
    offset: usize,
    /// Enclosing declarations: name and whether it is a class.
    scope: Vec<(String, bool)>,
    symbols: Vec<Symbol>,
    chunks: Vec<Chunk>,
    found: Vec<Occurrence>,
    /// Fields already declared, by qualified name: a field is declared once per class.
    fields: std::collections::BTreeSet<String>,
    /// v5: names bound in each enclosing function (and lambda), and those it declares `global`
    /// or `nonlocal`. A local name is never a use of a module declaration.
    locals: Vec<Bound>,
}

/// Names a function binds anywhere in its body, Python's rule for what is local to it.
#[derive(Default)]
struct Bound {
    local: std::collections::BTreeSet<String>,
    global: std::collections::BTreeSet<String>,
}

impl<'a> Walker<'a> {
    fn new(python: &Python, source: Option<&'a Source>, bytes: &'a [u8], offset: usize) -> Self {
        Self { g6: python.g6, v5: python.v5, v6: python.v6, source, bytes, offset, scope: vec![], symbols: vec![], chunks: vec![], found: vec![],
            fields: Default::default(), locals: vec![] }
    }

    fn is_local(&self, name: &str) -> bool {
        self.locals.iter().any(|b| b.local.contains(name))
    }

    /// v5: a name read as a value (`Depends(get_db)`, `with _lock:`), unless it is local.
    fn reference(&mut self, name: &ast::ExprName) {
        if !self.is_local(&name.id) {
            self.push("name_reference", name.range, Some(name.id.to_string()));
        }
    }

    /// v5: an assignment target that writes a module name: the name itself when the function
    /// declares it `global`, or the container it indexes (`_cache[key] = v`). Returns whether
    /// the target was handled; otherwise it is visited as before.
    fn write(&mut self, target: &ast::Expr) -> bool {
        match target {
            ast::Expr::Name(n) if self.locals.last().is_some_and(|b| b.global.contains(n.id.as_str())) => {
                self.push("name_write", n.range, Some(n.id.to_string()));
                true
            }
            ast::Expr::Subscript(s) => match s.value.as_ref() {
                ast::Expr::Name(n) if !self.is_local(&n.id) => {
                    self.push("name_write", n.range, Some(n.id.to_string()));
                    self.visit_expr(*s.slice.clone());
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// v5: a name assigned at module level is a module variable, declared by its first
    /// assignment. Returns whether the target declares one.
    fn module_variable(&mut self, target: &ast::Expr, statement: TextRange) -> bool {
        let ast::Expr::Name(n) = target else { return false };
        if !self.v5 || !self.scope.is_empty() {
            return false;
        }
        if self.fields.insert(n.id.to_string()) {
            self.declare_in(&[], "variable", &n.id, statement, &[]);
            return true;
        }
        false
    }

    /// v5: the iterable of a comprehension's first `for`.
    fn iterable(&mut self, generators: &[ast::Comprehension]) {
        if let Some(first) = generators.first() {
            self.push("loop_iterable", first.iter.range(), None);
        }
    }

    /// v5: a loop or `with` block, labelled by its head and spanning its body.
    fn block(&mut self, kind: &'static str, range: TextRange, head_end: TextRange) {
        let (start, end) = self.span(range);
        let (_, label_end) = self.span(head_end);
        self.found.push(Occurrence { kind, start, end, candidate: None, label_end: Some(label_end) });
    }

    /// The names a function body binds: parameters, assignment and loop targets, `with ... as`,
    /// `except ... as`, imports and nested definitions. Nested bodies are their own scopes.
    fn bound(arguments: &ast::Arguments, body: &[ast::Stmt]) -> Bound {
        let mut binder = Binder::default();
        let params = arguments.posonlyargs.iter().chain(&arguments.args).chain(&arguments.kwonlyargs).map(|a| &a.def);
        for arg in params.chain(arguments.vararg.as_deref()).chain(arguments.kwarg.as_deref()) {
            binder.0.local.insert(arg.arg.to_string());
        }
        for stmt in body {
            binder.visit_stmt(stmt.clone());
        }
        let Binder(mut bound) = binder;
        bound.local.retain(|n| !bound.global.contains(n));
        bound
    }

    fn span(&self, range: TextRange) -> (usize, usize) {
        (usize::from(range.start()) + self.offset, usize::from(range.end()) + self.offset)
    }

    /// The parsed range starts at `def`/`class`; decorators belong to the declaration too.
    fn declare(&mut self, kind: &str, name: &str, range: TextRange, decorators: &[ast::Expr]) {
        let scope: Vec<String> = self.scope.iter().map(|(n, _)| n.clone()).collect();
        self.declare_in(&scope, kind, name, range, decorators);
    }

    fn declare_in(&mut self, scope: &[String], kind: &str, name: &str, range: TextRange, decorators: &[ast::Expr]) {
        let Some(source) = self.source else { return };
        let (mut start, end) = self.span(range);
        if let Some(first) = decorators.first() {
            let (at, _) = self.span(first.range());
            start = self.bytes[..at].iter().rposition(|&b| b == b'@').unwrap_or(start).min(start);
        }
        let id = format!("{}:{}:{}:{}", source.id, kind, start, end);
        let mut qualified: Vec<&str> = scope.iter().map(String::as_str).collect();
        qualified.push(name);
        // A naming convention observed (pytest), not evidence that a test ran.
        let is_test = name.starts_with("test") || name.starts_with("Test")
            || self.scope.iter().any(|(n, class)| *class && n.starts_with("Test"));
        self.symbols.push(Symbol {
            id: id.clone(),
            source_id: source.id.clone(),
            name: name.into(),
            qualified_name: qualified.join("."),
            kind: kind.into(),
            is_test,
            chunk_id: id.clone(),
        });
        self.chunks.push(Chunk {
            id,
            source_id: source.id.clone(),
            start_byte: start,
            end_byte: end,
            start_line: crate::line_at(self.bytes, start),
            end_line: crate::line_at(self.bytes, end.saturating_sub(1)),
            sha256: hash(&self.bytes[start..end]),
        });
    }

    fn function(&mut self, name: &str, range: TextRange, decorators: &[ast::Expr], returns: Option<&ast::Expr>) {
        let kind = if self.scope.last().is_some_and(|(_, class)| *class) { "method" } else { "function" };
        self.declare(kind, name, range, decorators);
        if let Some(returns) = returns {
            self.push("declared_return", returns.range(), None);
            self.annotation(returns);
        }
    }

    /// G6 fields: names assigned in a class body, and `self.<name>` assigned in its `__init__`.
    /// Returns whether the target declares a field, so it is not observed as an access of itself.
    fn field(&mut self, target: &ast::Expr, statement: TextRange) -> bool {
        if !self.g6 {
            return false;
        }
        let depth = self.scope.len();
        let in_class_body = self.scope.last().is_some_and(|(_, class)| *class);
        let in_init = depth >= 2 && self.scope[depth - 1].0 == "__init__" && !self.scope[depth - 1].1 && self.scope[depth - 2].1;
        let name = match target {
            ast::Expr::Name(n) if in_class_body => n.id.to_string(),
            ast::Expr::Attribute(a) if in_init && matches!(a.value.as_ref(), ast::Expr::Name(n) if n.id.as_str() == "self") => {
                a.attr.to_string()
            }
            _ => return false,
        };
        let class: Vec<String> = self.scope[..if in_init { depth - 1 } else { depth }].iter().map(|(n, _)| n.clone()).collect();
        // A later assignment of the same field is a write to it, not a second declaration.
        if self.fields.insert(format!("{}.{}", class.join("."), name)) {
            self.declare_in(&class, "field", &name, statement, &[]);
            return true;
        }
        false
    }

    fn push(&mut self, kind: &'static str, range: TextRange, candidate: Option<String>) {
        let (start, end) = self.span(range);
        self.found.push(Occurrence { kind, start, end, candidate, label_end: None });
    }

    /// Names written in an annotation or base list; candidates, never resolved types.
    fn annotation(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Name(n) => self.push("type_reference", n.range, Some(n.id.to_string())),
            ast::Expr::Attribute(a) => self.push("type_reference", a.range, Some(a.attr.to_string())),
            ast::Expr::Subscript(s) => {
                self.annotation(&s.value);
                self.annotation(&s.slice);
            }
            ast::Expr::Tuple(t) => t.elts.iter().for_each(|e| self.annotation(e)),
            ast::Expr::List(l) => l.elts.iter().for_each(|e| self.annotation(e)),
            ast::Expr::BinOp(b) => {
                self.annotation(&b.left);
                self.annotation(&b.right);
            }
            // Forward reference written as a string, e.g. "Node".
            ast::Expr::Constant(c) => {
                if let ast::Constant::Str(text) = &c.value {
                    if !text.is_empty() && text.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
                        self.push("type_reference", c.range, Some(text.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    /// Names bound by assignment targets; attributes and subscripts are not local bindings.
    fn binding(&mut self, target: &ast::Expr) {
        match target {
            ast::Expr::Name(n) => self.push("local_binding", n.range, None),
            ast::Expr::Tuple(t) => t.elts.iter().for_each(|e| self.binding(e)),
            ast::Expr::List(l) => l.elts.iter().for_each(|e| self.binding(e)),
            ast::Expr::Starred(s) => self.binding(&s.value),
            _ => {}
        }
    }

    /// v6: a function's parameters in order, labelled by the name: `parameter` when a call may
    /// fill it by position, `keyword_parameter` after `*` or `*args`. `*args` and `**kwargs`
    /// themselves are not recorded.
    fn parameters(&mut self, arguments: &ast::Arguments) {
        let positional = arguments.posonlyargs.iter().chain(&arguments.args).map(|a| ("parameter", &a.def));
        let keyword = arguments.kwonlyargs.iter().map(|a| ("keyword_parameter", &a.def));
        for (kind, arg) in positional.chain(keyword) {
            let (start, end) = self.span(arg.range);
            self.found.push(Occurrence { kind, start, end, candidate: None, label_end: Some(start + arg.arg.len()) });
        }
    }

    /// v6: a plain name bound to a list inside a function (`x = [...]`, a list comprehension,
    /// `list(...)` or `sorted(...)`). Searching it with `in` walks the list.
    fn list_binding(&mut self, target: &ast::Expr, value: &ast::Expr) {
        let ast::Expr::Name(n) = target else { return };
        let is_list = match value {
            ast::Expr::List(_) | ast::Expr::ListComp(_) => true,
            ast::Expr::Call(c) => matches!(c.func.as_ref(), ast::Expr::Name(f) if matches!(f.id.as_str(), "list" | "sorted")),
            _ => false,
        };
        if self.v6 && is_list && !self.locals.is_empty() {
            self.push("list_binding", n.range, None);
        }
    }
}

impl Visitor for Walker<'_> {
    fn visit_stmt_function_def(&mut self, node: ast::StmtFunctionDef) {
        self.function(&node.name, node.range, &node.decorator_list, node.returns.as_deref());
        self.scope.push((node.name.to_string(), false));
        if self.v5 {
            self.locals.push(Self::bound(&node.args, &node.body));
        }
        if self.v6 {
            self.parameters(&node.args);
        }
        self.generic_visit_stmt_function_def(node);
        if self.v5 {
            self.locals.pop();
        }
        self.scope.pop();
    }

    fn visit_stmt_async_function_def(&mut self, node: ast::StmtAsyncFunctionDef) {
        self.function(&node.name, node.range, &node.decorator_list, node.returns.as_deref());
        self.scope.push((node.name.to_string(), false));
        if self.v5 {
            self.locals.push(Self::bound(&node.args, &node.body));
        }
        if self.v6 {
            self.parameters(&node.args);
        }
        self.generic_visit_stmt_async_function_def(node);
        if self.v5 {
            self.locals.pop();
        }
        self.scope.pop();
    }

    fn visit_expr_lambda(&mut self, node: ast::ExprLambda) {
        if !self.v5 {
            return self.generic_visit_expr_lambda(node);
        }
        self.locals.push(Self::bound(&node.args, &[]));
        self.visit_expr(*node.body);
        self.locals.pop();
    }

    fn visit_expr_name(&mut self, node: ast::ExprName) {
        // Stores are bindings or `write`s, seen from their statement.
        if self.v5 && matches!(node.ctx, ast::ExprContext::Load) {
            self.reference(&node);
        }
    }

    fn visit_stmt_aug_assign(&mut self, node: ast::StmtAugAssign) {
        if !self.v5 || !self.write(&node.target) {
            self.visit_expr(*node.target);
        }
        self.visit_expr(*node.value);
    }

    // The iterable of a `for` runs once, before the loop: it is its own occurrence.
    fn visit_stmt_for(&mut self, node: ast::StmtFor) {
        self.binding(&node.target);
        if self.v5 {
            self.block("loop", node.range, node.iter.range());
            self.push("loop_iterable", node.iter.range(), None);
        }
        self.generic_visit_stmt_for(node);
    }

    fn visit_stmt_async_for(&mut self, node: ast::StmtAsyncFor) {
        self.binding(&node.target);
        if self.v5 {
            self.block("loop", node.range, node.iter.range());
            self.push("loop_iterable", node.iter.range(), None);
        }
        self.generic_visit_stmt_async_for(node);
    }

    fn visit_stmt_while(&mut self, node: ast::StmtWhile) {
        if self.v5 {
            self.block("loop", node.range, node.test.range());
        }
        self.generic_visit_stmt_while(node);
    }

    fn visit_stmt_with(&mut self, node: ast::StmtWith) {
        if let (true, Some(last)) = (self.v5, node.items.last()) {
            self.block("with_block", node.range, last.optional_vars.as_deref().unwrap_or(&last.context_expr).range());
        }
        self.generic_visit_stmt_with(node);
    }

    fn visit_stmt_async_with(&mut self, node: ast::StmtAsyncWith) {
        if let (true, Some(last)) = (self.v5, node.items.last()) {
            self.block("with_block", node.range, last.optional_vars.as_deref().unwrap_or(&last.context_expr).range());
        }
        self.generic_visit_stmt_async_with(node);
    }

    // A comprehension is a loop written as an expression; its whole text is its label. The
    // iterable of its first `for` runs once, outside the loop.
    fn visit_expr_list_comp(&mut self, node: ast::ExprListComp) {
        if self.v5 {
            self.block("loop", node.range, node.range);
            self.iterable(&node.generators);
        }
        self.generic_visit_expr_list_comp(node);
    }

    fn visit_expr_set_comp(&mut self, node: ast::ExprSetComp) {
        if self.v5 {
            self.block("loop", node.range, node.range);
            self.iterable(&node.generators);
        }
        self.generic_visit_expr_set_comp(node);
    }

    fn visit_expr_dict_comp(&mut self, node: ast::ExprDictComp) {
        if self.v5 {
            self.block("loop", node.range, node.range);
            self.iterable(&node.generators);
        }
        self.generic_visit_expr_dict_comp(node);
    }

    fn visit_expr_generator_exp(&mut self, node: ast::ExprGeneratorExp) {
        if self.v5 {
            self.block("loop", node.range, node.range);
            self.iterable(&node.generators);
        }
        self.generic_visit_expr_generator_exp(node);
    }

    fn visit_stmt_class_def(&mut self, node: ast::StmtClassDef) {
        self.declare("class", &node.name, node.range, &node.decorator_list);
        // G6: bases are their own occurrence kind, so inheritance is told apart from annotations.
        for base in &node.bases {
            match (self.g6, base) {
                (true, ast::Expr::Name(n)) => self.push("base_class", n.range, Some(n.id.to_string())),
                (true, ast::Expr::Attribute(a)) => self.push("base_class", a.range, Some(a.attr.to_string())),
                _ => self.annotation(base),
            }
        }
        self.scope.push((node.name.to_string(), true));
        self.generic_visit_stmt_class_def(node);
        self.scope.pop();
    }

    fn visit_stmt_assign(&mut self, node: ast::StmtAssign) {
        node.targets.iter().for_each(|t| self.binding(t));
        node.targets.iter().for_each(|t| self.list_binding(t, &node.value));
        if !self.g6 {
            return self.generic_visit_stmt_assign(node);
        }
        // A target that declares a field is not also an access to that field. With several
        // targets (`x = y = 0`) each field spans its own name, so their records stay distinct.
        let several = node.targets.len() > 1;
        for target in node.targets {
            let range = if several { target.range() } else { node.range };
            if !self.field(&target, range) && !self.module_variable(&target, range) && !(self.v5 && self.write(&target)) {
                self.visit_expr(target);
            }
        }
        self.visit_expr(*node.value);
    }

    fn visit_stmt_ann_assign(&mut self, node: ast::StmtAnnAssign) {
        self.binding(&node.target);
        if let Some(value) = &node.value {
            self.list_binding(&node.target, value);
        }
        self.annotation(&node.annotation);
        if !self.g6 {
            return self.generic_visit_stmt_ann_assign(node);
        }
        // The annotation is already a type reference; only target and value are visited.
        if !self.field(&node.target, node.range) && !self.module_variable(&node.target, node.range) && !(self.v5 && self.write(&node.target)) {
            self.visit_expr(*node.target);
        }
        if let Some(value) = node.value {
            self.visit_expr(*value);
        }
    }

    // v6: `x in name` and `x not in name`, a membership test on a plain name. Whether that walks
    // a list is the inventory's question, from how the name is bound.
    fn visit_expr_compare(&mut self, node: ast::ExprCompare) {
        let membership = matches!(node.ops.as_slice(), [ast::CmpOp::In | ast::CmpOp::NotIn])
            && matches!(node.comparators.as_slice(), [ast::Expr::Name(_)]);
        if self.v6 && membership {
            self.push("membership", node.range, None);
        }
        self.generic_visit_expr_compare(node);
    }

    fn visit_expr_attribute(&mut self, node: ast::ExprAttribute) {
        if self.g6 {
            self.push("attribute_access", node.range, Some(node.attr.to_string()));
        }
        self.generic_visit_expr_attribute(node);
    }

    fn visit_expr_call(&mut self, node: ast::ExprCall) {
        // Keep receiver and arguments; no receiver type or target is inferred.
        if !self.g6 {
            match node.func.as_ref() {
                ast::Expr::Attribute(_) => self.push("method_call_observed", node.range, None),
                func => self.push("call_observed", func.range(), None),
            }
            return self.generic_visit_expr_call(node);
        }
        // G6: the written callee name is a candidate. A called attribute is a call, not also an
        // attribute access, so only its receiver is visited.
        match *node.func {
            ast::Expr::Attribute(a) => {
                self.push("method_call_observed", node.range, Some(a.attr.to_string()));
                self.visit_expr(*a.value);
            }
            func => {
                let name = match &func {
                    ast::Expr::Name(n) => Some(n.id.to_string()),
                    _ => None,
                };
                self.push("call_observed", func.range(), name);
                // v5: a called name is the call; it is not also a name used as a value.
                if !(self.v5 && matches!(func, ast::Expr::Name(_))) {
                    self.visit_expr(func);
                }
            }
        }
        node.args.into_iter().for_each(|a| self.visit_expr(a));
        node.keywords.into_iter().for_each(|k| self.visit_keyword(k));
    }

    // The generated visitor leaves these empty; without them nested calls and defs are missed.
    fn visit_arguments(&mut self, node: ast::Arguments) {
        let with_defaults = node.posonlyargs.into_iter().chain(node.args).chain(node.kwonlyargs);
        for arg in with_defaults {
            self.visit_arg(arg.def);
            if let Some(default) = arg.default {
                self.visit_expr(*default);
            }
        }
        for arg in node.vararg.into_iter().chain(node.kwarg) {
            self.visit_arg(*arg);
        }
    }

    fn visit_arg(&mut self, node: ast::Arg) {
        if let Some(annotation) = &node.annotation {
            self.annotation(annotation);
        }
    }

    fn visit_keyword(&mut self, node: ast::Keyword) {
        // v5: `User(email=...)` fills a field named `email`, by the written name.
        if let (true, Some(arg)) = (self.v5, &node.arg) {
            self.push("keyword_argument", node.range, Some(arg.to_string()));
        }
        self.visit_expr(node.value);
    }

    fn visit_withitem(&mut self, node: ast::WithItem) {
        if let Some(target) = &node.optional_vars {
            self.binding(target);
        }
        self.visit_expr(node.context_expr);
    }

    fn visit_match_case(&mut self, node: ast::MatchCase) {
        if let Some(guard) = node.guard {
            self.visit_expr(*guard);
        }
        for stmt in node.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_comprehension(&mut self, node: ast::Comprehension) {
        self.binding(&node.target);
        self.visit_expr(node.iter);
        for condition in node.ifs {
            self.visit_expr(condition);
        }
    }
}

/// Collects what a function body binds (see `Walker::bound`). Nested functions, classes and
/// lambdas bind only their own name here; their bodies are scopes of their own.
#[derive(Default)]
struct Binder(Bound);

impl Visitor for Binder {
    fn visit_stmt_function_def(&mut self, node: ast::StmtFunctionDef) {
        self.0.local.insert(node.name.to_string());
    }

    fn visit_stmt_async_function_def(&mut self, node: ast::StmtAsyncFunctionDef) {
        self.0.local.insert(node.name.to_string());
    }

    fn visit_stmt_class_def(&mut self, node: ast::StmtClassDef) {
        self.0.local.insert(node.name.to_string());
    }

    fn visit_stmt_global(&mut self, node: ast::StmtGlobal) {
        self.0.global.extend(node.names.iter().map(|n| n.to_string()));
    }

    fn visit_stmt_nonlocal(&mut self, node: ast::StmtNonlocal) {
        self.0.global.extend(node.names.iter().map(|n| n.to_string()));
    }

    fn visit_expr_lambda(&mut self, _node: ast::ExprLambda) {}

    fn visit_expr_name(&mut self, node: ast::ExprName) {
        if !matches!(node.ctx, ast::ExprContext::Load) {
            self.0.local.insert(node.id.to_string());
        }
    }

    fn visit_alias(&mut self, node: ast::Alias) {
        // `import a.b` binds `a`; `import a.b as c` and `from a import b as c` bind `c`.
        let name = node.asname.as_ref().map_or_else(|| node.name.split('.').next().unwrap_or_default().to_owned(), |a| a.to_string());
        self.0.local.insert(name);
    }

    fn visit_excepthandler_except_handler(&mut self, node: ast::ExceptHandlerExceptHandler) {
        if let Some(name) = &node.name {
            self.0.local.insert(name.to_string());
        }
        self.generic_visit_excepthandler_except_handler(node);
    }

    fn visit_withitem(&mut self, node: ast::WithItem) {
        if let Some(target) = node.optional_vars {
            self.visit_expr(*target);
        }
    }

    fn visit_comprehension(&mut self, node: ast::Comprehension) {
        self.visit_expr(node.target);
    }

    fn visit_match_case(&mut self, node: ast::MatchCase) {
        for stmt in node.body {
            self.visit_stmt(stmt);
        }
    }
}
