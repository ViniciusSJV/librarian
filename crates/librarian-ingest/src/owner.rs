//! G6d: the owner context of a chain step. A method target brings its owner type and the
//! declarations that write the owner's fields the method reads. Everything comes from written
//! names, in the forms the edition profile's adapter declares (`OwnerSyntax`): the container
//! that is or names the owner, `<receiver>.<field>` in the method, `<receiver>.<field> = ...` in
//! another method of the same owner, and literals of the owner type. Nothing is resolved by type.
use crate::{
    extractors::OwnerSyntax,
    graph::{Evidence, Graph},
    Catalog, Result,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// At most this many declarations are added to a chain step; the rest is reported as truncated.
pub const MAX_ADDED: usize = 8;
/// Writes not followed are listed up to this many; the total is always reported.
const MAX_LISTED: usize = 20;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnerContext {
    pub added: Vec<Added>,
    /// Writes of a read field through another receiver (`camera.transform = ...`): the receiver
    /// type is not known, so they are counted and listed, never added.
    pub not_followed: Vec<NotFollowed>,
    pub not_followed_total: usize,
    /// True when a declaration was left out because of `MAX_ADDED`.
    pub truncated: bool,
    /// Targets whose owner could not be told by name (homonym types, none declared).
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Added {
    pub symbol_id: String,
    pub name: String,
    pub kind: String,
    /// `owner_type`, `field_writer` or `called` (a declaration the target calls).
    pub reason: String,
    /// Qualified name of the target this declaration was added for.
    pub for_target: String,
    /// `container` (the declaration that contains the target is its owner), `container_name`
    /// (the type named by the target's container), `receiver_in_owner` (`<receiver>.f = ...` in
    /// another method of the owner), `owner_literal` (a literal of the owner type), or, for a
    /// called declaration, `unique` or `receiver` as the call traversal resolved it.
    pub resolution: String,
    /// Owner type: the fields the target reads. Writer: the read fields it writes.
    pub fields: Vec<String>,
    /// Where the link is written: the container's type name, or each write.
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotFollowed {
    pub field: String,
    pub text: String,
    pub path: String,
    pub line: usize,
    /// Same-named fields the write matches, or the count above the profile's cap.
    pub candidates: usize,
}

struct Occurrence<'g> {
    id: &'g str,
    /// Homonyms above the profile cap, when no candidate edge was drawn.
    ambiguous: Option<usize>,
    kind: &'g str,
    label: &'g str,
    owner: &'g str,
    evidence: &'g Evidence,
}

struct Index<'g> {
    parent: BTreeMap<&'g str, &'g str>,
    owned: BTreeMap<&'g str, Vec<usize>>,
    occurrences: Vec<Occurrence<'g>>,
    candidates: BTreeMap<&'g str, Vec<&'g str>>,
}

impl<'g> Index<'g> {
    fn new(graph: &'g Graph) -> Self {
        let mut index = Index { parent: BTreeMap::new(), owned: BTreeMap::new(), occurrences: vec![], candidates: BTreeMap::new() };
        for e in &graph.edges {
            match e.resolution.as_str() {
                "structural" if e.kind == "contains" => {
                    index.parent.insert(&e.to, &e.from);
                }
                "syntax_only" => {
                    let node = &graph.nodes[&e.to];
                    index.owned.entry(&e.from).or_default().push(index.occurrences.len());
                    index.occurrences.push(Occurrence {
                        id: &e.to,
                        ambiguous: node.ambiguous_candidates,
                        kind: &node.kind,
                        label: &node.label,
                        owner: &e.from,
                        evidence: &e.evidence,
                    });
                }
                "name_candidate" => index.candidates.entry(&e.from).or_default().push(&e.to),
                _ => {}
            }
        }
        index
    }

    fn owned(&self, symbol: &str) -> impl Iterator<Item = &Occurrence<'g>> {
        self.owned.get(symbol).into_iter().flatten().map(|i| &self.occurrences[*i])
    }
}

/// How the owner of a method was found.
struct Owner {
    id: String,
    resolution: &'static str,
    evidence: Vec<Evidence>,
}

