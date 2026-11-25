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
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Resolutions a consumer may take as the one declaration a call reaches. `calls_from` reports
/// them only with a single target.
pub const SETTLED: [&str; 6] = ["unique", "import", "scope", "receiver", "type", "trait_default"];

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
    /// Step 2.3b (Python; DEC-40): the byte spans of the decorator expressions the file writes, at
    /// any depth. A call inside one runs when the `def` or `class` runs, not when the decorated
    /// declaration is called.
    pub decorators: Vec<(usize, usize)>,
    /// Step 2.3c (DEC-41): a receiver bound to a call's result (`x = f(..)`, `f(..).m()`,
    /// `self.x = f(..)`; `let x = T::f(..)`, `let x = f(..)`), by the receiver's span: the span of
    /// each call occurrence that binds it, as the extractor records the call.
    pub bound_calls: BTreeMap<(usize, usize), Vec<(usize, usize)>>,
    /// Step 2.3c: what each written return type names, by the span of its `declared_return`.
    pub returns: BTreeMap<(usize, usize), Returned>,
    /// Step 2.3c (Python): a receiver `a.x` whose base has a written type: the base's written type,
    /// where it is written, and the field's name. The field is read in the file of the base's class.
    pub field_receivers: BTreeMap<(usize, usize), (String, usize, String)>,
    /// Step 2.4b (Python): the spans of the receivers that are a `classmethod`'s first parameter.
    pub cls_receivers: BTreeSet<(usize, usize)>,
    /// Step 2.4b (Python): what each function or method is, by where its declaration ends.
    pub methods: BTreeMap<usize, MethodFacts>,
    /// Step 2.4a (Rust): the file's modules; the first is the file itself.
    pub modules: Vec<RustModule>,
    /// Step 2.4a (Rust): the blocks that declare items, with what their own `use` and `type` bind:
    /// seen only from inside the block.
    pub blocks: Vec<RustModule>,
    /// Step 2.4a (Rust): the names each function binds as a variable (parameters, `let`, closure
    /// parameters, patterns), by the function's declaration span. A call `f()` with `f` one of them
    /// calls a value, not an item.
    pub variables: BTreeMap<(usize, usize), BTreeSet<String>>,
}

/// Step 2.3c: what a written return type gives a receiver bound to the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Returned {
    /// A named type: `T`, `m.T` (Python), `Self`; with whether it is written as a reference (Rust).
    Type { name: String, by_reference: bool },
    /// Nothing decided: a wrapper (`Option`, `Result`, `Box`, `Rc`, `Arc`, `Vec`), a union, a
    /// generic, a tuple, `impl Trait`, `None`.
    Undecided,
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
    /// Step 2.3c: the written type of each field (annotation, or `self.x = T(..)` / `self.x = p`
    /// with `p: T` in `__init__`), as written, when every write agrees.
    pub fields: BTreeMap<String, String>,
    /// Step 2.4b (DEC-43): the bases as written, without `object` and without a subscript; None for an
    /// expression that is not a name.
    pub bases: Vec<Option<String>>,
    /// Step 2.4b: where the class statement starts: its bases are named there.
    pub at: Option<usize>,
    /// Step 2.4b: the file declares two classes of this qualified name.
    pub repeated: bool,
    /// Step 2.4b: a class decorator, a `metaclass` or another keyword: it may bring methods.
    pub dynamic: bool,
    /// Step 2.4b: the names the class body binds to anything but a method declared in it.
    pub names: BTreeSet<String>,
    /// Step 2.4b: the attributes its methods write through `self` or `cls`.
    pub stored: BTreeSet<String>,
}

/// Step 2.4b (Python): what a function or method is.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MethodFacts {
    pub property: bool,
    pub is_static: bool,
    /// `@abstractmethod`, or a body that only raises `NotImplementedError`, `pass` or `...`.
    pub is_abstract: bool,
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
    /// Step 2.4a (DEC-42): every binding of each name in this scope, in the order of the text. The
    /// rule of a plain call (`f()`) and of a type alias reads these; `imports` and `locals` stay as
    /// the 2.2 left them, for the receiver's rule.
    pub bindings: BTreeMap<String, Vec<Binding>>,
    /// Step 2.4a: the scope has a `from x import *` (outside `TYPE_CHECKING`): a name it does not
    /// bind otherwise may come from there.
    pub star: bool,
    /// Step 2.4a: the parts of the span that run outside the scope (a `def`'s or `class`'s header, a
    /// lambda's defaults, a comprehension's first iterable): its names are not seen from them.
    pub outside: Vec<(usize, usize)>,
    /// Step 2.4a: the names the scope declares `global`: the module's, not an enclosing function's.
    pub globals: BTreeSet<String>,
}

impl NameScope {
    /// Step 2.4a: the names of the scope are seen from `at`, but for the class rule.
    fn around(&self, at: usize) -> bool {
        self.start <= at && at < self.end && !self.outside.iter().any(|&(start, end)| start <= at && at < end)
    }
}

/// Step 2.4a (DEC-42, Python): one binding of a name in a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// Where the binding statement starts.
    pub at: usize,
    /// For a `def` or a `class`: where its declaration ends (the graph's `end_byte`).
    pub end: usize,
    pub kind: Bound,
    /// Inside an `if`, `for`, `while`, `try` or `match` of its own scope, or made by another function
    /// (`global`, `nonlocal`): it may not run.
    pub conditional: bool,
    /// Inside `if TYPE_CHECKING:`: it exists for the type checker, not when the code runs.
    pub type_checking: bool,
}

/// Step 2.4a: what a binding binds a name to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bound {
    /// A `def` or a `class` of this file.
    Def,
    Import(Import),
    /// A parameter, an assignment, a `for`/`with`/`except` target: a variable.
    Local,
    /// `X = Name`, `X = m.Name` or `X: TypeAlias = Name`: another name for what `Name` names.
    Alias(String),
}

/// Step 2.4a (DEC-42, Rust): a module of a file (the file itself, or a `mod x { .. }`), by the span
/// of its declaration, with what its `use` items and type aliases bind.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RustModule {
    pub start: usize,
    pub end: usize,
    /// `use a::b::c` binds `c` to `[a, b, c]`; `use a::b as d` binds `d` to `[a, b]`;
    /// `use a::b::{self}` binds `b` to `[a, b]`.
    pub uses: BTreeMap<String, Vec<String>>,
    /// `use a::b::*`: `[a, b]`.
    pub globs: Vec<Vec<String>>,
    /// `type X = a::Y<..>;`: `X` to `Y` (the last segment).
    pub aliases: BTreeMap<String, String>,
    /// In a block: the spans of the items it declares (a module's are in the graph).
    pub items: Vec<(usize, usize)>,
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
    /// that module has the name. Step 2.4a (DEC-42): also a plain call `f()` whose name the
    /// caller's scope binds by an import (`from p import f`, `use a::f`, `use a::*`).
    Import,
    /// Step 2.4a (DEC-42): a plain call `f()` whose name the caller's scope binds to a definition of
    /// its own file: the module's (Python), a module's or a block's (Rust).
    Scope,
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
            How::Scope => "scope",
            How::Receiver => "receiver",
            How::Type => "type",
            How::TraitDefault => "trait_default",
            How::Trait => "trait",
        }
    }
    /// Where the resolution comes from: the graph alone (`graph_edge`), the receiver's written
    /// type (`type`), the file's imports (`import`), or a definition the caller's scope binds the
    /// name to (`scope`, step 2.4a).
    pub fn basis(self) -> &'static str {
        match self {
            How::Type | How::TraitDefault => "type",
            How::Import => "import",
            How::Scope => "scope",
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
    /// Step 2.4b (DEC-43): an abstract method's overrides in the project: which one runs is the
    /// object's. Never followed; the targets may have no candidate edge.
    Overrides(Vec<&'g str>),
    /// Step 2.4b (DEC-43): no class that may answer the receiver declares the name: no candidate, also
    /// above the limit.
    OutOfChain,
}

/// Step 2.4b: an element of a Python class's linearization.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Base<'g> {
    Class(&'g str),
    /// A base from outside the project, by its full name.
    Outside(String),
    /// A base the rule cannot name (an expression, a variable, bindings that disagree).
    Unknown,
}

/// Step 2.4b: what the search of a name along a linearization finds.
enum Search<'g> {
    /// The methods of that name the first project class that has it declares.
    Found(Vec<&'g str>, &'g str),
    /// A base from outside the project comes first.
    Outside,
    /// An unnamed base, `__getattr__`, `Protocol`, or a name bound to something that is not a method.
    Undecided,
    /// Every class is the project's, and none declares it.
    Nobody,
}

/// Step 2.4b: the static class of a Python receiver.
struct PyReceiver<'g> {
    class: &'g str,
    /// `super()`: the search starts after the class.
    parent: bool,
    /// A constructor: the object is exactly of the class.
    exact: bool,
}

