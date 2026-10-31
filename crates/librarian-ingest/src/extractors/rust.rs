//! Rust adapter (syn). Only syntactic declarations and occurrences.
//! No macro expansion, cfg evaluation or name resolution.
use super::{Extraction, Extractor, Occurrence, OwnerSyntax};
use crate::{hash, Chunk, Diagnostic, Source, Symbol};
use proc_macro2::Span;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

/// `g6` adds named struct fields, field accesses and call candidates (profile v3).
/// `g6d` adds struct literals, field writes and the type written in an impl (profile v4).
/// `v5` adds enum variants, names used as values, struct patterns and loops (profile v5).
/// `v6` adds closures of iterator chains as loops and the written type of a method call's
/// receiver (profile v7).
/// `v7` adds the written type of a field access's or write's base (`group.transform = ..`) (profile v8).
/// Without them, records and occurrences are exactly those of the frozen v1 adapter.
pub(crate) struct Rust {
    g6: bool,
    g6d: bool,
    v5: bool,
    v6: bool,
    v7: bool,
}

pub(crate) static RUST: Rust = Rust { g6: false, g6d: false, v5: false, v6: false, v7: false };
pub(crate) static RUST_V3: Rust = Rust { g6: true, g6d: false, v5: false, v6: false, v7: false };
pub(crate) static RUST_V4: Rust = Rust { g6: true, g6d: true, v5: false, v6: false, v7: false };
pub(crate) static RUST_V5: Rust = Rust { g6: true, g6d: true, v5: true, v6: false, v7: false };
pub(crate) static RUST_V6: Rust = Rust { g6: true, g6d: true, v5: true, v6: true, v7: false };
pub(crate) static RUST_V7: Rust = Rust { g6: true, g6d: true, v5: true, v6: true, v7: true };

impl Extractor for Rust {
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
        let (tree, offset) = match parse(text) {
            Ok(parsed) => parsed,
            Err(error) => return failure("rust_parse_error", error.to_string()),
        };
        let mut visitor = Declarations {
            g6: self.g6,
            v5: self.v5,
            source,
            bytes,
            offset,
            scope: vec![],
            symbols: vec![],
            chunks: vec![],
        };
        visitor.visit_file(&tree);
        Extraction {
            symbols: visitor.symbols,
            chunks: visitor.chunks,
            diagnostics: vec![],
        }
    }

    fn observe(&self, bytes: &[u8]) -> Vec<Occurrence> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return vec![];
        };
        let Ok((tree, offset)) = parse(text) else {
            return vec![];
        };
        let mut shapes = Shapes::default();
        if self.v6 {
            shapes.visit_file(&tree);
        }
        let mut observer = Occurrences {
            g6: self.g6,
            g6d: self.g6d,
            v5: self.v5,
            v6: self.v6,
            v7: self.v7,
            offset,
            found: vec![],
            impl_type: None,
            locals: vec![],
            shapes,
            impl_name: None,
            types: vec![],
        };
        observer.visit_file(&tree);
        observer.found
    }

    fn candidate_kinds(&self) -> &'static [&'static str] {
        &["struct", "enum", "trait", "type", "union"]
    }

    fn candidate_kinds_for(&self, occurrence: &str) -> &'static [&'static str] {
        if self.v6 && occurrence == "receiver_type" {
            return &["struct", "enum", "union", "type", "trait"];
        }
        match (self.v5, occurrence) {
            // A tuple variant or tuple struct is built by a call (`Shape::Sphere(r)`).
            (true, "call_observed") => return &["function", "method", "trait_method", "variant", "struct"],
            (true, "name_reference") => return &["const", "static", "variant", "function", "method", "struct"],
            (true, "struct_literal") => return &["struct", "enum", "union", "type", "variant"],
            (true, "struct_pattern") => return &["struct", "union", "variant"],
            (true, "field_pattern") => return &["field"],
            _ => {}
        }
        match (self.g6, occurrence) {
            (true, "call_observed" | "method_call_observed") => &["function", "method", "trait_method"],
            (true, "attribute_access" | "field_assign" | "field_init") => &["field"],
            (true, "impl_self_type" | "struct_literal") => &["struct", "enum", "union", "type"],
            _ => self.candidate_kinds(),
        }
    }

    fn navigation_kinds(&self) -> &'static [&'static str] {
        &["module", "impl"]
    }

    fn owner_syntax(&self) -> Option<&'static OwnerSyntax> {
        self.g6d.then_some(&OWNER)
    }

    fn call_kinds(&self) -> &'static [&'static str] {
        &["call_observed", "method_call_observed"]
    }

    fn matched_only(&self) -> &'static [&'static str] {
        if self.v5 {
            &["name_reference", "struct_pattern", "field_pattern"]
        } else {
            &[]
        }
    }
}