/// The owner context of each target, in target order. Targets and earlier additions are never
/// added twice.
pub fn owner_context(catalog: &Catalog, graph: &Graph, targets: &[String]) -> Result<OwnerContext> {
    if !catalog.registry.owner_context {
        return Err("Owner context needs an edition of a profile that records owner syntax (v4)".into());
    }
    let index = Index::new(graph);
    let mut context = OwnerContext { added: vec![], not_followed: vec![], not_followed_total: 0, truncated: false, notes: vec![] };
    let mut seen: BTreeSet<String> = targets.iter().cloned().collect();
    let mut listed: BTreeSet<(String, usize)> = BTreeSet::new();
    for target in targets {
        let symbol = &catalog.symbols[target];
        let language = &catalog.sources[&symbol.source_id].language;
        let Some(syntax) = catalog.registry.owner_syntax(language) else { continue };
        if !syntax.method_kinds.contains(&symbol.kind.as_str()) {
            continue;
        }
        let Some(owner) = owner_of(catalog, &index, syntax, target, &mut context.notes) else { continue };
        let fields = fields_of(catalog, syntax, &owner.id);
        let own = format!("{}.", syntax.receiver);
        let read: BTreeSet<String> = index
            .owned(target)
            .filter(|o| o.kind == syntax.read)
            .filter_map(|o| o.label.strip_prefix(own.as_str()))
            .filter(|f| fields.contains_key(*f))
            .map(str::to_owned)
            .collect();
        add(&mut context, &mut seen, catalog, Added {
            symbol_id: owner.id.clone(),
            name: String::new(),
            kind: String::new(),
            reason: "owner_type".into(),
            for_target: symbol.qualified_name.clone(),
            resolution: owner.resolution.into(),
            fields: read.iter().cloned().collect(),
            evidence: owner.evidence,
        });
        // Writers of the read fields, by declaration, in source order.
        let mut writers: BTreeMap<&str, (&str, BTreeSet<String>, Vec<Evidence>)> = BTreeMap::new();
        for o in &index.occurrences {
            let (field, resolution) = if Some(o.kind) == syntax.write {
                let Some((receiver, field)) = o.label.rsplit_once('.') else { continue };
                if !read.contains(field) {
                    continue;
                }
                if receiver.trim() != syntax.receiver {
                    // Another receiver: its type is unknown. Listed when the name may be the
                    // owner's field (a candidate edge reaches it, or there were too many).
                    let edges = index.candidates.get(o.id);
                    let may_be_ours = edges.is_some_and(|c| c.contains(&fields[field])) || o.ambiguous.is_some();
                    if may_be_ours && listed.insert((o.evidence.path.clone(), o.evidence.start_byte)) {
                        context.not_followed.push(NotFollowed {
                            field: field.into(),
                            text: o.label.into(),
                            path: o.evidence.path.clone(),
                            line: o.evidence.start_line,
                            candidates: edges.map_or(o.ambiguous.unwrap_or(0), Vec::len),
                        });
                    }
                    continue;
                }
                if !owner_by_container(catalog, &index, syntax, o.owner).is_some_and(|t| t.id == owner.id) {
                    continue;
                }
                (field, "receiver_in_owner")
            } else if let Some((literal_kind, _, own_type)) = syntax.literal.filter(|l| l.1 == o.kind) {
                if !read.contains(o.label) {
                    continue;
                }
                let Some(literal) = literal_around(&index, literal_kind, o) else { continue };
                let ours = if names(literal.label, own_type) {
                    owner_by_container(catalog, &index, syntax, o.owner).is_some_and(|t| t.id == owner.id)
                } else {
                    index.candidates.get(literal.id).is_some_and(|c| c.contains(&owner.id.as_str()))
                };
                if !ours {
                    continue;
                }
                (o.label, "owner_literal")
            } else {
                continue;
            };
            if o.owner == target {
                continue;
            }
            let entry = writers.entry(o.owner).or_insert_with(|| (resolution, BTreeSet::new(), vec![]));
            entry.1.insert(field.to_owned());
            entry.2.push(o.evidence.clone());
        }
        let mut writers: Vec<_> = writers.into_iter().collect();
        writers.sort_by_key(|(id, _)| {
            let s = &catalog.symbols[*id];
            (catalog.sources[&s.source_id].path.clone(), catalog.chunks[&s.chunk_id].start_line)
        });
        for (writer, (resolution, written, mut evidence)) in writers {
            evidence.sort_by_key(|e| e.start_byte);
            add(&mut context, &mut seen, catalog, Added {
                symbol_id: writer.into(),
                name: String::new(),
                kind: String::new(),
                reason: "field_writer".into(),
                for_target: symbol.qualified_name.clone(),
                resolution: resolution.into(),
                fields: written.into_iter().collect(),
                evidence,
            });
        }
    }
    context.not_followed_total = context.not_followed.len();
    context.not_followed.sort_by(|a, b| (&a.path, a.line, &a.text).cmp(&(&b.path, b.line, &b.text)));
    context.not_followed.truncate(MAX_LISTED);
    Ok(context)
}

