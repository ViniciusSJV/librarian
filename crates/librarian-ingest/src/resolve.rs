//! The one rule that settles a call: which declaration a written call reaches, and how. Used by
//! "what does it call?" (`Graph::calls_from`), by the operation inventory, and, through the
//! links of `calls_from`, by the envelope and the owner context.
//!
//! A layer over the v2 graph: it reads nodes and edges and adds none. The graph's edges stay
//! name candidates, never compiler resolution; `type` and `trait_default` use the receiver's
//! written type (a `receiver_type` occurrence, or a `T::f` path), which the Rust extractor
//! records and the Python one does not.
//!
//! Step 2.2: the form of the call admits only some candidates. A call with a receiver (`x.m()`)
//! never reaches a free function, except a Python `module.f()` whose receiver is bound to a module
//! of the project, which reaches only that module's functions; a receiver bound to an import from
//! outside the project reaches nothing in it. A call without one (`f()`) never reaches a method,
//! except, in a class body or a method, a method of that class. What the graph does not keep (a
//! file's imports and local bindings, the receiver's written type when it is not the project's)
//! is read from the edition's preserved bytes, through `CallSyntax::facts`.
//!
//! Precedence: chosen > unique > import > receiver > type > trait_default > trait > homonym,
//! always over the admitted candidates. `unique` comes before `type` so that its label stays when
//! the type agrees; when the receiver's written type rules the one candidate out, the call has no
//! candidate.
use crate::graph::v2::{CallSyntax, Edge, Graph};
use crate::graph::Evidence;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Resolutions a consumer may take as the one declaration a call reaches. `calls_from` reports
/// them only with a single target.
pub const SETTLED: [&str; 5] = ["unique", "import", "receiver", "type", "trait_default"];

/// Step 2.3a (DEC-39): kinds of declaration that a settled call builds instead of calling: a
/// Python class, a Rust tuple struct or variant. Its link is `instantiates`.
pub const INSTANTIATED: [&str; 3] = ["class", "struct", "variant"];

/// Written types that reach their contents' methods (auto-deref): a receiver of one of these is
/// not ruled out by its type (step 2.2, conservative).
const WRAPPERS: [&str; 3] = ["Box", "Rc", "Arc"];

/// What the rule reads from one file's preserved bytes besides the graph (step 2.2).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CallFacts {
    /// Name scopes (module, class, function, lambda, comprehension): what each binds to an import
    /// and what it binds locally. Empty for a language without imports of modules.
    pub scopes: Vec<NameScope>,
    /// A method call receiver's written type, by the receiver's span: the type by its last
    /// segment, and whether it is written as a reference (`&T`, `&mut T`).
    pub receiver_types: BTreeMap<(usize, usize), (String, bool)>,
    /// Step 2.3a (Python): each class the file declares, by its qualified name, with what the rule
    /// needs to know of it.
    pub classes: BTreeMap<String, ClassFacts>,
}

/// Step 2.3a: what a Python class declares that decides whether its type rules a candidate out.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ClassFacts {
    /// A base is written (other than `object`): a method may come from it (inheritance is 2.4).
    pub based: bool,
    /// It defines `__getattr__` or `__getattribute__`: any name may be answered.
    pub getattr: bool,
    /// One of its bases is `Protocol`: its methods are signatures, and the body that runs is the
    /// passed object's.
    pub protocol: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NameScope {
    pub start: usize,
    pub end: usize,
    /// A class body: its names are not seen from the functions inside it.
    pub class: bool,
    pub imports: BTreeMap<String, Import>,
    /// Names bound otherwise: parameters, assignments, `for`/`with`/`except` targets, nested
    /// definitions. They win over an import of the same name in the same scope.
    pub locals: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Import {
    /// `import a.b` (bound as `a`, and `a.b` as written) or `import a.b as c`: the dotted module.
    Module(String),
    /// `from p import n`: the package, with its leading dots, and the name.
    From(String, String),
}

/// Reads a file's facts by its path, from the edition the graph was built from.
#[derive(Clone)]
pub struct FileFacts(pub Arc<dyn Fn(&str) -> Option<CallFacts> + Send + Sync>);

impl std::fmt::Debug for FileFacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FileFacts")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// The person picked it among the candidates.
    Chosen,
    /// The written name matches only this declaration, among those the call's form reaches.
    Unique,
    /// `module.f()`: the receiver is bound to a module of the project, and only this function of
    /// that module has the name.
    Import,
    /// `self.m()` (the language's receiver, nothing else before the name) and only this
    /// candidate shares the caller's container.
    Receiver,
    /// A method of the receiver's written type.
    Type,
    /// The receiver's type has no such method; the default of a trait it implements.
    TraitDefault,
    /// No written type, and every candidate belongs to one trait: all its implementations.
    Trait,
}