/// Step 2.4b (DEC-43): what the receiver's static class says of the call.
enum Chain<'g> {
    /// The methods that answer.
    Methods(Vec<&'g str>),
    /// The method found is abstract: the overrides in the subclasses.
    Overrides(Vec<&'g str>),
    /// The candidates that may answer, by the chain (the rest is never the target).
    Only(BTreeSet<&'g str>),
    /// None may.
    Nobody,
    /// It says nothing.
    Unknown,
}

/// Step 2.4b: the linearization of every class of the project and the subclasses of each.
#[derive(Default)]
struct Hierarchy<'g> {
    mro: BTreeMap<&'g str, Option<Vec<Base<'g>>>>,
    subclasses: BTreeMap<&'g str, Vec<&'g str>>,
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
    /// Step 2.3c: each call occurrence by where it is written (path, start, end), with the symbol
    /// that observes it.
    call_at: BTreeMap<(&'g str, usize, usize), (&'g str, &'g str)>,
    /// Step 2.3c: where each declaration starts in its file.
    declared_at: BTreeMap<&'g str, usize>,
    /// Step 2.3c: the `declared_return` occurrence of each function or method.
    returns_of: BTreeMap<&'g str, &'g str>,
    /// Step 2.3c: free functions by their written name.
    functions_by_name: BTreeMap<&'g str, Vec<&'g str>>,
    /// Step 2.3c (Rust): the names of the project's types (struct, enum, union, trait, type alias).
    type_names: BTreeSet<String>,
    /// Step 2.3c: set while the call that binds a receiver is resolved. That resolution never
    /// reads a return, so a type comes from one level only, and no loop can start.
    inner: Cell<bool>,
    /// Step 2.3c: what the return rule gave each method call (None: it does not apply).
    by_return_cache: RefCell<BTreeMap<String, Option<ByReturn<'g>>>>,
    /// Step 2.4b: the linearizations read so far, and the whole project's, once needed.
    mro_cache: RefCell<BTreeMap<&'g str, Option<Vec<Base<'g>>>>>,
    hierarchy: std::cell::OnceCell<Hierarchy<'g>>,
    /// Step 2.4a: each declaration's span in its file.
    declared_span: BTreeMap<&'g str, (usize, usize)>,
    /// Step 2.4a (Rust): the declaration with a span of a file.
    declared_by_span: BTreeMap<(&'g str, (usize, usize)), &'g str>,
    /// Step 2.4a (Python): a function, method or class by its file and where its declaration ends.
    declared_end: BTreeMap<(&'g str, usize), Vec<&'g str>>,
    /// Step 2.4a (Rust): the items a plain name may name, by file, container (a `mod {}`, a
    /// function for an item of its block, none for the file's top level) and name.
    rust_items: BTreeMap<(&'g str, Option<&'g str>, String), Vec<&'g str>>,
}

/// Step 2.4a (DEC-42): what the caller's scope says a plain call's name is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Scoped<'g> {
    /// A declaration of the project: by a definition of the caller's file (`Scope`) or by an
    /// import (`Import`).
    Target(&'g str, How),
    /// Something that is not a declaration of the project: an import from outside it, a variable,
    /// a builtin. The call has no candidate.
    Nothing,
    /// The scope does not tell (a star import, a glob from outside the crate, conditional bindings
    /// that disagree, `TYPE_CHECKING`, a module called): the rule decides as before.
    Undecided,
}

/// Step 2.4a (Rust): where a `use` path's module is.
enum RustPlace<'g> {
    /// A module of the project: its file, its `mod {}` (none for the file's top) and its path.
    Project(&'g str, Option<&'g str>, Vec<String>),
    /// Another crate (`std`, a dependency).
    Outside,
    /// A path of the crate that names no module the rule finds.
    Unknown,
}

/// Step 2.3c (DEC-41): the type a receiver bound to a call takes from the written return of the
/// call's targets.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ByReturn<'g> {
    /// Python: a class of the project.
    Class(&'g str),
    /// Rust: a type of the project, by name.
    Named(String),
    /// Rust: a type from outside the project, and whether it is written as a reference.
    Outside(String, bool),
    /// No type, and nothing ruled out: a wrapper, no annotation, a name that is not a class, or
    /// targets that disagree.
    Undecided,
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
            call_at: BTreeMap::new(),
            declared_at: BTreeMap::new(),
            returns_of: BTreeMap::new(),
            functions_by_name: BTreeMap::new(),
            type_names: BTreeSet::new(),
            inner: Cell::new(false),
            by_return_cache: RefCell::new(BTreeMap::new()),
            mro_cache: RefCell::new(BTreeMap::new()),
            hierarchy: std::cell::OnceCell::new(),
            declared_span: BTreeMap::new(),
            declared_by_span: BTreeMap::new(),
            declared_end: BTreeMap::new(),
            rust_items: BTreeMap::new(),
        };
        for e in &graph.edges {
            match e.kind.as_str() {
                "name_candidate" => r.candidates.entry(&e.from).or_default().push(e),
                "contains" => {
                    r.parent.insert(&e.to, &e.from);
                }
                "declares" => {
                    r.declared_in.insert(&e.to, &e.evidence.path);
                    r.declared_at.insert(&e.to, e.evidence.start_byte);
                    let span = (e.evidence.start_byte, e.evidence.end_byte);
                    r.declared_span.insert(&e.to, span);
                    r.declared_by_span.insert((e.evidence.path.as_str(), span), &e.to);
                    if matches!(graph.nodes.get(&e.to).map(|n| n.kind.as_str()), Some("function" | "method" | "class")) {
                        r.declared_end.entry((e.evidence.path.as_str(), e.evidence.end_byte)).or_default().push(&e.to);
                    }
                }
                "observes" => {
                    r.call_evidence.insert(&e.to, &e.evidence);
                    match graph.nodes.get(&e.to).map(|n| n.kind.as_str()) {
                        Some("receiver_type") => {
                            r.receiver_types.entry((e.evidence.path.as_str(), e.evidence.start_byte)).or_default().push((e.evidence.end_byte, &e.to));
                        }
                        Some("call_observed" | "method_call_observed") => {
                            r.call_at.insert((e.evidence.path.as_str(), e.evidence.start_byte, e.evidence.end_byte), (&e.to, &e.from));
                        }
                        // The signature's, the first in the text: a closure's return type (Rust)
                        // is observed by the same function.
                        Some("declared_return") => {
                            let first = r.returns_of.get(e.from.as_str()).and_then(|o| r.call_evidence.get(o)).is_some_and(|ev| ev.start_byte < e.evidence.start_byte);
                            if !first {
                                r.returns_of.insert(&e.from, &e.to);
                            }
                        }
                        _ => {}
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
            if node.kind == "function" {
                r.functions_by_name.entry(node.label.rsplit([':', '.']).next().unwrap_or_default()).or_default().push(id);
            }
            if matches!(node.kind.as_str(), "struct" | "enum" | "union" | "trait" | "type") {
                r.type_names.insert(last_segment(&node.label));
            }
            if node.kind == "impl" {
                if let (Some(ty), Some(tr)) = impl_parts(&node.label) {
                    r.impl_traits.push((ty, tr));
                }
            }
        }
        // Step 2.4a: the Rust items by file, container and name. An item of an `impl` or a trait is
        // not named by a plain path.
        for (id, node) in &graph.nodes {
            if node.category != "symbol" || node.language != "rust" {
                continue;
            }
            if !matches!(node.kind.as_str(), "function" | "struct" | "enum" | "union" | "trait" | "type" | "const" | "static" | "module") {
                continue;
            }
            let container = r.parent.get(id.as_str()).copied();
            if container.is_some_and(|c| !matches!(graph.nodes[c].kind.as_str(), "module" | "function" | "method")) {
                continue;
            }
            let Some(path) = r.declared_in.get(id.as_str()).copied() else { continue };
            r.rust_items.entry((path, container, last_segment(&node.label))).or_default().push(id);
        }
        r
    }

    /// The candidate edges the graph drew for a call occurrence.
    pub fn candidates(&self, occurrence: &str) -> &[&'g Edge] {
        self.candidates.get(occurrence).map_or(&[], Vec::as_slice)
    }

    /// The candidate edges the call's form reaches (step 2.2) and, since the step 2.4b, the receiver's
    /// chain of bases allows: the ones the rule decides among.
    pub fn admitted(&self, caller: &str, occurrence: &str) -> Vec<&'g Edge> {
        let (edges, module) = self.admit(caller, occurrence);
        self.in_chain(edges, &self.chain(caller, occurrence, module))
    }

    /// Step 2.4b (DEC-43): the candidates a chain leaves. One that is not a method is not its matter.
    fn in_chain(&self, edges: Vec<&'g Edge>, chain: &Chain<'g>) -> Vec<&'g Edge> {
        match chain {
            Chain::Only(left) => edges.into_iter().filter(|e| self.kind(&e.to) != "method" || left.contains(e.to.as_str())).collect(),
            Chain::Nobody => edges.into_iter().filter(|e| self.kind(&e.to) != "method").collect(),
            _ => edges,
        }
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

    /// Step 2.3b (DEC-40): the call is written inside a decorator (Python), read from the file's
    /// text. It runs when the `def` or `class` runs, so its link is `decorates` and is not followed.
    pub fn decorates(&self, occurrence: &str) -> bool {
        let Some(evidence) = self.call_evidence.get(occurrence) else { return false };
        let Some(facts) = self.file_facts(&evidence.path) else { return false };
        facts.decorators.iter().any(|&(start, end)| start <= evidence.start_byte && evidence.end_byte <= end)
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
            if !receiver.starts_with(|c: char| c.is_uppercase()) {
                return vec![];
            }
            // Step 2.4a (N8): `use a::Cliente as C;` or `type C = Cliente;`: `C::new` is `Cliente`'s
            // (a `type` alias is itself a declaration of the project, so it is asked first).
            if self.graph.nodes[caller].language == "rust" {
                if let Some(named) = self.call_evidence.get(occurrence).and_then(|e| self.rust_alias(&e.path, e.start_byte, &receiver)) {
                    return vec![named];
                }
            }
            return vec![receiver];
        }
        // Step 2.3c (DEC-41): the written return of the call that binds the receiver, when it
        // applies, replaces the type the path names (`let x = T::f(..)`).
        if self.graph.nodes[caller].language != "python" {
            if let Some(found) = self.by_return(caller, occurrence) {
                return match found {
                    ByReturn::Named(name) => vec![name],
                    _ => vec![],
                };
            }
        }
        let Some((written, span)) = self.receiver_type_node(occurrence) else { return vec![] };
        let names: BTreeSet<String> = self.candidates(written).iter().map(|t| last_segment(self.label(&t.to))).collect();
        // Step 2.4a (N8, Rust): a written type no declaration has may be an alias of one.
        if names.is_empty() && self.graph.nodes[caller].language == "rust" {
            if let Some(path) = self.call_evidence.get(occurrence).map(|e| e.path.as_str()) {
                if let Some((text, _)) = self.file_facts(path).and_then(|f| f.receiver_types.get(&span).cloned()) {
                    if let Some(named) = self.rust_alias(path, span.0, &text) {
                        return vec![named];
                    }
                }
            }
        }
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
            // Step 2.4b: with a base too, by the chain (`chain`); this is the class without one.
            return self.container(target) != Some(class) && self.baseless(class) && !self.subclass_reaches(class, target);
        }
        let types = self.receiver_types(caller, occurrence);
        if !types.is_empty() {
            return !self.fits(target, &types);
        }
        // Step 2.3c: a return from outside the project follows the rule of a written type from
        // outside; any other return that applies decides nothing.
        if self.graph.nodes[caller].language != "python" {
            if let Some(found) = self.by_return(caller, occurrence) {
                return match found {
                    ByReturn::Outside(name, by_reference) => self.outside_rules_out(&name, by_reference, target),
                    _ => false,
                };
            }
        }
        let Some((written, span)) = self.receiver_type_node(occurrence) else { return false };
        if !self.candidates(written).is_empty() {
            return false;
        }
        let Some(path) = self.call_evidence.get(occurrence).map(|e| e.path.as_str()) else { return false };
        let Some((name, by_reference)) = self.file_facts(path).and_then(|f| f.receiver_types.get(&span).cloned()) else { return false };
        self.outside_rules_out(&name, by_reference, target)
    }

    /// Step 2.2: a type from outside the project rules out only an inherent method of a type of the
    /// project, and not through a reference, a `Box`, `Rc` or `Arc`, nor a trait the project
    /// implements for it.
    fn outside_rules_out(&self, name: &str, by_reference: bool, target: &str) -> bool {
        if by_reference || WRAPPERS.contains(&name) || self.impl_traits.iter().any(|(t, _)| t == name) {
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
        if let Some(class) = self.written_class(caller, occurrence) {
            return Some(class);
        }
        if self.graph.nodes[caller].language != "python" {
            return None;
        }
        // Step 2.3c (DEC-41): by the written return of the call that binds the receiver, or by a
        // field of a class of another file.
        match self.by_return(caller, occurrence) {
            Some(ByReturn::Class(class)) => Some(class),
            Some(_) => None,
            None => self.field_class(occurrence).or_else(|| self.property_class(occurrence)),
        }
    }

    /// Step 2.3a: the class the receiver's written type (its receiver_type) means.
    fn written_class(&self, caller: &str, occurrence: &str) -> Option<&'g str> {
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
            // Step 2.4a (N8): a name no class has may be an alias of one.
            if self.graph.nodes[caller].language != "python" {
                return None;
            }
            let path = self.call_evidence.get(occurrence)?.path.as_str();
            let facts = self.file_facts(path)?;
            let (text, _) = facts.receiver_types.get(&span)?;
            return self.class_named(path, span.0, text, &[]);
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
        self.class_named(path, span.0, text, &classes)
    }

    /// Step 2.3a: which of `classes` a type name written at `at` in `path` means: the file's own
    /// top-level class, the class a `from p import Name` brings, or the class `T` of the project
    /// module `m` in `m.T`. Step 2.3c: also for a return or a field written in another file.
    fn class_named(&self, path: &str, at: usize, text: &str, classes: &[&'g str]) -> Option<&'g str> {
        let facts = self.file_facts(path)?;
        let declared = |file: &str, name: &str| classes.iter().copied().find(|c| self.declared_in.get(c) == Some(&file) && self.label(c) == name);
        match text.split_once('.') {
            None => match self.bound(&facts, at, text)? {
                // Step 2.4a (N8): a local or module name may be an alias (`D = C`, `D: TypeAlias = C`).
                None => declared(path, text).or_else(|| self.python_alias_class(path, at, text)),
                Some(Import::From(package, name)) if name == *text => self.class_in_module(path, &package, &name, &declared, 0),
                // Step 2.4a (N8): `from p import C as D`.
                Some(Import::From(package, name)) => {
                    let named = self.classes_named(&name);
                    let declared = |file: &str, n: &str| named.iter().copied().find(|c| self.declared_in.get(c) == Some(&file) && self.label(c) == n);
                    self.class_in_module(path, &package, &name, &declared, 0)
                }
                Some(_) => None,
            },
            Some((head, _)) => {
                let (module, name) = text.rsplit_once('.')?;
                let dotted = match self.bound(&facts, at, head)?? {
                    Import::Module(m) if module == head || module.starts_with(&format!("{head}.")) => format!("{m}{}", &module[head.len()..]),
                    Import::From(package, imported) if module == head => format!("{package}{}{imported}", if package.ends_with('.') { "" } else { "." }),
                    _ => return None,
                };
                declared(&self.module_file(path, &dotted)?, name)
            }
        }
    }

    /// Step 2.3c: the classes of the project with a written name's last segment.
    fn classes_named(&self, text: &str) -> Vec<&'g str> {
        self.classes_by_name.get(text.rsplit('.').next().unwrap_or_default()).cloned().unwrap_or_default()
    }

    /// Step 2.3c (DEC-41): the type a method call's receiver takes from the written return of the
    /// call that binds it. None when the rule does not apply (no such binding, or the binding call
    /// has no candidate in the project: then what applied before applies), and while the binding
    /// call itself is being resolved (one level only).
    fn by_return(&self, _caller: &str, occurrence: &str) -> Option<ByReturn<'g>> {
        if self.inner.get() || self.kind(occurrence) != "method_call_observed" {
            return None;
        }
        if let Some(hit) = self.by_return_cache.borrow().get(occurrence) {
            return hit.clone();
        }
        self.inner.set(true);
        let found = self.binding_return(occurrence);
        self.inner.set(false);
        self.by_return_cache.borrow_mut().insert(occurrence.to_owned(), found.clone());
        found
    }

    /// Step 2.3c: the return every call that binds the receiver agrees on. (a) the binding call
    /// settles one target: its return; (b) it does not, but every candidate declares the same
    /// return: that one; (c) it has no candidate in the project: None; (d) the candidates, or the
    /// calls, disagree: Undecided.
    fn binding_return(&self, occurrence: &str) -> Option<ByReturn<'g>> {
        let evidence = self.call_evidence.get(occurrence)?;
        let end = evidence.start_byte + written_receiver(self.label(occurrence))?.len();
        let facts = self.file_facts(&evidence.path)?;
        let calls = facts.bound_calls.get(&(evidence.start_byte, end))?.clone();
        let mut agreed: Option<ByReturn<'g>> = None;
        for (start, end) in calls {
            let &(call, observer) = self.call_at.get(&(evidence.path.as_str(), start, end))?;
            for target in self.binding_targets(observer, call)? {
                let returned = self.returned_type(target)?;
                match &agreed {
                    None => agreed = Some(returned),
                    Some(a) if *a == returned => {}
                    Some(_) => return Some(ByReturn::Undecided),
                }
            }
        }
        agreed
    }

    /// Step 2.3c: the declarations the call that binds a receiver may reach, by the rule without
    /// the return and without a person's choice: its one target, its several targets, or its
    /// admitted homonyms; above the limit, the declarations of the name (for `T::f`, those of
    /// `T`). None when it has no candidate in the project.
    fn binding_targets(&self, observer: &str, call: &str) -> Option<Vec<&'g str>> {
        let text = self.label(call);
        let path: Vec<String> = if self.kind(call) == "call_observed" { self.receiver_types(observer, call) } else { vec![] };
        let of_path = |found: Vec<&'g str>| -> Vec<&'g str> {
            if path.is_empty() {
                return found;
            }
            found.into_iter().filter(|t| self.owner_of(t).is_some_and(|(ty, tr)| ty.is_some_and(|t| path.contains(&t)) || tr.is_some_and(|t| path.contains(&t)))).collect()
        };
        let targets = match self.resolve(observer, call, None) {
            Resolution::Linked(_, targets) => targets,
            Resolution::Homonym(targets) => of_path(targets),
            Resolution::AboveLimit(_) => {
                let name = call_parts(text)?.1;
                let by_name = if !path.is_empty() || written_receiver(text).is_some() { &self.methods_by_name } else { &self.functions_by_name };
                of_path(by_name.get(name.as_str()).cloned().unwrap_or_default())
            }
            Resolution::Overrides(targets) => targets,
            Resolution::NoCandidate | Resolution::OutOfChain => vec![],
        };
        (!targets.is_empty()).then_some(targets)
    }

    /// Step 2.3c: what a declaration's written return gives. None for a declaration that is built,
    /// not called (a class, a struct, a variant: the 2.3a decides).
    fn returned_type(&self, target: &str) -> Option<ByReturn<'g>> {
        if !matches!(self.kind(target), "function" | "method" | "trait_method") {
            return None;
        }
        let Some(occurrence) = self.returns_of.get(target) else { return Some(ByReturn::Undecided) };
        let evidence = self.call_evidence.get(occurrence)?;
        let returned = self.file_facts(&evidence.path).and_then(|f| f.returns.get(&(evidence.start_byte, evidence.end_byte)).cloned());
        let Some(Returned::Type { name, by_reference }) = returned else { return Some(ByReturn::Undecided) };
        if self.graph.nodes[target].language == "python" {
            if name == "Self" {
                return Some(self.container(target).filter(|c| self.kind(c) == "class").map_or(ByReturn::Undecided, ByReturn::Class));
            }
            let classes = self.classes_named(&name);
            return Some(self.class_named(&evidence.path, evidence.start_byte, &name, &classes).map_or(ByReturn::Undecided, ByReturn::Class));
        }
        let name = if name == "Self" {
            match self.owner_of(target) {
                Some((Some(ty), _)) => ty,
                _ => return Some(ByReturn::Undecided),
            }
        } else {
            name
        };
        Some(if self.type_names.contains(&name) { ByReturn::Named(name) } else { ByReturn::Outside(name, by_reference) })
    }

    /// Step 2.3c (Python): the class of a receiver `a.x` whose base `a` has a written type, by the
    /// field `x` as the class's own file writes it.
    fn field_class(&self, occurrence: &str) -> Option<&'g str> {
        // One level: not while the call that binds another receiver is resolved.
        if self.inner.get() {
            return None;
        }
        let evidence = self.call_evidence.get(occurrence)?;
        let end = evidence.start_byte + written_receiver(self.label(occurrence))?.len();
        let facts = self.file_facts(&evidence.path)?;
        let (base, at, field) = facts.field_receivers.get(&(evidence.start_byte, end))?;
        let owner = self.class_named(&evidence.path, *at, base, &self.classes_named(base))?;
        let file = *self.declared_in.get(owner)?;
        let written = self.file_facts(file)?.classes.get(self.label(owner))?.fields.get(field)?.clone();
        let at = self.declared_at.get(owner).copied().unwrap_or_default();
        self.class_named(file, at, &written, &self.classes_named(&written))
    }

    /// Step 2.3c: the rule that gave a receiver's type, for a link settled by it (`type_rule` of
    /// `/api/calls`): `retorno` or `campo` when the 2.3c decided, otherwise the rule the
    /// receiver_type recorded, or `caminho` for `T::f(..)`.
    pub fn type_rule(&self, caller: &str, occurrence: &str) -> Option<String> {
        if self.kind(occurrence) == "call_observed" {
            return call_parts(self.label(occurrence)).filter(|(r, _)| !r.is_empty()).map(|_| "caminho".to_owned());
        }
        if let Some(rule) = self.receiver_type_node(occurrence).and_then(|(n, _)| self.graph.nodes[n].type_rule.clone()) {
            if self.written_class(caller, occurrence).is_some() {
                return Some(rule);
            }
        }
        match self.by_return(caller, occurrence) {
            Some(ByReturn::Undecided) => {}
            Some(_) => return Some("retorno".to_owned()),
            None => {}
        }
        if self.graph.nodes[caller].language == "python" {
            if self.field_class(occurrence).is_some() {
                return Some("campo".to_owned());
            }
            // Step 2.4b.
            if self.property_class(occurrence).is_some() {
                return Some("propriedade".to_owned());
            }
            if self.super_class(caller, occurrence).is_some() {
                return Some("super".to_owned());
            }
            if self.cls_class(caller, occurrence).is_some() {
                return Some("cls".to_owned());
            }
        }
        self.receiver_type_node(occurrence).and_then(|(n, _)| self.graph.nodes[n].type_rule.clone())
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
        match module_scope.imports.get(name) {
            Some(Import::From(p, n)) if n == name => self.class_in_module(&file, p, name, declared, hops + 1),
            // Step 2.4a (N8): a re-export under another name (`from x import C as D`).
            Some(Import::From(p, n)) => {
                let named = self.classes_named(n);
                let declared = |f: &str, x: &str| named.iter().copied().find(|c| self.declared_in.get(c) == Some(&f) && self.label(c) == x);
                self.class_in_module(&file, p, n, &declared, hops + 1)
            }
            Some(_) => None,
            // Step 2.4a (N8): an alias the module writes (`D = C`).
            None => match self.python_in_module(&file, name, hops + 1) {
                Scoped::Target(t, _) if self.kind(t) == "class" => Some(t),
                _ => None,
            },
        }
    }

    /// Step 2.3a: what the file of a Python class says about it.
    fn class_facts(&self, class: &str) -> ClassFacts {
        let Some(file) = self.declared_in.get(class) else { return ClassFacts::default() };
        self.file_facts(file).and_then(|f| f.classes.get(self.label(class)).cloned()).unwrap_or_default()
    }

    /// Step 2.3a: the declaration an instantiation runs: the `__init__` the class itself declares.
    /// Step 2.4b: or the one it inherits. None for a class without one, a struct or a variant.
    pub fn instance_entry(&self, target: &str) -> Option<&'g str> {
        if self.kind(target) != "class" {
            return None;
        }
        let own = self.methods_by_name.get("__init__")?.iter().copied().find(|m| self.container(m) == Some(target));
        if own.is_some() || self.graph.nodes[target].language != "python" {
            return own;
        }
        // Step 2.4b (DEC-43): the inherited one, by the same search; not when a class decorator, a
        // `metaclass` or a `__new__` may change what the construction runs.
        let target = self.graph.nodes.get_key_value(target)?.0.as_str();
        let Search::Found(found, at) = self.search(target, "__init__", true) else { return None };
        let changed = |c: &str| self.class_facts(c).dynamic || self.methods_by_name.get("__new__").is_some_and(|ms| ms.iter().any(|m| self.container(m) == Some(c)));
        for base in self.mro(target)? {
            match base {
                Base::Class(c) if changed(c) => return None,
                Base::Class(c) if c == at => break,
                _ => {}
            }
        }
        (found.len() == 1).then(|| found[0])
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

    /// Step 2.4b: the class of the method, or of a field of it, that writes a call.
    fn enclosing_method_class(&self, caller: &str) -> Option<(&'g str, &'g str)> {
        let mut at = self.graph.nodes.get_key_value(caller).map(|(id, _)| id.as_str());
        while let Some(symbol) = at {
            let container = self.container(symbol);
            if self.kind(symbol) == "method" {
                return container.filter(|c| self.kind(c) == "class").map(|c| (symbol, c));
            }
            at = container;
        }
        None
    }

    /// Step 2.4b: what a function or method is, read from its file.
    fn method_facts(&self, method: &str) -> MethodFacts {
        let (Some(path), Some((_, end))) = (self.declared_in.get(method), self.declared_span.get(method)) else { return MethodFacts::default() };
        self.file_facts(path).and_then(|f| f.methods.get(end).copied()).unwrap_or_default()
    }

    /// Step 2.4b: the class a `super().m()` or `super(C, self).m()` starts after: the enclosing method's.
    fn super_class(&self, caller: &str, occurrence: &str) -> Option<&'g str> {
        if self.graph.nodes[caller].language != "python" || self.kind(occurrence) != "method_call_observed" {
            return None;
        }
        let inside = written_receiver(self.label(occurrence))?.strip_prefix("super(")?.strip_suffix(')')?.trim();
        let (method, class) = self.enclosing_method_class(caller)?;
        if self.method_facts(method).is_static {
            return None;
        }
        let named = match inside.split_once(',') {
            None => inside.is_empty(),
            Some((c, _)) => Some(c.trim()) == self.label(class).rsplit('.').next(),
        };
        named.then_some(class)
    }

    /// Step 2.4b: the class of `cls.m()`, with `cls` a `classmethod`'s first parameter.
    fn cls_class(&self, caller: &str, occurrence: &str) -> Option<&'g str> {
        if self.graph.nodes[caller].language != "python" || self.kind(occurrence) != "method_call_observed" {
            return None;
        }
        let evidence = self.call_evidence.get(occurrence)?;
        let end = evidence.start_byte + written_receiver(self.label(occurrence))?.len();
        self.file_facts(&evidence.path)?.cls_receivers.contains(&(evidence.start_byte, end)).then(|| self.enclosing_method_class(caller))?.map(|(_, c)| c)
    }

    /// Step 2.4b: the class of a receiver `a.p` whose base `a` has a written type, when `p` is a
    /// `@property` with one `def` and an annotated return, found along the chain of `a`'s class.
    fn property_class(&self, occurrence: &str) -> Option<&'g str> {
        // One level, as the return of a call (DEC-41).
        if self.inner.get() {
            return None;
        }
        let evidence = self.call_evidence.get(occurrence)?;
        let end = evidence.start_byte + written_receiver(self.label(occurrence))?.len();
        let facts = self.file_facts(&evidence.path)?;
        let (base, at, name) = facts.field_receivers.get(&(evidence.start_byte, end))?;
        let owner = self.class_named(&evidence.path, *at, base, &self.classes_named(base))?;
        let Search::Found(found, _) = self.search(owner, name, false) else { return None };
        if found.len() != 1 || !self.method_facts(found[0]).property {
            return None;
        }
        match self.returned_type(found[0]) {
            Some(ByReturn::Class(class)) => Some(class),
            _ => None,
        }
    }

    /// Step 2.4b: the static class of a Python receiver, by any rule.
    fn python_receiver(&self, caller: &str, occurrence: &str) -> Option<PyReceiver<'g>> {
        if self.graph.nodes[caller].language != "python" || self.kind(occurrence) != "method_call_observed" {
            return None;
        }
        if let Some(class) = self.super_class(caller, occurrence) {
            return Some(PyReceiver { class, parent: true, exact: false });
        }
        if let Some(class) = self.cls_class(caller, occurrence) {
            return Some(PyReceiver { class, parent: false, exact: false });
        }
        let class = self.python_class(caller, occurrence)?;
        let constructed = self.receiver_type_node(occurrence).is_some_and(|(n, _)| self.graph.nodes[n].type_rule.as_deref() == Some("construtor"));
        Some(PyReceiver { class, parent: false, exact: constructed && self.written_class(caller, occurrence) == Some(class) })
    }

    /// Step 2.4b: what a base written at `at` in `path` is, named as the language binds it there
    /// (DEC-42).
    fn base(&self, path: &str, at: usize, written: &str) -> Base<'g> {
        let scoped = match written.contains('.') {
            true => self.python_alias(path, at, written, 0),
            false => self.python_name(path, at, written, 0),
        };
        match scoped {
            Scoped::Target(t, _) if self.kind(t) == "class" => Base::Class(t),
            Scoped::Nothing => {
                let head = written.split('.').next().unwrap_or_default();
                match self.file_facts(path).and_then(|f| self.bound(&f, at, head)) {
                    Some(Some(Import::Module(m))) => Base::Outside(format!("{m}{}", &written[head.len()..])),
                    Some(Some(Import::From(p, n))) => Base::Outside(format!("{p}.{n}{}", &written[head.len()..])),
                    // A builtin.
                    None => Base::Outside(written.to_owned()),
                    Some(None) => Base::Unknown,
                }
            }
            _ => Base::Unknown,
        }
    }

    /// Step 2.4b: the C3 linearization of a Python class, the language's own, over its written bases;
    /// a base from outside the project is one opaque element. None when the order is inconsistent or
    /// the class's file says nothing of it.
    fn mro(&self, class: &'g str) -> Option<Vec<Base<'g>>> {
        if let Some(hit) = self.mro_cache.borrow().get(class) {
            return hit.clone();
        }
        // A class that reaches itself has no order.
        self.mro_cache.borrow_mut().insert(class, None);
        let found = self.linearize(class);
        self.mro_cache.borrow_mut().insert(class, found.clone());
        found
    }

    fn linearize(&self, class: &'g str) -> Option<Vec<Base<'g>>> {
        let path = *self.declared_in.get(class)?;
        let facts = self.file_facts(path)?.classes.get(self.label(class))?.clone();
        if facts.repeated {
            return None;
        }
        let at = facts.at?;
        let bases: Vec<Base<'g>> = facts.bases.iter().map(|b| b.as_deref().map_or(Base::Unknown, |b| self.base(path, at, b))).collect();
        let mut lists: Vec<Vec<Base<'g>>> = vec![];
        for base in &bases {
            lists.push(match base {
                Base::Class(c) => self.mro(c)?,
                other => vec![other.clone()],
            });
        }
        lists.push(bases);
        let mut out = vec![Base::Class(class)];
        loop {
            lists.retain(|l| !l.is_empty());
            if lists.is_empty() {
                return Some(out);
            }
            // An unnamed base is never the same as another one.
            let head = lists.iter().map(|l| &l[0]).find(|h| !lists.iter().any(|l| l[1..].contains(h)))?.clone();
            for l in &mut lists {
                if l[0] == head {
                    l.remove(0);
                }
            }
            out.push(head);
        }
    }

    /// Step 2.4b: the methods of a name that a class itself declares.
    fn own_methods(&self, class: &str, name: &str) -> Vec<&'g str> {
        self.methods_by_name.get(name).map_or(&[][..], Vec::as_slice).iter().copied().filter(|m| self.kind(m) == "method" && self.container(m) == Some(class)).collect()
    }

    /// Step 2.4b (DEC-43): a name along a class's linearization (after the class itself, for `super()`
    /// and an inherited `__init__`).
    fn search(&self, class: &'g str, name: &str, after: bool) -> Search<'g> {
        if !after && !self.class_facts(class).protocol {
            let own = self.own_methods(class, name);
            if !own.is_empty() {
                return Search::Found(own, class);
            }
        }
        let Some(mro) = self.mro(class) else { return Search::Undecided };
        for base in mro.iter().skip(usize::from(after)) {
            let c = match base {
                Base::Class(c) => *c,
                Base::Outside(_) => return Search::Outside,
                Base::Unknown => return Search::Undecided,
            };
            let facts = self.class_facts(c);
            if facts.protocol {
                return Search::Undecided;
            }
            let found = self.own_methods(c, name);
            if !found.is_empty() {
                // A subclass's or a base's method may have written an attribute of that name.
                let stored = !after && mro.iter().any(|b| matches!(b, Base::Class(k) if self.class_facts(k).stored.contains(name)));
                return if stored { Search::Undecided } else { Search::Found(found, c) };
            }
            if facts.names.contains(name) || facts.stored.contains(name) || facts.getattr {
                return Search::Undecided;
            }
        }
        Search::Nobody
    }

    /// Step 2.4b: the linearization of every Python class of the project, and the subclasses of each.
    fn hierarchy(&self) -> &Hierarchy<'g> {
        self.hierarchy.get_or_init(|| {
            let mut h = Hierarchy::default();
            for class in self.classes_by_name.values().flatten().copied().filter(|c| self.graph.nodes[*c].language == "python") {
                let mro = self.mro(class);
                for base in mro.iter().flatten().skip(1) {
                    if let Base::Class(b) = base {
                        h.subclasses.entry(b).or_default().push(class);
                    }
                }
                h.mro.insert(class, mro);
            }
            h
        })
    }

    fn subclasses(&self, class: &str) -> &[&'g str] {
        self.hierarchy().subclasses.get(class).map_or(&[], Vec::as_slice)
    }

    /// Step 2.3a: a class without a base, `__getattr__` or `Protocol` rules a candidate out.
    fn baseless(&self, class: &str) -> bool {
        let facts = self.class_facts(class);
        !facts.based && !facts.getattr && !facts.protocol
    }

    /// Step 2.4b (DEC-43, changes the DEC-39): a subclass of the project may bring the method (a mixin).
    fn subclass_reaches(&self, class: &str, method: &str) -> bool {
        let owner = self.container(method);
        self.subclasses(class).iter().any(|s| match self.hierarchy().mro.get(s) {
            Some(Some(mro)) => mro.iter().any(|b| matches!(b, Base::Unknown) || matches!(b, Base::Class(c) if Some(*c) == owner)),
            _ => true,
        })
    }

    /// Step 2.4b (DEC-43): the classes whose method may answer: the receiver's chain and that of its
    /// subclasses; in a `super()`, only what comes after the class. None when one of them may bring
    /// methods the rule does not see.
    fn chain_classes(&self, receiver: &PyReceiver<'g>, name: &str) -> Option<BTreeSet<&'g str>> {
        let mut out = BTreeSet::new();
        for start in std::iter::once(receiver.class).chain(self.subclasses(receiver.class).iter().copied()) {
            let mro = self.mro(start)?;
            let from = if receiver.parent { mro.iter().position(|b| *b == Base::Class(receiver.class))? + 1 } else { 0 };
            for base in &mro[from..] {
                match base {
                    Base::Class(c) => {
                        out.insert(*c);
                    }
                    Base::Outside(_) => {}
                    Base::Unknown => return None,
                }
            }
        }
        let opens = |c: &&str| {
            let f = self.class_facts(c);
            f.dynamic || f.protocol || f.names.contains(name) || (!receiver.parent && (f.getattr || f.stored.contains(name)))
        };
        (!out.iter().any(opens)).then_some(out)
    }

    /// Step 2.4b (DEC-43): what the receiver's static class says of a call, by its chain of bases.
    fn chain(&self, caller: &str, occurrence: &str, module: bool) -> Chain<'g> {
        if module {
            return Chain::Unknown;
        }
        let Some(receiver) = self.python_receiver(caller, occurrence) else { return Chain::Unknown };
        let Some((_, name)) = call_parts(self.label(occurrence)) else { return Chain::Unknown };
        let class = receiver.class;
        match self.search(class, &name, receiver.parent) {
            Search::Found(found, at) => {
                // `super()` decides only if every subclass goes on to the same method.
                if receiver.parent && self.subclasses(class).iter().any(|s| !self.continues_to(s, class, &name, at)) {
                    return self.left_by_chain(&receiver, occurrence, &name);
                }
                // An abstract method that subclasses override: the object's body runs.
                if !receiver.parent && !receiver.exact && found.len() == 1 && self.method_facts(found[0]).is_abstract {
                    let overrides: Vec<&'g str> =
                        self.subclasses(class).iter().flat_map(|s| self.own_methods(s, &name)).filter(|m| !self.method_facts(m).is_abstract).collect();
                    if !overrides.is_empty() {
                        return Chain::Overrides(overrides);
                    }
                }
                Chain::Methods(found)
            }
            _ => self.left_by_chain(&receiver, occurrence, &name),
        }
    }

    /// Step 2.4b: in the linearization of `subclass`, the first class after `class` that has the name
    /// is `at`, before any base from outside.
    fn continues_to(&self, subclass: &'g str, class: &str, name: &str, at: &str) -> bool {
        let Some(mro) = self.mro(subclass) else { return false };
        let Some(start) = mro.iter().position(|b| matches!(b, Base::Class(c) if *c == class)) else { return false };
        for base in &mro[start + 1..] {
            let Base::Class(c) = base else { return false };
            if *c == at {
                return true;
            }
            let facts = self.class_facts(c);
            if !self.own_methods(c, name).is_empty() || facts.names.contains(name) || facts.getattr {
                return false;
            }
        }
        false
    }

    /// Step 2.4b: a method of a class outside the chain is never the target. Without a base nothing else
    /// brings it, and the step 2.3a's rule stays.
    fn left_by_chain(&self, receiver: &PyReceiver<'g>, occurrence: &str, name: &str) -> Chain<'g> {
        let declared = self.methods_by_name.get(name).map_or(&[][..], Vec::as_slice);
        let written: Vec<&'g str> = match self.graph.nodes[occurrence].ambiguous_candidates {
            Some(_) => declared.iter().copied().filter(|m| self.graph.nodes[*m].language == "python").collect(),
            None => self.candidates(occurrence).iter().map(|e| e.to.as_str()).filter(|t| self.kind(t) == "method").collect(),
        };
        if written.is_empty() {
            return Chain::Unknown;
        }
        let left: BTreeSet<&'g str> = match self.chain_classes(receiver, name) {
            Some(classes) => written.into_iter().filter(|m| self.container(m).is_some_and(|c| classes.contains(c))).collect(),
            None if !receiver.parent && self.baseless(receiver.class) => written.into_iter().filter(|m| self.subclass_reaches(receiver.class, m)).collect(),
            None => return Chain::Unknown,
        };
        if left.is_empty() {
            return Chain::Nobody;
        }
        Chain::Only(left)
    }

    /// Step 2.4a: the graph's own copy of a source path.
    fn path_ref(&self, path: &str) -> Option<&'g str> {
        self.paths.get(path).copied()
    }

    /// Step 2.4a (DEC-42): what the caller's scope says a plain call's name is (Python `f()`;
    /// Rust `f()`, `super::f()`, `m::f()`). Read from the file, as the language binds names.
    fn scoped(&self, caller: &str, occurrence: &str) -> Scoped<'g> {
        if self.kind(occurrence) != "call_observed" {
            return Scoped::Undecided;
        }
        let Some(evidence) = self.call_evidence.get(occurrence) else { return Scoped::Undecided };
        let text = self.label(occurrence);
        match self.graph.nodes[caller].language.as_str() {
            "python" if !text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '_') => {
                self.python_name(&evidence.path, evidence.start_byte, text, 0)
            }
            "rust" => self.rust_scoped(caller, &evidence.path, evidence.start_byte, text),
            _ => Scoped::Undecided,
        }
    }

    /// Step 2.4a (Python): what `name`, written at `at` in `path`, is bound to. The innermost scope
    /// that binds it decides (a class body is not seen from the functions inside it); a binding
    /// under `TYPE_CHECKING` does not exist when the code runs; at the module's own level (or a
    /// class body in it) the code runs in order, so only what is bound before counts. Bindings of
    /// which one may not run (`if`, `try`, loops) must agree; otherwise the last one counts (the
    /// implementation after its `@overload`s). No scope binds it: a builtin, or nothing.
    fn python_name(&self, path: &str, at: usize, name: &str, depth: usize) -> Scoped<'g> {
        if depth > 3 {
            return Scoped::Undecided;
        }
        let Some(facts) = self.file_facts(path) else { return Scoped::Undecided };
        let mut scopes: Vec<&NameScope> = facts.scopes.iter().filter(|s| s.around(at)).collect();
        scopes.sort_by_key(|s| s.end - s.start);
        let outermost = scopes.len().saturating_sub(1);
        let mut global = false;
        for (i, s) in scopes.iter().enumerate() {
            if (i > 0 && s.class) || (global && i < outermost) {
                continue;
            }
            // `global name`: the module's, whatever an enclosing function binds.
            if i < outermost && s.globals.contains(name) {
                global = true;
                continue;
            }
            if let Some(all) = s.bindings.get(name) {
                let in_order = i == outermost && scopes[..i].iter().all(|s| s.class);
                let live: Vec<&Binding> = all.iter().filter(|b| !b.type_checking && !(in_order && b.at >= at)).collect();
                if live.is_empty() {
                    if all.iter().all(|b| b.type_checking) {
                        return Scoped::Undecided;
                    }
                    continue;
                }
                return self.python_bindings(path, name, &live, true, depth);
            }
            if s.star {
                return Scoped::Undecided;
            }
        }
        Scoped::Nothing
    }

    /// Step 2.4a: what the live bindings of a name say. `own`: they are the caller's own scope's
    /// (a variable there is a variable; one reached through an import may hold anything).
    fn python_bindings(&self, path: &str, name: &str, live: &[&Binding], own: bool, depth: usize) -> Scoped<'g> {
        let outcomes: Vec<Scoped<'g>> = live.iter().map(|b| self.python_bound(path, name, b, own, depth)).collect();
        if live.iter().any(|b| b.conditional) && outcomes.iter().any(|o| *o != outcomes[0]) {
            return Scoped::Undecided;
        }
        outcomes.last().cloned().unwrap_or(Scoped::Undecided)
    }

    /// Step 2.4a: what one binding binds a name to.
    fn python_bound(&self, path: &str, name: &str, binding: &Binding, own: bool, depth: usize) -> Scoped<'g> {
        match &binding.kind {
            Bound::Def => {
                let Some(file) = self.path_ref(path) else { return Scoped::Undecided };
                let found = self.declared_end.get(&(file, binding.end)).and_then(|ids| ids.iter().copied().find(|id| self.label(id).rsplit('.').next() == Some(name)));
                found.map_or(Scoped::Undecided, |id| Scoped::Target(id, How::Scope))
            }
            Bound::Import(Import::From(package, imported)) => self.python_from(path, package, imported, depth),
            // A module called (`import m; m()`): not a declaration.
            Bound::Import(Import::Module(_)) => Scoped::Undecided,
            Bound::Local if own => Scoped::Nothing,
            Bound::Local => Scoped::Undecided,
            Bound::Alias(value) => self.python_alias(path, binding.at, value, depth),
        }
    }

    /// Step 2.4a: `from package import name`: the name as that module binds it (a `def` there, or
    /// what it imports in turn, up to three steps). From outside the project: nothing in it.
    fn python_from(&self, path: &str, package: &str, name: &str, depth: usize) -> Scoped<'g> {
        let submodule = format!("{package}{}{name}", if package.ends_with('.') { "" } else { "." });
        if self.module_file(path, &submodule).is_some() {
            return Scoped::Undecided;
        }
        match self.module_file(path, package) {
            Some(file) => match self.python_in_module(&file, name, depth + 1) {
                Scoped::Target(t, _) => Scoped::Target(t, How::Import),
                other => other,
            },
            None if self.project_module(path, package) => Scoped::Undecided,
            None => Scoped::Nothing,
        }
    }

    /// Step 2.4a: what a module of the project binds `name` to at its top level, once loaded.
    fn python_in_module(&self, file: &str, name: &str, depth: usize) -> Scoped<'g> {
        if depth > 3 {
            return Scoped::Undecided;
        }
        let Some(facts) = self.file_facts(file) else { return Scoped::Undecided };
        let Some(module) = facts.scopes.iter().max_by_key(|s| s.end - s.start) else { return Scoped::Undecided };
        let Some(all) = module.bindings.get(name) else { return Scoped::Undecided };
        let live: Vec<&Binding> = all.iter().filter(|b| !b.type_checking).collect();
        if live.is_empty() {
            return Scoped::Undecided;
        }
        self.python_bindings(file, name, &live, false, depth)
    }

    /// Step 2.4a (N8): `X = value` names what `value` names where it is written: `Name` by the same
    /// rule, `m.Name` in the module `m` of the project.
    fn python_alias(&self, path: &str, at: usize, value: &str, depth: usize) -> Scoped<'g> {
        if depth >= 3 {
            return Scoped::Undecided;
        }
        let Some((module, last)) = value.rsplit_once('.') else { return self.python_name(path, at, value, depth + 1) };
        let Some(facts) = self.file_facts(path) else { return Scoped::Undecided };
        let head = module.split('.').next().unwrap_or_default();
        let dotted = match self.bound(&facts, at, head) {
            Some(Some(Import::Module(m))) if module == head || module.starts_with(&format!("{head}.")) => format!("{m}{}", &module[head.len()..]),
            Some(Some(Import::From(p, n))) if module == head => format!("{p}{}{n}", if p.ends_with('.') { "" } else { "." }),
            _ => return Scoped::Undecided,
        };
        match self.module_file(path, &dotted) {
            Some(file) => match self.python_in_module(&file, last, depth + 1) {
                Scoped::Target(t, _) => Scoped::Target(t, How::Import),
                other => other,
            },
            None if self.project_module(path, &dotted) => Scoped::Undecided,
            None => Scoped::Nothing,
        }
    }

    /// Step 2.4a (N8): the class a name written at `at` names through an alias (`from p import C as
    /// D`, `D = C`, `D: TypeAlias = C`).
    fn python_alias_class(&self, path: &str, at: usize, text: &str) -> Option<&'g str> {
        match self.python_name(path, at, text, 0) {
            Scoped::Target(t, _) if self.kind(t) == "class" => Some(t),
            _ => None,
        }
    }

    /// Step 2.4a (Rust): a plain call or a module path, by the caller's scope: a variable of an
    /// enclosing function (a closure, a function value: not decided); then, for each block around the
    /// call from the innermost, and for the caller's module last, its items, its `use` and its
    /// `use ..::*`. Only what is around the call is seen.
    fn rust_scoped(&self, caller: &str, path: &'g str, at: usize, text: &str) -> Scoped<'g> {
        let segments: Vec<String> = text.split("::").map(|s| s.trim().to_owned()).collect();
        let Some(facts) = self.file_facts(path) else { return Scoped::Undecided };
        let module_node = self.rust_module_of(caller);
        let Some(here) = self.rust_module_segments(path, module_node) else { return Scoped::Undecided };
        let Some(module) = self.rust_facts_module(&facts, module_node) else { return Scoped::Undecided };
        if segments.len() == 1 {
            let name = segments[0].as_str();
            if name.is_empty() || name == "Self" {
                return Scoped::Undecided;
            }
            let mut c = Some(caller);
            while let Some(f) = c.filter(|f| self.kind(f) != "module") {
                if self.declared_span.get(f).is_some_and(|span| facts.variables.get(span).is_some_and(|v| v.contains(name))) {
                    return Scoped::Undecided;
                }
                c = self.container(f);
            }
            for block in rust_blocks(&facts, module, at) {
                let item = block.items.iter().filter_map(|span| self.declared_by_span.get(&(path, *span)).copied()).find(|id| last_segment(self.label(id)) == name);
                if let Some(id) = item {
                    return Scoped::Target(id, How::Scope);
                }
                if let Some(found) = self.rust_bound(path, &here, block, name) {
                    return found;
                }
            }
            if let Some(ids) = self.rust_items.get(&(path, module_node, name.to_owned())) {
                return Scoped::Target(ids[0], How::Scope);
            }
            return self.rust_bound(path, &here, module, name).unwrap_or(Scoped::Nothing);
        }
        // A module path: `super::f`, `m::f` (`m` a module of the project, or bound by a `use`).
        // `T::f` is the type's rule.
        if !segments[0].starts_with(|c: char| c.is_lowercase()) {
            return Scoped::Undecided;
        }
        let (name, mut module_path) = (segments[segments.len() - 1].clone(), segments[..segments.len() - 1].to_vec());
        let bound = rust_blocks(&facts, module, at).into_iter().chain([module]).find_map(|m| m.uses.get(&module_path[0]));
        if let Some(bound) = bound {
            module_path = bound.iter().cloned().chain(module_path[1..].iter().cloned()).collect();
        }
        match self.rust_module(path, &here, &module_path) {
            RustPlace::Project(file, node, segs) => match self.rust_in_module(file, node, &segs, &name, 0) {
                Scoped::Target(t, _) => Scoped::Target(t, How::Import),
                _ => Scoped::Undecided,
            },
            _ => Scoped::Undecided,
        }
    }

    /// Step 2.4a (Rust): what the `use` items of a module or a block bind a name to: the explicit one,
    /// then the globs. None when they bind nothing.
    fn rust_bound(&self, path: &str, here: &[String], module: &RustModule, name: &str) -> Option<Scoped<'g>> {
        if let Some(target) = module.uses.get(name) {
            return Some(self.rust_use(path, here, target, 0));
        }
        let (mut found, mut outside) = (vec![], false);
        for glob in &module.globs {
            match self.rust_module(path, here, glob) {
                RustPlace::Project(file, node, segs) => {
                    if let Scoped::Target(t, _) = self.rust_in_module(file, node, &segs, name, 0) {
                        if !found.contains(&t) {
                            found.push(t);
                        }
                    }
                }
                RustPlace::Outside => outside = true,
                RustPlace::Unknown => return Some(Scoped::Undecided),
            }
        }
        match found.len() {
            1 => Some(Scoped::Target(found[0], How::Import)),
            0 if !outside => None,
            _ => Some(Scoped::Undecided),
        }
    }

    /// Step 2.4a (Rust): an item a `use` names: from outside the crate, nothing of the project.
    fn rust_use(&self, path: &str, here: &[String], target: &[String], hops: usize) -> Scoped<'g> {
        let Some((name, module_path)) = target.split_last() else { return Scoped::Undecided };
        match self.rust_module(path, here, module_path) {
            RustPlace::Project(file, node, segs) => match self.rust_in_module(file, node, &segs, name, hops + 1) {
                Scoped::Target(t, _) => Scoped::Target(t, How::Import),
                _ => Scoped::Undecided,
            },
            RustPlace::Outside => Scoped::Nothing,
            RustPlace::Unknown => Scoped::Undecided,
        }
    }

    /// Step 2.4a (Rust): an item of a module, or one its `use` brings (a re-export), up to three
    /// steps.
    fn rust_in_module(&self, file: &'g str, node: Option<&'g str>, segs: &[String], name: &str, hops: usize) -> Scoped<'g> {
        if let Some(ids) = self.rust_items.get(&(file, node, name.to_owned())) {
            return Scoped::Target(ids[0], How::Scope);
        }
        if hops >= 3 {
            return Scoped::Undecided;
        }
        let Some(facts) = self.file_facts(file) else { return Scoped::Undecided };
        let Some(module) = self.rust_facts_module(&facts, node) else { return Scoped::Undecided };
        match module.uses.get(name) {
            Some(target) => self.rust_use(file, segs, target, hops),
            None => Scoped::Undecided,
        }
    }

    /// Step 2.4a (Rust): the nearest `mod {}` that contains a declaration; none at a file's top.
    fn rust_module_of(&self, id: &str) -> Option<&'g str> {
        let mut at = self.container(id);
        while let Some(c) = at {
            if self.kind(c) == "module" {
                return Some(c);
            }
            at = self.container(c);
        }
        None
    }

    /// Step 2.4a (Rust): the facts of a module: the file's own, or the `mod {}` with the node's span.
    fn rust_facts_module<'f>(&self, facts: &'f CallFacts, node: Option<&str>) -> Option<&'f RustModule> {
        match node {
            None => facts.modules.first(),
            Some(n) => {
                let span = *self.declared_span.get(n)?;
                facts.modules.iter().skip(1).find(|m| (m.start, m.end) == span)
            }
        }
    }

    /// Step 2.4a (Rust): the crate a file belongs to: the folder that holds `src/`, and its root
    /// file (`src/lib.rs`, `src/main.rs`, or the binary's own file under `src/bin/`).
    fn rust_crate(&self, path: &str) -> Option<(String, String)> {
        let at = path.rfind("src/")?;
        let dir = path[..at + 4].to_owned();
        if path[dir.len()..].starts_with("bin/") {
            return Some((dir, path.to_owned()));
        }
        let root = ["lib.rs", "main.rs"].iter().map(|f| format!("{dir}{f}")).find(|f| self.paths.contains(f.as_str()))?;
        Some((dir, root))
    }

    /// Step 2.4a (Rust): a module's path from its crate's root: the file's (`src/a/b.rs` is `a::b`)
    /// and the `mod {}` around it.
    fn rust_module_segments(&self, path: &str, node: Option<&str>) -> Option<Vec<String>> {
        let (dir, root) = self.rust_crate(path)?;
        let mut segs: Vec<String> = vec![];
        if path != root {
            segs = path[dir.len()..].trim_end_matches(".rs").split('/').map(str::to_owned).collect();
            if segs.last().is_some_and(|s| s == "mod") {
                segs.pop();
            }
        }
        let mut inline = vec![];
        let mut at = node;
        while let Some(m) = at {
            inline.push(last_segment(self.label(m)));
            at = self.rust_module_of(m);
        }
        inline.reverse();
        segs.extend(inline);
        Some(segs)
    }

    /// Step 2.4a (Rust): where a `use` path's module is: in the project (its file, its `mod {}` and
    /// its path), outside the crate, or a path of the crate that names no module (a type, a
    /// variant, a module written with `#[path]`).
    fn rust_module(&self, path: &str, here: &[String], target: &[String]) -> RustPlace<'g> {
        let Some((dir, root)) = self.rust_crate(path) else { return RustPlace::Unknown };
        let mut base: Vec<String> = match target.first().map(String::as_str) {
            Some("crate") => target[1..].to_vec(),
            Some("self") => here.iter().chain(&target[1..]).cloned().collect(),
            Some("super") => {
                let supers = target.iter().take_while(|s| *s == "super").count();
                if supers > here.len() {
                    return RustPlace::Unknown;
                }
                here[..here.len() - supers].iter().chain(&target[supers..]).cloned().collect()
            }
            Some(first) => {
                // A child of this module (paths are relative since the 2018 edition), or, from a
                // binary, the library of the same package.
                let child: Vec<String> = here.iter().chain(target).cloned().collect();
                if self.rust_find(&dir, &root, &child).is_some() {
                    child
                } else if !matches!(first, "std" | "core" | "alloc") && root.contains("/bin/") {
                    let lib = format!("{dir}lib.rs");
                    if !self.paths.contains(lib.as_str()) {
                        return RustPlace::Outside;
                    }
                    return match self.rust_find(&dir, &lib, &target[1..]) {
                        Some((file, node)) => RustPlace::Project(file, node, target[1..].to_vec()),
                        None => RustPlace::Outside,
                    };
                } else {
                    return RustPlace::Outside;
                }
            }
            None => here.to_vec(),
        };
        base.retain(|s| !s.is_empty());
        match self.rust_find(&dir, &root, &base) {
            Some((file, node)) => RustPlace::Project(file, node, base),
            None => RustPlace::Unknown,
        }
    }

    /// Step 2.4a (Rust): the file and the `mod {}` of a module path from a crate's root.
    fn rust_find(&self, dir: &str, root: &str, segs: &[String]) -> Option<(&'g str, Option<&'g str>)> {
        let mut file = self.path_ref(root)?;
        let mut node: Option<&'g str> = None;
        for seg in segs {
            // A `mod seg { .. }` here: a module node whose span is a module of the file's facts.
            let inline = self.rust_items.get(&(file, node, seg.clone())).and_then(|ids| {
                let facts = self.file_facts(file)?;
                ids.iter().copied().find(|id| self.kind(id) == "module" && self.rust_facts_module(&facts, Some(id)).is_some())
            });
            if let Some(m) = inline {
                node = Some(m);
                continue;
            }
            if node.is_some() {
                return None;
            }
            let folder = if file == root || file.starts_with(&format!("{dir}bin/")) {
                dir.to_owned()
            } else if let Some(stem) = file.strip_suffix("mod.rs") {
                stem.to_owned()
            } else {
                format!("{}/", file.trim_end_matches(".rs"))
            };
            file = [format!("{folder}{seg}.rs"), format!("{folder}{seg}/mod.rs")].iter().find_map(|f| self.path_ref(f))?;
        }
        Some((file, node))
    }

    /// Step 2.4a (Rust, N8): the type of the project a name names through a `use .. as` or a
    /// `type X = Y;` of the module written around `at`.
    fn rust_alias(&self, path: &str, at: usize, name: &str) -> Option<String> {
        let facts = self.file_facts(path)?;
        let module = facts.modules.iter().filter(|m| m.start <= at && at < m.end).min_by_key(|m| m.end - m.start)?;
        let named = rust_blocks(&facts, module, at)
            .into_iter()
            .chain([module])
            .find_map(|m| m.uses.get(name).and_then(|p| p.last().cloned()).or_else(|| m.aliases.get(name).cloned()))?;
        (named != name && self.type_names.contains(&named)).then_some(named)
    }

    /// The rule. `caller` is the symbol whose body writes the call; `chosen`, the candidate the
    /// person picked for this occurrence, if any.
    pub fn resolve(&self, caller: &str, occurrence: &str, chosen: Option<&str>) -> Resolution<'g> {
        let all = self.candidates(occurrence);
        let (edges, module) = self.admit(caller, occurrence);
        // Step 2.4b (DEC-43): the receiver's static class, by its chain of bases.
        let chain = self.chain(caller, occurrence, module);
        if let Chain::Overrides(overrides) = &chain {
            return match chosen.and_then(|c| overrides.iter().find(|o| **o == c)) {
                Some(picked) => Resolution::Linked(How::Chosen, vec![picked]),
                None => Resolution::Overrides(overrides.clone()),
            };
        }
        let edges = self.in_chain(edges, &chain);
        if matches!(chain, Chain::Nobody) && edges.is_empty() && (!all.is_empty() || self.graph.nodes[occurrence].ambiguous_candidates.is_some()) {
            return Resolution::OutOfChain;
        }
        if let Some(e) = chosen.and_then(|c| edges.iter().find(|e| e.to == c)) {
            return Resolution::Linked(How::Chosen, vec![&e.to]);
        }
        // Step 2.4a (DEC-42): a plain call by the caller's scope. A `unique` it confirms keeps its
        // label; otherwise it settles among homonyms and above the limit, or rules the candidates out.
        let text = self.label(occurrence);
        let plain = call_parts(text).is_some_and(|(receiver, _)| receiver.is_empty());
        match self.scoped(caller, occurrence) {
            Scoped::Nothing => return Resolution::NoCandidate,
            Scoped::Target(to, how) => {
                if edges.len() == 1 && edges[0].to == to && (all.len() == 1 || plain) {
                    return Resolution::Linked(How::Unique, vec![&edges[0].to]);
                }
                return Resolution::Linked(how, vec![to]);
            }
            Scoped::Undecided => {}
        }
        if edges.len() == 1 {
            let to = edges[0].to.as_str();
            if module && all.len() > 1 {
                return Resolution::Linked(How::Import, vec![to]);
            }
            if all.len() == 1 || plain || module {
                if self.type_rules_out(caller, occurrence, to) {
                    return Resolution::NoCandidate;
                }
                // Step 2.4b: the chain's method, when it is not this candidate.
                if let Chain::Methods(found) = &chain {
                    if !found.contains(&to) {
                        return Resolution::Linked(How::Type, found.clone());
                    }
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
        // Step 2.4b: or the one it inherits (`chain`).
        if let Chain::Methods(found) = chain {
            return Resolution::Linked(How::Type, found);
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

/// Step 2.4a (Rust): the blocks around `at` that declare items, inside the module, innermost first.
fn rust_blocks<'f>(facts: &'f CallFacts, module: &RustModule, at: usize) -> Vec<&'f RustModule> {
    let mut blocks: Vec<&RustModule> = facts.blocks.iter().filter(|b| b.start <= at && at < b.end && module.start <= b.start && b.end <= module.end).collect();
    blocks.sort_by_key(|b| b.end - b.start);
    blocks
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