/// P5: the owner type of a method, by its container (the container itself, or the only type its
/// written name matches), with where that name is written. None for other declarations.
pub fn owner_type(catalog: &Catalog, graph: &Graph, method: &str) -> Option<(String, Vec<Evidence>)> {
    if !catalog.registry.owner_context {
        return None;
    }
    let symbol = catalog.symbols.get(method)?;
    let syntax = catalog.registry.owner_syntax(&catalog.sources[&symbol.source_id].language)?;
    if !syntax.method_kinds.contains(&symbol.kind.as_str()) {
        return None;
    }
    owner_by_container(catalog, &Index::new(graph), syntax, method).map(|o| (o.id, o.evidence))
}

fn add(context: &mut OwnerContext, seen: &mut BTreeSet<String>, catalog: &Catalog, mut added: Added) {
    if !seen.insert(added.symbol_id.clone()) {
        return;
    }
    if context.added.len() >= MAX_ADDED {
        context.truncated = true;
        return;
    }
    let symbol = &catalog.symbols[&added.symbol_id];
    added.name = symbol.qualified_name.clone();
    added.kind = symbol.kind.clone();
    context.added.push(added);
}

/// Owner of a method, with a note when its container names a type that cannot be told apart.
fn owner_of(catalog: &Catalog, index: &Index, syntax: &OwnerSyntax, method: &str, notes: &mut Vec<String>) -> Option<Owner> {
    if let Some(owner) = owner_by_container(catalog, index, syntax, method) {
        return Some(owner);
    }
    let parent = *index.parent.get(method)?;
    let (container, naming) = syntax.naming_container?;
    if catalog.symbols[parent].kind != container {
        return None;
    }
    let written = index.owned(parent).find(|o| o.kind == naming)?;
    let count = named_types(catalog, syntax, parent, written.label).1;
    notes.push(format!(
        "{}: o bloco que o contém escreve o tipo `{}`, com {count} declarações com esse nome; nenhuma acrescentada.",
        catalog.symbols[method].qualified_name, written.label
    ));
    None
}

/// The owner of a declaration by its container: the container itself, or the only type its
/// written name matches in the container's file, else in its language.
fn owner_by_container(catalog: &Catalog, index: &Index, syntax: &OwnerSyntax, symbol: &str) -> Option<Owner> {
    let parent = *index.parent.get(symbol)?;
    let kind = catalog.symbols[parent].kind.as_str();
    if syntax.owner_containers.contains(&kind) {
        return Some(Owner { id: parent.into(), resolution: "container", evidence: vec![] });
    }
    let (container, naming) = syntax.naming_container?;
    if kind != container {
        return None;
    }
    let written = index.owned(parent).find(|o| o.kind == naming)?;
    let (found, _) = named_types(catalog, syntax, parent, written.label);
    Some(Owner { id: found?, resolution: "container_name", evidence: vec![written.evidence.clone()] })
}