impl How {
    pub fn as_str(self) -> &'static str {
        match self {
            How::Chosen => "chosen",
            How::Unique => "unique",
            How::Import => "import",
            How::Receiver => "receiver",
            How::Type => "type",
            How::TraitDefault => "trait_default",
            How::Trait => "trait",
        }
    }
    /// Where the resolution comes from: the graph alone (`graph_edge`), the receiver's written
    /// type (`type`), or the file's imports (`import`).
    pub fn basis(self) -> &'static str {
        match self {
            How::Type | How::TraitDefault => "type",
            How::Import => "import",
            _ => "graph_edge",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution<'g> {
    /// Followed: one target, or several (`trait`, or a type that names several impls).
    Linked(How, Vec<&'g str>),
    /// Several candidate edges and nothing settles them.
    Homonym(Vec<&'g str>),
    /// More same-named declarations than the profile links, and no written type settles it.
    AboveLimit(usize),
    NoCandidate,
}

/// What a receiver bound by an import is, for the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Module {
    /// A module file of the project, and the receiver is exactly the bound name.
    File(String),
    /// From outside the project: nothing in it is reached.
    External,
}

pub struct Resolver<'g> {
    graph: &'g Graph,
    receivers: BTreeMap<String, Option<&'static str>>,
    candidates: BTreeMap<&'g str, Vec<&'g Edge>>,
    parent: BTreeMap<&'g str, &'g str>,
    methods_by_name: BTreeMap<&'g str, Vec<&'g str>>,
    /// Step 2.3a: Python classes by their written name (the label's last segment).
    classes_by_name: BTreeMap<&'g str, Vec<&'g str>>,
    /// receiver_type occurrences by where they start: (end, occurrence).
    receiver_types: BTreeMap<(&'g str, usize), Vec<(usize, &'g str)>>,
    call_evidence: BTreeMap<&'g str, &'g Evidence>,
    /// (type, trait) of every `impl Trait for Type`.
    impl_traits: Vec<(String, String)>,
    /// The file that declares each symbol.
    declared_in: BTreeMap<&'g str, &'g str>,
    /// Every source path of the graph.
    paths: BTreeSet<&'g str>,
    facts: Option<crate::resolve::FileFacts>,
    cache: RefCell<BTreeMap<String, Option<Arc<CallFacts>>>>,
}