/// A method's owner is the type its impl names; fields are written by assignment or literal.
static OWNER: OwnerSyntax = OwnerSyntax {
    method_kinds: &["method"],
    owner_containers: &[],
    naming_container: Some(("impl", "impl_self_type")),
    owner_kinds: &["struct", "enum", "union"],
    receiver: "self",
    read: "attribute_access",
    write: Some("field_assign"),
    literal: Some(("struct_literal", "field_init", "Self")),
};

/// parse_file strips a UTF-8 BOM and shebang before parsing; the offset restores original bytes.
fn parse(text: &str) -> syn::Result<(syn::File, usize)> {
    let tree = syn::parse_file(text)?;
    let offset = if text.starts_with('\u{feff}') { 3 } else { 0 }
        + tree.shebang.as_ref().map_or(0, String::len);
    Ok((tree, offset))
}

struct Declarations<'a> {
    g6: bool,
    v5: bool,
    source: &'a Source,
    bytes: &'a [u8],
    offset: usize,
    scope: Vec<String>,
    symbols: Vec<Symbol>,
    chunks: Vec<Chunk>,
}

impl Declarations<'_> {
    fn add(&mut self, kind: &str, name: String, span: Span, attrs: &[syn::Attribute]) {
        let parsed_range = span.byte_range();
        let range = (parsed_range.start + self.offset)..(parsed_range.end + self.offset);
        let id = format!("{}:{}:{}:{}", self.source.id, kind, range.start, range.end);
        let mut qualified = self.scope.clone();
        qualified.push(name.clone());
        self.symbols.push(Symbol {
            id: id.clone(),
            source_id: self.source.id.clone(),
            name,
            qualified_name: qualified.join("::"),
            kind: kind.into(),
            // This is an attribute observation, not evidence that a test ran.
            is_test: attrs.iter().any(|a| a.path().is_ident("test")),
            chunk_id: id.clone(),
        });
        self.chunks.push(Chunk {
            id,
            source_id: self.source.id.clone(),
            start_byte: range.start,
            end_byte: range.end,
            start_line: crate::line_at(self.bytes, range.start),
            end_line: crate::line_at(self.bytes, range.end.saturating_sub(1)),
            sha256: hash(&self.bytes[range]),
        });
    }

    fn span_text(&self, span: Span) -> String {
        let range = span.byte_range();
        String::from_utf8_lossy(&self.bytes[range.start + self.offset..range.end + self.offset])
            .into_owned()
    }
}