/// Fields declared inside the owner and not inside another type nested in it. A field may sit
/// in a method of the owner (one assigned in a constructor), so containment is by bytes.
fn fields_of<'c>(catalog: &'c Catalog, syntax: &OwnerSyntax, owner: &str) -> BTreeMap<String, &'c str> {
    let owner = &catalog.symbols[owner];
    let span = |s: &crate::Symbol| {
        let c = &catalog.chunks[&s.chunk_id];
        (c.start_byte, c.end_byte)
    };
    let (start, end) = span(owner);
    let inside: Vec<_> = catalog
        .symbols
        .values()
        .filter(|s| s.source_id == owner.source_id && s.id != owner.id)
        .filter(|s| {
            let (a, b) = span(s);
            start <= a && b <= end
        })
        .collect();
    let types: Vec<_> = inside
        .iter()
        .filter(|s| syntax.owner_containers.contains(&s.kind.as_str()) || syntax.owner_kinds.contains(&s.kind.as_str()))
        .map(|s| span(s))
        .collect();
    inside
        .iter()
        .filter(|s| s.kind == "field")
        .filter(|s| {
            let (a, b) = span(s);
            !types.iter().any(|(ta, tb)| *ta <= a && b <= *tb)
        })
        .map(|s| (s.name.clone(), s.id.as_str()))
        .collect()
}

/// The type a container's written name points to, and how many types have that name.
fn named_types(catalog: &Catalog, syntax: &OwnerSyntax, container: &str, name: &str) -> (Option<String>, usize) {
    let source = &catalog.symbols[container].source_id;
    let language = &catalog.sources[source].language;
    let found: Vec<_> = catalog
        .symbols_named(name)
        .into_iter()
        .filter(|s| syntax.owner_kinds.contains(&s.kind.as_str()))
        .filter(|s| catalog.sources[&s.source_id].language == *language)
        .collect();
    let local: Vec<_> = found.iter().filter(|s| s.source_id == *source).collect();
    let chosen = match (local.as_slice(), found.as_slice()) {
        ([one], _) => Some(one.id.clone()),
        ([], [one]) => Some(one.id.clone()),
        _ => None,
    };
    (chosen, found.len())
}

/// The innermost literal of the same declaration whose bytes contain a filled field.
fn literal_around<'a, 'g>(index: &'a Index<'g>, kind: &str, field: &Occurrence) -> Option<&'a Occurrence<'g>> {
    index
        .owned(field.owner)
        .filter(|o| o.kind == kind)
        .filter(|o| o.evidence.start_byte <= field.evidence.start_byte && field.evidence.end_byte <= o.evidence.end_byte)
        .min_by_key(|o| o.evidence.end_byte - o.evidence.start_byte)
}

/// Whether a literal's text starts with the written name `name` as a whole word.
fn names(literal: &str, name: &str) -> bool {
    literal.strip_prefix(name).is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
}

/// P4: the declarations a target calls, added to a chain step so that a change to the target is
/// written with what they do in view. Only calls the "what does it call?" traversal follows
/// without a person's choice: the one declaration of that name (`unique`), or the one in the
/// target's own container reached through its receiver (`receiver`). Homonyms are not added.
/// At most `room` declarations; the rest is reported as truncated.
pub fn called(catalog: &Catalog, targets: &[String], already: &[String], room: usize) -> Result<(Vec<Added>, bool)> {
    let graph = crate::graph::v2::Graph::build(catalog);
    let syntax = catalog.call_syntax();
    let (mut added, mut truncated): (Vec<Added>, bool) = (vec![], false);
    for target in targets {
        let calls = graph.calls_from(target, 1, 64, &syntax, &BTreeMap::new())?;
        let qualified = catalog.symbols.get(target).map_or_else(|| target.clone(), |s| s.qualified_name.clone());
        for link in calls.links.iter().filter(|l| &l.from == target && matches!(l.resolution.as_str(), "unique" | "receiver")) {
            if already.contains(&link.to) {
                continue;
            }
            if let Some(known) = added.iter_mut().find(|a| a.symbol_id == link.to) {
                known.evidence.push(link.evidence.clone());
                continue;
            }
            let Some(symbol) = catalog.symbols.get(&link.to) else { continue };
            if added.len() >= room {
                truncated = true;
                continue;
            }
            added.push(Added {
                symbol_id: link.to.clone(), name: symbol.name.clone(), kind: symbol.kind.clone(), reason: "called".into(),
                for_target: qualified.clone(), resolution: link.resolution.clone(), fields: vec![], evidence: vec![link.evidence.clone()],
            });
        }
    }
    Ok((added, truncated))
}