impl<'g> Resolver<'g> {
    pub fn new(graph: &'g Graph, syntax: &CallSyntax) -> Self {
        let mut r = Resolver {
            graph,
            receivers: syntax.languages.iter().map(|(l, (_, receiver))| (l.clone(), *receiver)).collect(),
            candidates: BTreeMap::new(),
            parent: BTreeMap::new(),
            methods_by_name: BTreeMap::new(),
            classes_by_name: BTreeMap::new(),
            receiver_types: BTreeMap::new(),
            call_evidence: BTreeMap::new(),
            impl_traits: vec![],
            declared_in: BTreeMap::new(),
            paths: BTreeSet::new(),
            facts: syntax.facts.clone(),
            cache: RefCell::new(BTreeMap::new()),
        };
        for e in &graph.edges {
            match e.kind.as_str() {
                "name_candidate" => r.candidates.entry(&e.from).or_default().push(e),
                "contains" => {
                    r.parent.insert(&e.to, &e.from);
                }
                "declares" => {
                    r.declared_in.insert(&e.to, &e.evidence.path);
                }
                "observes" => {
                    r.call_evidence.insert(&e.to, &e.evidence);
                    if graph.nodes.get(&e.to).is_some_and(|n| n.kind == "receiver_type") {
                        r.receiver_types.entry((e.evidence.path.as_str(), e.evidence.start_byte)).or_default().push((e.evidence.end_byte, &e.to));
                    }
                }
                _ => {}
            }
        }
        for (id, node) in &graph.nodes {
            if node.category == "source" {
                r.paths.insert(&node.label);
            }
            if node.category != "symbol" {
                continue;
            }
            if matches!(node.kind.as_str(), "method" | "trait_method") {
                // The written name is the label's last segment: after `::` (`impl S::go`) or `.`
                // (`S.go`); neither separator is part of a name (step 2.3a-0; until then, `::` only).
                r.methods_by_name.entry(node.label.rsplit([':', '.']).next().unwrap_or_default()).or_default().push(id);
            }
            if node.kind == "class" {
                r.classes_by_name.entry(node.label.rsplit('.').next().unwrap_or_default()).or_default().push(id);
            }
            if node.kind == "impl" {
                if let (Some(ty), Some(tr)) = impl_parts(&node.label) {
                    r.impl_traits.push((ty, tr));
                }
            }
        }
        r
    }

    /// The candidate edges the graph drew for a call occurrence.
    pub fn candidates(&self, occurrence: &str) -> &[&'g Edge] {
        self.candidates.get(occurrence).map_or(&[], Vec::as_slice)
    }