impl<'ast> Visit<'ast> for Declarations<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let declaration = match item {
            syn::Item::Fn(i) => Some(("function", i.sig.ident.to_string(), &i.attrs)),
            syn::Item::Struct(i) => Some(("struct", i.ident.to_string(), &i.attrs)),
            syn::Item::Enum(i) => Some(("enum", i.ident.to_string(), &i.attrs)),
            syn::Item::Union(i) => Some(("union", i.ident.to_string(), &i.attrs)),
            syn::Item::Trait(i) => Some(("trait", i.ident.to_string(), &i.attrs)),
            syn::Item::Type(i) => Some(("type", i.ident.to_string(), &i.attrs)),
            syn::Item::Mod(i) => Some(("module", i.ident.to_string(), &i.attrs)),
            syn::Item::Const(i) => Some(("const", i.ident.to_string(), &i.attrs)),
            syn::Item::Static(i) => Some(("static", i.ident.to_string(), &i.attrs)),
            syn::Item::Impl(i) => {
                let target = self.span_text(i.self_ty.span());
                let name = match &i.trait_ {
                    Some((path, _)) => {
                        format!("impl {} for {}", self.span_text(path.span()), target)
                    }
                    None => format!("impl {}", target),
                };
                Some(("impl", name, &i.attrs))
            }
            _ => None,
        };
        if let Some((kind, name, attrs)) = declaration {
            self.add(kind, name.clone(), item.span(), attrs);
            self.scope.push(name);
            // Named fields are declarations of their struct; tuple fields have no name to match.
            if let (true, syn::Item::Struct(s)) = (self.g6, item) {
                for field in &s.fields {
                    if let Some(ident) = &field.ident {
                        self.add("field", ident.to_string(), field.span(), &field.attrs);
                    }
                }
            }
            // v5: each variant is a declaration of its enum (`Shape::Sphere`).
            if let (true, syn::Item::Enum(e)) = (self.v5, item) {
                for variant in &e.variants {
                    self.add("variant", variant.ident.to_string(), variant.span(), &variant.attrs);
                }
            }
            visit::visit_item(self, item);
            self.scope.pop();
        } else {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let name = item.sig.ident.to_string();
        self.add("method", name.clone(), item.span(), &item.attrs);
        self.scope.push(name);
        visit::visit_impl_item_fn(self, item);
        self.scope.pop();
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        let name = item.sig.ident.to_string();
        self.add("trait_method", name.clone(), item.span(), &item.attrs);
        self.scope.push(name);
        visit::visit_trait_item_fn(self, item);
        self.scope.pop();
    }
}

/// Written types, returns, calls and bindings. Calls are never linked to presumed targets.
struct Occurrences {
    g6: bool,
    g6d: bool,
    v5: bool,
    v6: bool,
    v7: bool,
    offset: usize,
    found: Vec<Occurrence>,
    /// Span of the type written in the impl being visited: it is `impl_self_type`, not also a
    /// type reference.
    impl_type: Option<std::ops::Range<usize>>,
    /// v5: names bound in each enclosing function (parameters, `let`, patterns, closures). A
    /// one-segment path with one of these names is a local, not a use of a declaration.
    locals: Vec<std::collections::BTreeSet<String>>,
    /// v6: field and variant types declared in this file.
    shapes: Shapes,
    /// v6: the type written in the impl being visited, by its last segment.
    impl_name: Option<String>,
    /// v6: names with a written type in each enclosing function. A later binding of the same
    /// name replaces the earlier one; blocks are not scopes.
    types: Vec<BTreeMap<String, String>>,
}

use std::collections::BTreeMap;

/// v6: the types a file declares for struct fields and tuple variants, read before the
/// occurrences. Only this file: a struct declared elsewhere has no known fields.
#[derive(Default)]
struct Shapes {
    /// struct -> field -> (type, element type of a collection).
    fields: BTreeMap<String, BTreeMap<String, (Option<String>, Option<String>)>>,
    /// (enum, variant) -> the type of its only field.
    variants: BTreeMap<(String, String), String>,
    /// v7: free function -> the type its signature returns.
    returns: BTreeMap<String, String>,
}

impl<'ast> Visit<'ast> for Shapes {
    fn visit_item_struct(&mut self, value: &'ast syn::ItemStruct) {
        let fields = self.fields.entry(value.ident.to_string()).or_default();
        for field in &value.fields {
            if let Some(ident) = &field.ident {
                fields.insert(ident.to_string(), (type_name(&field.ty), element_name(&field.ty)));
            }
        }
    }
    fn visit_item_fn(&mut self, value: &'ast syn::ItemFn) {
        if let syn::ReturnType::Type(_, ty) = &value.sig.output {
            if let Some(name) = type_name(ty) {
                self.returns.insert(value.sig.ident.to_string(), name);
            }
        }
        visit::visit_item_fn(self, value);
    }
    fn visit_item_enum(&mut self, value: &'ast syn::ItemEnum) {
        for variant in &value.variants {
            if let syn::Fields::Unnamed(fields) = &variant.fields {
                if let (1, Some(ty)) = (fields.unnamed.len(), fields.unnamed.first().and_then(|f| type_name(&f.ty))) {
                    self.variants.insert((value.ident.to_string(), variant.ident.to_string()), ty);
                }
            }
        }
    }
}