    /// The candidate edges the call's form reaches (step 2.2): the ones the rule decides among.
    pub fn admitted(&self, caller: &str, occurrence: &str) -> Vec<&'g Edge> {
        self.admit(caller, occurrence).0
    }

    fn label(&self, id: &str) -> &'g str {
        &self.graph.nodes[id].label
    }

    fn kind(&self, id: &str) -> &'g str {
        &self.graph.nodes[id].kind
    }

    fn container(&self, id: &str) -> Option<&'g str> {
        self.parent.get(id).copied()
    }

    fn file_facts(&self, path: &str) -> Option<Arc<CallFacts>> {
        let read = self.facts.as_ref()?;
        self.cache.borrow_mut().entry(path.to_owned()).or_insert_with(|| (read.0)(path).map(Arc::new)).clone()
    }

    /// The type and the trait an impl or a trait names, for a method in it.
    fn owner_of(&self, method: &str) -> Option<(Option<String>, Option<String>)> {
        let owner = self.container(method)?;
        match self.kind(owner) {
            "impl" => Some(impl_parts(self.label(owner))),
            "trait" => Some((None, Some(last_segment(self.label(owner))))),
            _ => None,
        }
    }

    /// The receiver_type occurrence of a method call's whole written receiver, if recorded: the
    /// one that ends where the call's written receiver ends (`self` and `self.transform` both
    /// start where `self.transform.inverse()` does).
    fn receiver_type_node(&self, occurrence: &str) -> Option<(&'g str, (usize, usize))> {
        let evidence = self.call_evidence.get(occurrence)?;
        let end = evidence.start_byte + written_receiver(self.label(occurrence))?.len();
        let found = self.receiver_types.get(&(evidence.path.as_str(), evidence.start_byte))?;
        found.iter().find(|(e, _)| *e == end).map(|(_, id)| (*id, (evidence.start_byte, end)))
    }

    /// The written type of a call's receiver: `T::f` and `Self::f` by their path, `x.f()` by the
    /// receiver_type the extractor recorded, when it names a declaration of the project.
    pub fn receiver_types(&self, caller: &str, occurrence: &str) -> Vec<String> {
        let text = self.label(occurrence);
        if self.kind(occurrence) == "call_observed" {
            let Some((receiver, _)) = call_parts(text) else { return vec![] };
            if receiver == "Self" {
                return self.container(caller).filter(|c| self.kind(c) == "impl").and_then(|c| impl_parts(self.label(c)).0).into_iter().collect();
            }
            return if receiver.starts_with(|c: char| c.is_uppercase()) { vec![receiver] } else { vec![] };
        }
        let Some((written, _)) = self.receiver_type_node(occurrence) else { return vec![] };
        let names: BTreeSet<String> = self.candidates(written).iter().map(|t| last_segment(self.label(&t.to))).collect();
        names.into_iter().collect()
    }

    /// Whether a method fits the receiver's written type: a method of the type's impl, the method
    /// of a trait the type implements, or of the trait the type is.
    fn fits(&self, method: &str, types: &[String]) -> bool {
        let Some((ty, tr)) = self.owner_of(method) else { return true };
        if ty.is_some_and(|t| types.contains(&t)) {
            return true;
        }
        tr.is_some_and(|tr| types.contains(&tr) || self.impl_traits.iter().any(|(t, i)| types.contains(t) && *i == tr))
    }

    /// Step 2.2: whether the receiver's written type rules out the call's one candidate. A type of
    /// the project rules out a method that does not fit it. A type from outside the project rules
    /// out only an inherent method of a type of the project, and not when the receiver is written
    /// as a reference or a `Box`, `Rc` or `Arc`, nor when the project implements a trait for it.
    fn type_rules_out(&self, caller: &str, occurrence: &str, target: &str) -> bool {
        if !matches!(self.kind(target), "method" | "trait_method") {
            return false;
        }
        // Step 2.3a: a Python class rules out a method of another class only when nothing can
        // bring the method to it (no base, no `__getattr__`, not a `Protocol`).
        if let Some(class) = self.python_class(caller, occurrence) {
            let facts = self.class_facts(class);
            return self.container(target) != Some(class) && !facts.based && !facts.getattr && !facts.protocol;
        }
        let types = self.receiver_types(caller, occurrence);
        if !types.is_empty() {
            return !self.fits(target, &types);
        }
        let Some((written, span)) = self.receiver_type_node(occurrence) else { return false };
        if !self.candidates(written).is_empty() {
            return false;
        }
        let Some(path) = self.call_evidence.get(occurrence).map(|e| e.path.as_str()) else { return false };
        let Some((name, by_reference)) = self.file_facts(path).and_then(|f| f.receiver_types.get(&span).cloned()) else { return false };
        if by_reference || WRAPPERS.contains(&name.as_str()) || self.impl_traits.iter().any(|(t, _)| *t == name) {
            return false;
        }
        matches!(self.owner_of(target), Some((Some(_), None)))
    }

    /// What the call's receiver is bound to by an import, if anything: the innermost scope that
    /// binds its first name decides, and a local binding wins over an import.
    fn module(&self, occurrence: &str, receiver: &str) -> Option<Module> {
        let evidence = self.call_evidence.get(occurrence)?;
        let facts = self.file_facts(&evidence.path)?;
        // The first name only: `TimestampSigner()` and `emails.x` start with a name.
        let head = receiver.split(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or_default();
        let at = evidence.start_byte;
        let mut scopes: Vec<&NameScope> = facts.scopes.iter().filter(|s| s.start <= at && at < s.end).collect();
        scopes.sort_by_key(|s| s.end - s.start);
        let import = scopes.iter().enumerate().filter(|(i, s)| *i == 0 || !s.class).find_map(|(_, s)| {
            if s.locals.contains(head) {
                Some(None)
            } else {
                s.imports.get(receiver).or_else(|| s.imports.get(head)).map(|i| Some((i, s.imports.contains_key(receiver))))
            }
        })??;
        let (import, exact) = import;
        let (dotted, whole) = match import {
            Import::Module(m) => (m.clone(), exact),
            Import::From(package, name) => {
                let module = format!("{package}{}{name}", if package.ends_with('.') { "" } else { "." });
                match self.module_file(&evidence.path, &module) {
                    Some(_) => (module, exact),
                    // `from p import Name`, not a module: a class or a function of the project, or
                    // a name from outside it.
                    None if self.project_module(&evidence.path, package) => return None,
                    None => return Some(Module::External),
                }
            }
        };
        match self.module_file(&evidence.path, &dotted) {
            Some(file) if whole => Some(Module::File(file)),
            // `module.Class.m()`: the class's method, by the usual rule.
            Some(_) => None,
            None if self.project_module(&evidence.path, &dotted) => None,
            None => Some(Module::External),
        }
    }

    /// Step 2.3a: what the innermost scope that binds `name` at `at` binds it to: an import, or a
    /// local binding (a class or an assignment of the file; `Some(None)`). None when no scope binds it.
    fn bound(&self, facts: &CallFacts, at: usize, name: &str) -> Option<Option<Import>> {
        let mut scopes: Vec<&NameScope> = facts.scopes.iter().filter(|s| s.start <= at && at < s.end).collect();
        scopes.sort_by_key(|s| s.end - s.start);
        scopes.iter().enumerate().filter(|(i, s)| *i == 0 || !s.class).find_map(|(_, s)| {
            if s.locals.contains(name) {
                Some(None)
            } else {
                s.imports.get(name).map(|i| Some(i.clone()))
            }
        })
    }

    /// Step 2.3a: the class a Python receiver's written type means, among the classes the graph
    /// matched by name: `self` is the caller's own class; a name is the file's own top-level class
    /// or the class a `from p import Name` brings (following up to three re-exports); `m.T` is
    /// the class `T` of the project module `m`. None when the type is not a class of the project,
    /// or is an alias (`as`, 2.4).
    fn python_class(&self, caller: &str, occurrence: &str) -> Option<&'g str> {
        let (written, span) = self.receiver_type_node(occurrence)?;
        let mut classes: Vec<&'g str> = self.candidates(written).iter().map(|e| e.to.as_str()).filter(|t| self.kind(t) == "class").collect();
        // Above the limit the graph kept only the count: the classes of the written name (17
        // `Command` in the Healthchecks), as the receiver rule does above the limit.
        if classes.is_empty() && self.graph.nodes[written].ambiguous_candidates.is_some() {
            let path = self.call_evidence.get(occurrence)?.path.as_str();
            let facts = self.file_facts(path)?;
            let (text, _) = facts.receiver_types.get(&span)?;
            let name = text.rsplit('.').next().unwrap_or_default();
            classes = self.classes_by_name.get(name).cloned().unwrap_or_default();
        }
        if classes.is_empty() {
            return None;
        }
        if self.graph.nodes[written].type_rule.as_deref() == Some("self") {
            let mut at = self.container(caller);
            while let Some(c) = at {
                if classes.contains(&c) {
                    return Some(c);
                }
                at = self.container(c);
            }
            return None;
        }
        let path = self.call_evidence.get(occurrence)?.path.as_str();
        let facts = self.file_facts(path)?;
        let (text, _) = facts.receiver_types.get(&span)?;
        let declared = |file: &str, name: &str| classes.iter().copied().find(|c| self.declared_in.get(c) == Some(&file) && self.label(c) == name);
        match text.split_once('.') {
            None => match self.bound(&facts, span.0, text)? {
                None => declared(path, text),
                Some(Import::From(package, name)) if name == *text => self.class_in_module(path, &package, &name, &declared, 0),
                Some(_) => None,
            },
            Some((head, _)) => {
                let (module, name) = text.rsplit_once('.')?;
                let dotted = match self.bound(&facts, span.0, head)?? {
                    Import::Module(m) if module == head || module.starts_with(&format!("{head}.")) => format!("{m}{}", &module[head.len()..]),
                    Import::From(package, imported) if module == head => format!("{package}{}{imported}", if package.ends_with('.') { "" } else { "." }),
                    _ => return None,
                };
                declared(&self.module_file(path, &dotted)?, name)
            }
        }
    }

    /// The class `name` of the module `package`, imported from `from`: declared there, or brought
    /// there by another `from .. import name` (a re-export), up to three steps.
    fn class_in_module(&self, from: &str, package: &str, name: &str, declared: &dyn Fn(&str, &str) -> Option<&'g str>, hops: usize) -> Option<&'g str> {
        let file = self.module_file(from, package)?;
        if let Some(c) = declared(&file, name) {
            return Some(c);
        }
        if hops >= 3 {
            return None;
        }
        let facts = self.file_facts(&file)?;
        let module_scope = facts.scopes.iter().max_by_key(|s| s.end - s.start)?;
        match module_scope.imports.get(name)? {
            Import::From(p, n) if n == name => self.class_in_module(&file, p, name, declared, hops + 1),
            _ => None,
        }
    }

    /// Step 2.3a: what the file of a Python class says about it.
    fn class_facts(&self, class: &str) -> ClassFacts {
        let Some(file) = self.declared_in.get(class) else { return ClassFacts::default() };
        self.file_facts(file).and_then(|f| f.classes.get(self.label(class)).cloned()).unwrap_or_default()
    }

    /// Step 2.3a: the declaration an instantiation runs: the `__init__` the class itself declares.
    /// None for a class without one (the inherited `__init__` is 2.4), a struct or a variant.
    pub fn instance_entry(&self, target: &str) -> Option<&'g str> {
        if self.kind(target) != "class" {
            return None;
        }
        self.methods_by_name.get("__init__")?.iter().copied().find(|m| self.container(m) == Some(target))
    }

    /// The source of a dotted module, from the importing file for a relative one: `a.b` is
    /// `a/b.py` or `a/b/__init__.py`, or the one source that ends so (a package under `src/`).
    fn module_file(&self, from: &str, dotted: &str) -> Option<String> {
        let base = module_base(from, dotted)?;
        let names = [format!("{base}.py"), format!("{base}/__init__.py")];
        if let Some(found) = names.iter().find(|n| self.paths.contains(n.as_str())) {
            return Some(found.clone());
        }
        if dotted.starts_with('.') {
            return None;
        }
        let ends: Vec<&&str> = self.paths.iter().filter(|p| names.iter().any(|n| p.ends_with(&format!("/{n}")))).collect();
        (ends.len() == 1).then(|| ends[0].to_string())
    }

    /// Whether a dotted name is a module or a package of the project.
    fn project_module(&self, from: &str, dotted: &str) -> bool {
        let Some(base) = module_base(from, dotted) else { return false };
        self.module_file(from, dotted).is_some() || self.paths.iter().any(|p| p.starts_with(&format!("{base}/")))
    }

    /// The candidates the call's form admits, and whether the receiver is a module of the project.
    fn admit(&self, caller: &str, occurrence: &str) -> (Vec<&'g Edge>, bool) {
        let edges = self.candidates(occurrence);
        let text = self.label(occurrence);
        if let Some(receiver) = written_receiver(text) {
            return match self.module(occurrence, receiver) {
                Some(Module::File(file)) => {
                    // Step 2.3a: and the module's classes (`forms.WebhookForm(..)` instantiates).
                    let own = edges
                        .iter()
                        .copied()
                        .filter(|e| matches!(self.kind(&e.to), "function" | "class") && self.declared_in.get(e.to.as_str()) == Some(&file.as_str()))
                        .collect();
                    (own, true)
                }
                Some(Module::External) => (vec![], false),
                None => (edges.iter().copied().filter(|e| self.kind(&e.to) != "function").collect(), false),
            };
        }
        if call_parts(text).is_some_and(|(receiver, _)| receiver.is_empty()) {
            // In a class body, or in a method's decorators, a plain name may be a function of the class.
            let class = [Some(caller), self.container(caller)].into_iter().flatten().find(|c| self.kind(c) == "class");
            let kept = |e: &&Edge| !matches!(self.kind(&e.to), "method" | "trait_method") || (class.is_some() && self.container(&e.to) == class);
            return (edges.iter().copied().filter(kept).collect(), false);
        }
        (edges.to_vec(), false)
    }

    /// The rule. `caller` is the symbol whose body writes the call; `chosen`, the candidate the
    /// person picked for this occurrence, if any.
    pub fn resolve(&self, caller: &str, occurrence: &str, chosen: Option<&str>) -> Resolution<'g> {
        let all = self.candidates(occurrence);
        let (edges, module) = self.admit(caller, occurrence);
        if let Some(e) = chosen.and_then(|c| edges.iter().find(|e| e.to == c)) {
            return Resolution::Linked(How::Chosen, vec![&e.to]);
        }
        let text = self.label(occurrence);
        let plain = call_parts(text).is_some_and(|(receiver, _)| receiver.is_empty());
        if edges.len() == 1 {
            let to = edges[0].to.as_str();
            if module && all.len() > 1 {
                return Resolution::Linked(How::Import, vec![to]);
            }
            if all.len() == 1 || plain || module {
                if self.type_rules_out(caller, occurrence, to) {
                    return Resolution::NoCandidate;
                }
                return Resolution::Linked(How::Unique, vec![to]);
            }
        }
        let receiver = self.receivers.get(&self.graph.nodes[caller].language).copied().flatten();
        let own = receiver.is_some_and(|r| written_receiver(text) == Some(r));
        let same: Vec<&'g str> = edges.iter().map(|e| e.to.as_str()).filter(|t| self.container(t) == self.container(caller)).collect();
        if own && same.len() == 1 && self.kind(same[0]) != "trait_method" {
            return Resolution::Linked(How::Receiver, same);
        }
        let name = call_parts(text).map(|(_, n)| n).unwrap_or_default();
        let methods = self.methods_by_name.get(name.as_str()).map_or(&[][..], Vec::as_slice);
        // Above the limit there is no candidate edge: `self.m()` by the declarations of the
        // caller's container (step 2.2; until 2.1 it fell to `type`).
        if own && all.is_empty() && self.graph.nodes[occurrence].ambiguous_candidates.is_some() {
            let same: Vec<&'g str> = methods.iter().copied().filter(|m| self.container(m) == self.container(caller) && self.kind(m) != "trait_method").collect();
            if same.len() == 1 {
                return Resolution::Linked(How::Receiver, same);
            }
        }
        // Step 2.3a, Python: the methods the receiver's class declares. Without one, the class rules
        // every candidate out, unless something may bring the method to it (a base, `__getattr__`);
        // a `Protocol` gives no link (its methods are signatures; the passed object's body runs).
        if let Some(class) = self.python_class(caller, occurrence) {
            let facts = self.class_facts(class);
            if !facts.protocol {
                let own: Vec<&'g str> = methods.iter().copied().filter(|m| self.container(m) == Some(class)).collect();
                if !own.is_empty() {
                    return Resolution::Linked(How::Type, own);
                }
                if !facts.based && !facts.getattr {
                    return Resolution::NoCandidate;
                }
            }
        }
        let types = self.receiver_types(caller, occurrence);
        if !types.is_empty() {
            // Step 2.3a, Rust: `T::V(..)` builds the variant `V` of the enum `T`.
            if self.kind(occurrence) == "call_observed" {
                let variants: Vec<&'g str> = edges
                    .iter()
                    .map(|e| e.to.as_str())
                    .filter(|t| self.kind(t) == "variant" && self.container(t).is_some_and(|e| types.contains(&last_segment(self.label(e)))))
                    .collect();
                if variants.len() == 1 {
                    return Resolution::Linked(How::Type, variants);
                }
            }
            let own: Vec<&'g str> =
                methods.iter().copied().filter(|m| self.owner_of(m).is_some_and(|(t, _)| t.is_some_and(|t| types.contains(&t)))).collect();
            if !own.is_empty() {
                return Resolution::Linked(How::Type, own);
            }
            let traits: BTreeSet<&str> = self.impl_traits.iter().filter(|(t, _)| types.contains(t)).map(|(_, tr)| tr.as_str()).collect();
            let defaults: Vec<&'g str> = methods
                .iter()
                .copied()
                .filter(|m| self.kind(m) == "trait_method" && self.owner_of(m).is_some_and(|(_, tr)| tr.is_some_and(|tr| traits.contains(tr.as_str()))))
                .collect();
            if !defaults.is_empty() {
                return Resolution::Linked(How::TraitDefault, defaults);
            }
        }
        let traits: BTreeSet<Option<String>> = edges.iter().map(|e| self.owner_of(&e.to).and_then(|(_, tr)| tr)).collect();
        if edges.len() > 1 && traits.len() == 1 && traits.iter().all(Option::is_some) {
            return Resolution::Linked(How::Trait, edges.iter().map(|e| e.to.as_str()).collect());
        }
        if edges.is_empty() {
            return match (all.is_empty(), self.graph.nodes[occurrence].ambiguous_candidates) {
                (true, Some(count)) => Resolution::AboveLimit(count),
                _ => Resolution::NoCandidate,
            };
        }
        Resolution::Homonym(edges.iter().map(|e| e.to.as_str()).collect())
    }
}

/// A dotted module as a path without extension, from the importing file for a relative one:
/// `a.b` -> `a/b`; `..c` from `p/q/r.py` -> `p/c`. None when the dots climb above the root.
fn module_base(from: &str, dotted: &str) -> Option<String> {
    let dots = dotted.chars().take_while(|c| *c == '.').count();
    let rest = &dotted[dots..];
    let mut parts: Vec<&str> = if dots == 0 { vec![] } else { from.split('/').collect() };
    if dots > 0 {
        parts.pop();
        for _ in 1..dots {
            parts.pop()?;
        }
    }
    parts.extend(rest.split('.').filter(|s| !s.is_empty()));
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// A written path's last segment without generic arguments or references: `ops::Mul` ->
/// `Mul`, `Matrix<4>` -> `Matrix`, `&'a Sphere` -> `Sphere`.
pub(crate) fn last_segment(text: &str) -> String {
    let text = text.split('<').next().unwrap_or_default();
    let last = text.rsplit("::").next().unwrap_or_default();
    last.rsplit(|c: char| c.is_whitespace() || c == '&').next().unwrap_or_default().to_owned()
}

/// The type and the trait of a Rust impl's name: `impl Intersectable for Sphere` ->
/// (`Sphere`, `Intersectable`), `impl Matrix<4>` -> (`Matrix`, none).
pub(crate) fn impl_parts(label: &str) -> (Option<String>, Option<String>) {
    let Some(rest) = label.strip_prefix("impl ") else { return (None, None) };
    match rest.split_once(" for ") {
        Some((tr, ty)) => (Some(last_segment(ty)), Some(last_segment(tr))),
        None => (Some(last_segment(rest)), None),
    }
}

/// A call's text without its last arguments and turbofish: `a.b::<T>(x)` -> `a.b`.
fn call_head(text: &str) -> Option<&str> {
    let mut head = text.trim_end();
    // A method call's text has its arguments: drop the last balanced parentheses.
    if head.ends_with(')') {
        head = &head[..matching_open(head, '(', ')')?];
    }
    // Turbofish: `collect::<Vec<_>>`.
    if head.ends_with('>') {
        head = head[..matching_open(head, '<', '>')?].trim_end_matches("::");
    }
    Some(head)
}

/// The written name of a call and the name written just before it, from the call's text:
/// `self.session.exec(s)` -> (`session`, `exec`), `File::open` -> (`File`, `open`),
/// `open` -> (``, `open`). None when the text ends in no name.
pub fn call_parts(text: &str) -> Option<(String, String)> {
    let head = call_head(text)?;
    let name = trailing_ident(head)?;
    let before = head[..head.len() - name.len()].trim_end();
    // A receiver that is itself a call's result or an index (`a.b(x).c()`) has no written
    // name: "()", never the "" of a call with no receiver.
    let receiver = match before.strip_suffix('.').or_else(|| before.strip_suffix("::")) {
        Some(rest) => trailing_ident(rest.trim_end()).unwrap_or("()"),
        None => "",
    };
    Some((receiver.to_owned(), name.to_owned()))
}

/// A method call's whole written receiver: `self.transform.inverse()` -> `self.transform`,
/// `self.f(x).g()` -> `self.f(x)`. None without a receiver.
pub fn written_receiver(text: &str) -> Option<&str> {
    let head = call_head(text)?;
    let name = trailing_ident(head)?;
    let receiver = head[..head.len() - name.len()].trim_end().strip_suffix('.')?.trim_end();
    (!receiver.is_empty()).then_some(receiver)
}

pub(crate) fn matching_open(text: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in text.char_indices().rev() {
        if c == close {
            depth += 1;
        } else if c == open {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

pub(crate) fn trailing_ident(text: &str) -> Option<&str> {
    let start = text.char_indices().rev().take_while(|(_, c)| c.is_alphanumeric() || *c == '_').last().map(|(i, _)| i)?;
    Some(&text[start..])
}