/// The written type by its last segment, references and parentheses dropped: `&mut
/// Matrix<4>` -> `Matrix`. None for tuples, slices, functions and the like.
fn type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Reference(r) => type_name(&r.elem),
        syn::Type::Paren(p) => type_name(&p.elem),
        syn::Type::Group(g) => type_name(&g.elem),
        syn::Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

/// The element type of a written collection: `Vec<Object>`, `[Object; 4]`, `&[Object]` ->
/// `Object`.
fn element_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Reference(r) => element_name(&r.elem),
        syn::Type::Paren(p) => element_name(&p.elem),
        syn::Type::Array(a) => type_name(&a.elem),
        syn::Type::Slice(s) => type_name(&s.elem),
        syn::Type::Path(p) => {
            let last = p.path.segments.last()?;
            if !matches!(last.ident.to_string().as_str(), "Vec" | "VecDeque" | "LinkedList" | "BTreeSet" | "HashSet" | "BinaryHeap") {
                return None;
            }
            let syn::PathArguments::AngleBracketed(args) = &last.arguments else { return None };
            match args.args.first()? {
                syn::GenericArgument::Type(t) => type_name(t),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Methods that start or adapt an iterator; a chain of them, down to one of these or a range,
/// is an iterator.
const ITERATOR_SOURCES: &[&str] = &[
    "iter", "iter_mut", "into_iter", "par_iter", "par_iter_mut", "into_par_iter", "par_bridge", "chars", "bytes",
    "char_indices", "lines", "split_whitespace", "windows", "chunks", "chunks_exact", "keys", "values", "values_mut", "drain",
];
const ITERATOR_ADAPTERS: &[&str] = &[
    "enumerate", "zip", "rev", "step_by", "skip", "take", "chain", "cycle", "flatten", "peekable", "copied", "cloned",
    "map", "filter", "filter_map", "flat_map", "inspect", "take_while", "skip_while", "scan", "cartesian_product",
];
/// Methods whose closure runs once per element of the iterator they are called on.
const ITERATOR_BODIES: &[&str] = &[
    "for_each", "map", "filter", "filter_map", "flat_map", "fold", "any", "all", "find", "find_map", "position",
    "try_for_each", "for_each_with", "for_each_init", "map_with", "map_init", "inspect", "take_while", "skip_while",
    "max_by_key", "min_by_key", "max_by", "min_by", "partition", "scan", "reduce",
];

/// Whether an expression is, as written, an iterator: a chain of sources and adapters that
/// ends in a source or a range. `opt.map(..)` is not; `(0..n).map(..)` and `v.iter().map(..)` are.
fn is_iterator(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Range(_) => true,
        syn::Expr::Paren(p) => is_iterator(&p.expr),
        syn::Expr::MethodCall(m) => {
            let name = m.method.to_string();
            ITERATOR_SOURCES.contains(&name.as_str()) || (ITERATOR_ADAPTERS.contains(&name.as_str()) && is_iterator(&m.receiver))
        }
        _ => false,
    }
}

/// Every name a pattern binds inside a function, closures included.
#[derive(Default)]
struct Bindings(std::collections::BTreeSet<String>);

impl<'ast> Visit<'ast> for Bindings {
    fn visit_pat_ident(&mut self, value: &'ast syn::PatIdent) {
        self.0.insert(value.ident.to_string());
        visit::visit_pat_ident(self, value);
    }
    // A nested item is its own scope.
    fn visit_item(&mut self, _value: &'ast syn::Item) {}
}

impl Occurrences {
    /// G6d: `x.field = ...` (or `+=` and the like) writes the field: a `field_assign`, not also an
    /// access, and only the receiver is visited. False when the target is not a named field.
    fn write(&mut self, target: &syn::Expr) -> bool {
        let syn::Expr::Field(field) = target else { return false };
        let syn::Member::Named(ident) = &field.member else { return false };
        self.push("field_assign", field.span(), Some(ident.to_string()));
        self.base_type(&field.base);
        self.visit_expr(&field.base);
        true
    }

    /// v7: the written type of the base of a field access or write (`group` in
    /// `group.transform = ..`), as a receiver_type spanning the base.
    fn base_type(&mut self, base: &syn::Expr) {
        if self.v7 {
            if let Some(ty) = self.type_of(base) {
                self.push("receiver_type", base.span(), Some(ty));
            }
        }
    }

    fn push(&mut self, kind: &'static str, span: Span, candidate: Option<String>) {
        let range = span.byte_range();
        self.found.push(Occurrence {
            kind,
            start: range.start + self.offset,
            end: range.end + self.offset,
            candidate,
            label_end: None,
        });
    }

    /// v5: a loop spans its body; its label is its head, up to `head`.
    fn block(&mut self, kind: &'static str, span: Span, head: Span) {
        let range = span.byte_range();
        self.found.push(Occurrence {
            kind,
            start: range.start + self.offset,
            end: range.end + self.offset,
            candidate: None,
            label_end: Some(head.byte_range().end + self.offset),
        });
    }

    /// v5: visits a function with the names it binds as its locals. v6: and with the types
    /// written for its parameters.
    fn scoped(&mut self, bindings: Bindings, sig: &syn::Signature, visit: impl FnOnce(&mut Self)) {
        if !self.v5 {
            return visit(self);
        }
        self.locals.push(bindings.0);
        if self.v6 {
            let mut types = BTreeMap::new();
            for input in &sig.inputs {
                if let syn::FnArg::Typed(typed) = input {
                    if let (syn::Pat::Ident(ident), Some(ty)) = (typed.pat.as_ref(), self.written(&typed.ty)) {
                        types.insert(ident.ident.to_string(), ty);
                    }
                }
            }
            self.types.push(types);
        }
        visit(self);
        if self.v6 {
            self.types.pop();
        }
        self.locals.pop();
    }

    /// v6: a written type by its last segment, `Self` read as the impl's type.
    fn written(&self, ty: &syn::Type) -> Option<String> {
        type_name(ty).and_then(|t| if t == "Self" { self.impl_name.clone() } else { Some(t) })
    }

    fn bind(&mut self, name: String, ty: String) {
        if let Some(types) = self.types.last_mut() {
            types.insert(name, ty);
        }
    }

    /// v6: the field of the impl's type that `self.f` names, as declared in this file.
    fn own_field(&self, expr: &syn::Expr) -> Option<&(Option<String>, Option<String>)> {
        let syn::Expr::Field(field) = expr else { return None };
        let (syn::Expr::Path(base), syn::Member::Named(ident)) = (field.base.as_ref(), &field.member) else { return None };
        if !base.path.is_ident("self") {
            return None;
        }
        self.shapes.fields.get(self.impl_name.as_ref()?)?.get(&ident.to_string())
    }

    /// v6: the type of an expression when it is written where the expression is: `self` in
    /// an impl, a name with a written type, `self.f` with a field declared in this file,
    /// `T::f(..)` and `T { .. }`.
    fn type_of(&self, expr: &syn::Expr) -> Option<String> {
        match expr {
            syn::Expr::Paren(p) => self.type_of(&p.expr),
            syn::Expr::Reference(r) => self.type_of(&r.expr),
            syn::Expr::Path(p) if p.path.is_ident("self") => self.impl_name.clone(),
            syn::Expr::Path(p) => {
                let name = p.path.get_ident()?.to_string();
                self.types.last()?.get(&name).cloned()
            }
            syn::Expr::Field(_) => self.own_field(expr)?.0.clone(),
            syn::Expr::Call(call) => {
                let syn::Expr::Path(p) = call.func.as_ref() else { return None };
                let segments = &p.path.segments;
                // v7: `f(..)` of a free function of this file, by the type its signature returns.
                if let (true, 1) = (self.v7, segments.len()) {
                    return self.shapes.returns.get(&segments[0].ident.to_string()).cloned();
                }
                let owner = segments.iter().rev().nth(1)?.ident.to_string();
                if owner == "Self" {
                    return self.impl_name.clone();
                }
                owner.starts_with(|c: char| c.is_uppercase()).then_some(owner)
            }
            syn::Expr::Struct(s) => {
                let name = s.path.segments.last()?.ident.to_string();
                if name == "Self" { self.impl_name.clone() } else { Some(name) }
            }
            _ => None,
        }
    }

    /// v6: the element type of what a `for` walks, when it is a field of the impl's type
    /// declared in this file as a collection: `&self.objects`, `self.objects.iter()`,
    /// `self.objects.iter().enumerate()` (then the second name of the pattern).
    fn element_of(&self, expr: &syn::Expr) -> Option<(String, bool)> {
        match expr {
            syn::Expr::Paren(p) => self.element_of(&p.expr),
            syn::Expr::Reference(r) => self.element_of(&r.expr),
            syn::Expr::MethodCall(m) => match m.method.to_string().as_str() {
                "iter" | "iter_mut" | "into_iter" => self.element_of(&m.receiver),
                "enumerate" => self.element_of(&m.receiver).filter(|(_, e)| !e).map(|(t, _)| (t, true)),
                _ => None,
            },
            syn::Expr::Field(_) => self.own_field(expr)?.1.clone().map(|t| (t, false)),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for Occurrences {
    fn visit_type_path(&mut self, value: &'ast syn::TypePath) {
        if self.impl_type.as_ref() == Some(&value.span().byte_range()) {
            self.impl_type = None;
        } else {
            let name = value.path.segments.last().map(|s| s.ident.to_string());
            self.push("type_reference", value.span(), name);
        }
        visit::visit_type_path(self, value);
    }
    fn visit_item_impl(&mut self, value: &'ast syn::ItemImpl) {
        // G6d: the type an impl is for, by its written name (`impl<T> Shape for Sphere<T>` ->
        // `Sphere`). Trait and generic arguments stay type references.
        if let (true, syn::Type::Path(path)) = (self.g6d, value.self_ty.as_ref()) {
            if let Some(last) = path.path.segments.last() {
                self.push("impl_self_type", last.ident.span(), Some(last.ident.to_string()));
                self.impl_type = Some(path.span().byte_range());
            }
        }
        let outer = self.impl_name.take();
        if self.v6 {
            self.impl_name = type_name(&value.self_ty);
        }
        visit::visit_item_impl(self, value);
        self.impl_type = None;
        self.impl_name = outer;
    }
    fn visit_item_trait(&mut self, value: &'ast syn::ItemTrait) {
        // A trait's default method has no written type for `self`.
        let outer = self.impl_name.take();
        visit::visit_item_trait(self, value);
        self.impl_name = outer;
    }
    fn visit_expr_struct(&mut self, value: &'ast syn::ExprStruct) {
        // G6d: `Camera { transform, .. }` constructs the written type, and each named field it
        // fills is a `field_init` inside it. `Self` matches no declaration.
        if self.g6d {
            let name = value.path.segments.last().map(|s| s.ident.to_string());
            self.push("struct_literal", value.span(), name);
            for field in &value.fields {
                if let syn::Member::Named(ident) = &field.member {
                    self.push("field_init", ident.span(), Some(ident.to_string()));
                }
            }
        }
        visit::visit_expr_struct(self, value);
    }
    fn visit_expr_assign(&mut self, value: &'ast syn::ExprAssign) {
        if !self.g6d || !self.write(&value.left) {
            return visit::visit_expr_assign(self, value);
        }
        self.visit_expr(&value.right);
    }
    fn visit_expr_binary(&mut self, value: &'ast syn::ExprBinary) {
        use syn::BinOp::*;
        let compound = matches!(value.op, AddAssign(_) | SubAssign(_) | MulAssign(_) | DivAssign(_) | RemAssign(_)
            | BitXorAssign(_) | BitAndAssign(_) | BitOrAssign(_) | ShlAssign(_) | ShrAssign(_));
        if !self.g6d || !compound || !self.write(&value.left) {
            return visit::visit_expr_binary(self, value);
        }
        self.visit_expr(&value.right);
    }
    fn visit_return_type(&mut self, value: &'ast syn::ReturnType) {
        if let syn::ReturnType::Type(_, ty) = value {
            self.push("declared_return", ty.span(), None);
        }
        visit::visit_return_type(self, value);
    }
    fn visit_expr_call(&mut self, value: &'ast syn::ExprCall) {
        // The last path segment is the written name (`Ray::new` -> `new`); never a resolved callee.
        let name = match (self.g6, value.func.as_ref()) {
            (true, syn::Expr::Path(p)) => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        };
        self.push("call_observed", value.func.span(), name);
        // v5: a called path is the call; it is not also a name used as a value.
        if self.v5 && matches!(value.func.as_ref(), syn::Expr::Path(_)) {
            return value.args.iter().for_each(|a| self.visit_expr(a));
        }
        visit::visit_expr_call(self, value);
    }
    fn visit_expr_path(&mut self, value: &'ast syn::ExprPath) {
        // v5: a path used as a value (`MAX_DEPTH`, `Shape::Sphere`, `Self::helper`) or matched
        // in a pattern, by its last segment. A one-segment local is not a use.
        if self.v5 {
            let segments = &value.path.segments;
            if let Some(last) = segments.last() {
                let name = last.ident.to_string();
                let local = segments.len() == 1 && (name == "self" || self.locals.iter().any(|l| l.contains(&name)));
                if !local {
                    self.push("name_reference", value.span(), Some(name));
                }
            }
        }
        visit::visit_expr_path(self, value);
    }
    fn visit_pat_struct(&mut self, value: &'ast syn::PatStruct) {
        // v5: `Camera { transform, .. }` takes the written type apart; each named field it
        // reads is a `field_pattern`.
        if self.v5 {
            let name = value.path.segments.last().map(|s| s.ident.to_string());
            self.push("struct_pattern", value.span(), name);
            for field in &value.fields {
                if let syn::Member::Named(ident) = &field.member {
                    self.push("field_pattern", ident.span(), Some(ident.to_string()));
                }
            }
        }
        visit::visit_pat_struct(self, value);
    }
    fn visit_pat_tuple_struct(&mut self, value: &'ast syn::PatTupleStruct) {
        if self.v5 {
            let name = value.path.segments.last().map(|s| s.ident.to_string());
            self.push("struct_pattern", value.span(), name);
        }
        // v6: `Object::Sphere(ref sphere)` binds `sphere` to the variant's field type, when
        // the enum is declared in this file.
        if let (true, 1, Some(syn::Pat::Ident(ident))) = (self.v6, value.elems.len(), value.elems.first()) {
            let segments = &value.path.segments;
            if let (Some(variant), Some(owner)) = (segments.last(), segments.iter().rev().nth(1)) {
                let owner = owner.ident.to_string();
                let owner = if owner == "Self" { self.impl_name.clone() } else { Some(owner) };
                if let Some(ty) = owner.and_then(|o| self.shapes.variants.get(&(o, variant.ident.to_string())).cloned()) {
                    self.bind(ident.ident.to_string(), ty);
                }
            }
        }
        visit::visit_pat_tuple_struct(self, value);
    }
    fn visit_expr_for_loop(&mut self, value: &'ast syn::ExprForLoop) {
        if self.v5 {
            self.block("loop", value.span(), value.expr.span());
            // The iterated expression runs once, before the loop.
            self.push("loop_iterable", value.expr.span(), None);
        }
        if self.v6 {
            if let Some((ty, enumerated)) = self.element_of(&value.expr) {
                let name = match (value.pat.as_ref(), enumerated) {
                    (syn::Pat::Ident(ident), false) => Some(&ident.ident),
                    (syn::Pat::Tuple(tuple), true) if tuple.elems.len() == 2 => match &tuple.elems[1] {
                        syn::Pat::Ident(ident) => Some(&ident.ident),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(name) = name {
                    self.bind(name.to_string(), ty);
                }
            }
        }
        visit::visit_expr_for_loop(self, value);
    }
    fn visit_expr_while(&mut self, value: &'ast syn::ExprWhile) {
        if self.v5 {
            self.block("loop", value.span(), value.cond.span());
        }
        visit::visit_expr_while(self, value);
    }
    fn visit_expr_loop(&mut self, value: &'ast syn::ExprLoop) {
        if self.v5 {
            self.block("loop", value.span(), value.loop_token.span());
        }
        visit::visit_expr_loop(self, value);
    }
    fn visit_item_fn(&mut self, value: &'ast syn::ItemFn) {
        let mut bindings = Bindings::default();
        if self.v5 {
            bindings.visit_item_fn(value);
        }
        self.scoped(bindings, &value.sig, |s| visit::visit_item_fn(s, value));
    }
    fn visit_impl_item_fn(&mut self, value: &'ast syn::ImplItemFn) {
        let mut bindings = Bindings::default();
        if self.v5 {
            bindings.visit_impl_item_fn(value);
        }
        self.scoped(bindings, &value.sig, |s| visit::visit_impl_item_fn(s, value));
    }
    fn visit_trait_item_fn(&mut self, value: &'ast syn::TraitItemFn) {
        let mut bindings = Bindings::default();
        if self.v5 {
            bindings.visit_trait_item_fn(value);
        }
        self.scoped(bindings, &value.sig, |s| visit::visit_trait_item_fn(s, value));
    }
    fn visit_expr_method_call(&mut self, value: &'ast syn::ExprMethodCall) {
        // Keep receiver and arguments; no receiver type is inferred.
        let name = self.g6.then(|| value.method.to_string());
        self.push("method_call_observed", value.span(), name);
        if self.v6 {
            // The receiver's type only when it is written (see type_of); never inferred.
            if let Some(ty) = self.type_of(&value.receiver) {
                self.push("receiver_type", value.receiver.span(), Some(ty));
            }
            // A closure run once per element: a loop from the method's name to the closure's
            // end, labelled up to its parameters (`for_each(|(x, y)|`).
            if ITERATOR_BODIES.contains(&value.method.to_string().as_str()) && is_iterator(&value.receiver) {
                for argument in &value.args {
                    if let syn::Expr::Closure(closure) = argument {
                        self.found.push(Occurrence {
                            kind: "loop",
                            start: value.method.span().byte_range().start + self.offset,
                            end: closure.span().byte_range().end + self.offset,
                            candidate: None,
                            label_end: Some(closure.inputs_end.span().byte_range().end + self.offset),
                        });
                    }
                }
            }
        }
        visit::visit_expr_method_call(self, value);
    }
    fn visit_expr_field(&mut self, value: &'ast syn::ExprField) {
        if let (true, syn::Member::Named(ident)) = (self.g6, &value.member) {
            self.push("attribute_access", value.span(), Some(ident.to_string()));
            self.base_type(&value.base);
        }
        visit::visit_expr_field(self, value);
    }
    fn visit_local(&mut self, value: &'ast syn::Local) {
        self.push("local_binding", value.pat.span(), None);
        visit::visit_local(self, value);
        // v6: after the value, so `let x = x.f()` reads the earlier `x`.
        if self.v6 {
            let (pat, written) = match &value.pat {
                syn::Pat::Type(typed) => (typed.pat.as_ref(), self.written(&typed.ty)),
                pat => (pat, value.init.as_ref().and_then(|i| self.type_of(&i.expr))),
            };
            if let (syn::Pat::Ident(ident), Some(ty)) = (pat, written) {
                self.bind(ident.ident.to_string(), ty);
            }
        }
    }
}
