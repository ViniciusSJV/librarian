//! Steps 2 and 3 of the road to investigation: the operation inventory. The graph is walked
//! for operations worth asking about, each found by a declared rule over written names: a
//! database call, a lazy relationship read, math, files, network, a lock, a module write. And
//! the loops they sit in, directly or through a chain of calls. Nothing here says an operation
//! is slow or a cause: being inside a loop is structure; how often and how long needs a
//! measurement.
use crate::{
    graph::{v2, Evidence},
    hash, Catalog, Result,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const VERSION: &str = "operation-inventory-v1";
/// How many calls a chain from a loop follows to reach an operation.
pub const MAX_CHAIN: usize = 4;

/// Rules of one project, or the built-in ones. Each finds operations by written names only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    /// What the operation is, as the report groups it: database, math, file, network...
    pub operation: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Only occurrences in this language; absent, every language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(flatten)]
    pub matcher: Matcher,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "match", rename_all = "snake_case", deny_unknown_fields)]
pub enum Matcher {
    /// A call whose written name is one of `names`. With `receivers`, the name written just
    /// before it (`session` in `self.session.exec(...)`, `File` in `File::open`) must be one of
    /// them; `""` stands for a call with no receiver (`open(...)`).
    Call {
        names: Vec<String>,
        #[serde(default)]
        receivers: Vec<String>,
    },
    /// A read or write of a field whose declaration calls one of `calls` (`Relationship(...)`).
    FieldDeclaredWith { calls: Vec<String> },
    /// A write of a module name (`global x; x = ...`, `x[k] = v`).
    ModuleWrite {},
}

/// Generic rules by language: what a name says in any project. Database rules depend on the
/// project's libraries and come from its own rules.
const BUILTIN: &str = r#"{ "rules": [
  { "id": "math.call", "operation": "math", "description": "Função matemática da biblioteca padrão",
    "match": "call", "names": ["sqrt", "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "radians", "degrees", "hypot", "exp", "log", "pow"],
    "receivers": ["", "math", "np", "numpy"], "language": "python" },
  { "id": "math.method", "operation": "math", "description": "Método matemático de número",
    "match": "call", "names": ["sqrt", "powi", "powf", "sin", "cos", "tan", "asin", "acos", "atan2", "hypot", "exp", "ln", "abs"], "language": "rust" },
  { "id": "file.open", "operation": "file", "description": "Abre ou lê arquivo",
    "match": "call", "names": ["open", "read_text", "write_text", "read_bytes", "write_bytes"], "language": "python" },
  { "id": "file.fs", "operation": "file", "description": "Abre ou lê arquivo",
    "match": "call", "names": ["open", "create", "read_to_string", "read", "write"], "receivers": ["File", "fs"], "language": "rust" },
  { "id": "network.http", "operation": "network", "description": "Requisição HTTP",
    "match": "call", "names": ["get", "post", "put", "patch", "delete", "request"], "receivers": ["requests", "httpx", "client"], "language": "python" },
  { "id": "network.urlopen", "operation": "network", "description": "Abre uma URL",
    "match": "call", "names": ["urlopen"], "language": "python" },
  { "id": "process.sleep", "operation": "sleep", "description": "Espera parada",
    "match": "call", "names": ["sleep"], "receivers": ["time", "asyncio", "thread", ""] },
  { "id": "process.subprocess", "operation": "process", "description": "Processo externo",
    "match": "call", "names": ["run", "Popen", "check_output", "check_call", "call"], "receivers": ["subprocess"], "language": "python" },
  { "id": "lock.python", "operation": "lock", "description": "Cria ou toma uma trava",
    "match": "call", "names": ["Lock", "RLock", "Semaphore", "BoundedSemaphore", "acquire"], "language": "python" },
  { "id": "lock.rust", "operation": "lock", "description": "Toma uma trava",
    "match": "call", "names": ["lock"], "language": "rust" },
  { "id": "state.module_write", "operation": "state", "description": "Escreve um nome do módulo",
    "match": "module_write" }
] }"#;

pub fn builtin() -> RuleSet {
    serde_json::from_str(BUILTIN).expect("built-in rules parse")
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Inventory {
    pub version: String,
    pub profile: String,
    pub graph_version: String,
    pub catalog_sha256: String,
    /// Every rule applied, where it came from and how many operations it found.
    pub rules: Vec<RuleSummary>,
    pub operations: Vec<Operation>,
    /// The declarations that hold operations, with the calls between them.
    pub symbols: Vec<SymbolOperations>,
    /// Loops with an operation inside, directly or through calls, or with a loop inside.
    pub loops: Vec<LoopFinding>,
    /// Loops with none of that, counted, not listed.
    pub loops_without_operations: usize,
    /// Calls that no rule matched: outside the inventory, counted.
    pub calls_unmatched: usize,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuleSummary {
    pub id: String,
    pub operation: String,
    pub description: String,
    /// builtin or project.
    pub source: String,
    pub operations: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Operation {
    /// `op:` plus the hash of the occurrence and the rule: stable for the same edition.
    pub id: String,
    pub operation: String,
    pub rule: String,
    /// The declaration whose body writes it.
    pub symbol: String,
    pub symbol_name: String,
    pub path: String,
    pub line: usize,
    pub end_line: usize,
    /// The written text, at most 160 bytes.
    pub text: String,
    pub occurrence: String,
    pub occurrence_kind: String,
    /// For a field or module rule: the declarations the written name matched, all candidates.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub through: Vec<String>,
    /// Whether it is inside a loop of its own declaration.
    pub in_loop: bool,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SymbolOperations {
    pub symbol: String,
    pub name: String,
    pub path: String,
    pub line: usize,
    /// Operation ids, in the order written.
    pub operations: Vec<String>,
    /// Declarations with operations that this one calls (only calls followed without
    /// homonyms), and those that call it.
    pub calls: Vec<String>,
    pub called_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LoopFinding {
    pub occurrence: String,
    pub symbol: String,
    pub symbol_name: String,
    pub path: String,
    pub line: usize,
    pub end_line: usize,
    /// The loop's head (`for d in deliveries`).
    pub head: String,
    /// Loops of the same declaration around this one.
    pub nesting: usize,
    /// Loops of the same declaration inside this one.
    pub inner_loops: usize,
    /// Operations written inside the loop itself.
    pub direct: Vec<String>,
    /// Operations reached through calls written inside the loop.
    pub through_calls: Vec<Chain>,
    /// Names called more than once inside the loop, with the lines.
    pub repeated_calls: Vec<Repeated>,
    /// Calls inside the loop not followed because their name matches several declarations.
    pub unfollowed: Vec<Unfollowed>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Chain {
    pub operation: String,
    /// From the loop's declaration to the one that holds the operation.
    pub steps: Vec<Step>,
    /// Whether the operation is also inside a loop of its own declaration: a loop in a loop.
    pub inner_loop: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Step {
    pub from: String,
    pub to: String,
    pub to_name: String,
    pub text: String,
    pub path: String,
    pub line: usize,
    /// unique or receiver, as in "what does it call?".
    pub resolution: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Repeated {
    pub name: String,
    pub lines: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Unfollowed {
    pub text: String,
    pub line: usize,
    pub candidates: usize,
}

const LIMITS: &[&str] = &[
    "Operações são achadas por regras declaradas sobre nomes escritos; o que nenhuma regra cobre fica fora e é contado (calls_unmatched).",
    "Uma operação dentro de um laço é estrutura: não diz quantas vezes o laço roda, nem quanto tempo a operação leva. Isso exige medição.",
    "Cadeias seguem só chamadas sem homônimos (únicas ou do próprio receptor), até 4 chamadas; as outras são listadas em unfollowed.",
    "Receptores não são resolvidos: `session` é o nome escrito antes da chamada, não o tipo do objeto.",
    "Um campo achado por field_declared_with casa pelo nome com todos os campos homônimos que a regra marcou (through).",
];

/// Indexes over the v2 graph, built once.
struct Index<'g> {
    graph: &'g v2::Graph,
    observed: BTreeMap<&'g str, Vec<&'g v2::Edge>>,
    candidates: BTreeMap<&'g str, Vec<&'g str>>,
    parent: BTreeMap<&'g str, &'g str>,
    declared: BTreeMap<&'g str, &'g Evidence>,
}

impl<'g> Index<'g> {
    fn new(graph: &'g v2::Graph) -> Self {
        let mut index = Self { graph, observed: BTreeMap::new(), candidates: BTreeMap::new(), parent: BTreeMap::new(), declared: BTreeMap::new() };
        for e in &graph.edges {
            match e.kind.as_str() {
                "observes" => index.observed.entry(&e.from).or_default().push(e),
                "name_candidate" => index.candidates.entry(&e.from).or_default().push(&e.to),
                "contains" => {
                    index.parent.insert(&e.to, &e.from);
                }
                "declares" => {
                    index.declared.insert(&e.to, &e.evidence);
                }
                _ => {}
            }
        }
        index
    }

    fn label(&self, id: &str) -> &'g str {
        &self.graph.nodes[id].label
    }

    fn kind(&self, id: &str) -> &'g str {
        &self.graph.nodes[id].kind
    }

    fn place(&self, symbol: &str) -> (String, usize) {
        self.declared.get(symbol).map_or((String::new(), 0), |e| (e.path.clone(), e.start_line))
    }
}

/// The written name of a call and the name written just before it, from the call's text:
/// `self.session.exec(s)` -> (`session`, `exec`), `File::open` -> (`File`, `open`),
/// `open` -> (``, `open`). None when the text ends in no name.
pub fn call_parts(text: &str) -> Option<(String, String)> {
    let mut head = text.trim_end();
    // A method call's text has its arguments: drop the last balanced parentheses.
    if head.ends_with(')') {
        head = &head[..matching_open(head, '(', ')')?];
    }
    // Turbofish: `collect::<Vec<_>>`.
    if head.ends_with('>') {
        head = head[..matching_open(head, '<', '>')?].trim_end_matches("::");
    }
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

fn matching_open(text: &str, open: char, close: char) -> Option<usize> {
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

fn trailing_ident(text: &str) -> Option<&str> {
    let start = text.char_indices().rev().take_while(|(_, c)| c.is_alphanumeric() || *c == '_').last().map(|(i, _)| i)?;
    Some(&text[start..])
}

fn short(text: &str) -> String {
    let mut end = text.len().min(160);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

fn inside(outer: &Evidence, inner: &Evidence) -> bool {
    outer.source_id == inner.source_id && outer.start_byte <= inner.start_byte && inner.end_byte <= outer.end_byte
}

/// Checks a project's rules: ids unique and not built-in, names present.
pub fn check(rules: &RuleSet, builtin: &RuleSet) -> Result<()> {
    let mut seen: BTreeSet<&str> = builtin.rules.iter().map(|r| r.id.as_str()).collect();
    for rule in &rules.rules {
        if rule.id.trim().is_empty() || rule.operation.trim().is_empty() {
            return Err("Each rule needs an id and an operation".into());
        }
        if !seen.insert(&rule.id) {
            return Err(format!("Rule {} is listed twice or repeats a built-in id", rule.id).into());
        }
        let empty = match &rule.matcher {
            Matcher::Call { names, .. } => names.is_empty(),
            Matcher::FieldDeclaredWith { calls } => calls.is_empty(),
            Matcher::ModuleWrite {} => false,
        };
        if empty {
            return Err(format!("Rule {} matches no name", rule.id).into());
        }
    }
    Ok(())
}

/// The inventory of an edition with the built-in rules and the project's own.
pub fn inventory(catalog: &Catalog, project: &RuleSet) -> Result<Inventory> {
    let builtin = builtin();
    check(project, &builtin)?;
    let graph = v2::Graph::build(catalog);
    let index = Index::new(&graph);
    let syntax = catalog.call_syntax();
    let rules: Vec<(&Rule, &str)> = builtin.rules.iter().map(|r| (r, "builtin")).chain(project.rules.iter().map(|r| (r, "project"))).collect();

    // Fields whose declaration calls what a field rule names, by rule.
    let field_rules: Vec<(usize, BTreeSet<&str>)> = rules
        .iter()
        .enumerate()
        .filter_map(|(i, (rule, _))| match &rule.matcher {
            Matcher::FieldDeclaredWith { calls } => {
                let fields = catalog
                    .symbols()
                    .values()
                    .filter(|s| s.kind == "field")
                    .filter(|s| {
                        index.observed.get(s.id.as_str()).into_iter().flatten().any(|e| {
                            index.kind(&e.to) == "call_observed" && call_parts(index.label(&e.to)).is_some_and(|(_, n)| calls.contains(&n))
                        })
                    })
                    .map(|s| s.id.as_str())
                    .collect();
                Some((i, fields))
            }
            _ => None,
        })
        .collect();

    // Loops by declaration, to tell whether an occurrence is inside one.
    let mut loops_of: BTreeMap<&str, Vec<&v2::Edge>> = BTreeMap::new();
    for (symbol, edges) in &index.observed {
        for e in edges.iter().filter(|e| index.kind(&e.to) == "loop") {
            loops_of.entry(symbol).or_default().push(e);
        }
    }
    let in_loop = |symbol: &str, evidence: &Evidence| loops_of.get(symbol).into_iter().flatten().any(|l| inside(&l.evidence, evidence) && l.evidence != *evidence);

    let mut operations = vec![];
    let mut counts = vec![0usize; rules.len()];
    let mut calls_unmatched = 0;
    for (symbol, edges) in &index.observed {
        if graph.nodes[*symbol].category != "symbol" {
            continue;
        }
        let language = graph.nodes[*symbol].language.as_str();
        let call_kinds = syntax.languages.get(language).map(|(k, _)| k.clone()).unwrap_or_default();
        for edge in edges {
            let occurrence = edge.to.as_str();
            let kind = index.kind(occurrence);
            let text = index.label(occurrence);
            let is_call = call_kinds.contains(&kind);
            let parts = is_call.then(|| call_parts(text)).flatten();
            let mut matched = false;
            for (i, (rule, _)) in rules.iter().enumerate() {
                if rule.language.as_deref().is_some_and(|l| l != language) {
                    continue;
                }
                let through: Option<Vec<String>> = match &rule.matcher {
                    Matcher::Call { names, receivers } => parts.as_ref().and_then(|(receiver, name)| {
                        (names.contains(name) && (receivers.is_empty() || receivers.contains(receiver))).then(Vec::new)
                    }),
                    Matcher::FieldDeclaredWith { .. } if kind == "attribute_access" => {
                        let fields = &field_rules.iter().find(|(r, _)| *r == i).expect("field rule indexed").1;
                        let hit: Vec<String> = index.candidates.get(occurrence).into_iter().flatten().filter(|c| fields.contains(**c)).map(|c| c.to_string()).collect();
                        (!hit.is_empty()).then_some(hit)
                    }
                    Matcher::ModuleWrite {} if kind == "name_write" => {
                        let hit: Vec<String> = index.candidates.get(occurrence).into_iter().flatten().map(|c| c.to_string()).collect();
                        (!hit.is_empty()).then_some(hit)
                    }
                    _ => None,
                };
                let Some(through) = through else { continue };
                matched = true;
                counts[i] += 1;
                let (path, line) = (edge.evidence.path.clone(), edge.evidence.start_line);
                let id = format!("op:{}", &hash(format!("{occurrence}|{}", rule.id).as_bytes())[..16]);
                operations.push(Operation {
                    id,
                    operation: rule.operation.clone(),
                    rule: rule.id.clone(),
                    symbol: symbol.to_string(),
                    symbol_name: index.label(symbol).to_owned(),
                    path,
                    line,
                    end_line: edge.evidence.end_line,
                    text: short(text),
                    occurrence: occurrence.to_owned(),
                    occurrence_kind: kind.to_owned(),
                    through,
                    in_loop: in_loop(symbol, &edge.evidence),
                    evidence: edge.evidence.clone(),
                });
            }
            if is_call && !matched {
                calls_unmatched += 1;
            }
        }
    }
    operations.sort_by(|a, b| (&a.path, a.evidence.start_byte, &a.rule).cmp(&(&b.path, b.evidence.start_byte, &b.rule)));

    // Calls followed without homonyms, the rule of "what does it call?".
    let container = |id: &str| index.parent.get(id).copied();
    let follow = |symbol: &str, occurrence: &str| -> std::result::Result<(String, &'static str), usize> {
        let edges = index.candidates.get(occurrence).cloned().unwrap_or_default();
        if edges.len() == 1 {
            return Ok((edges[0].to_owned(), "unique"));
        }
        let receiver = syntax.languages.get(&graph.nodes[symbol].language).and_then(|(_, r)| *r);
        let own = receiver.is_some_and(|r| index.label(occurrence).starts_with(&format!("{r}.")));
        let same: Vec<&&str> = edges.iter().filter(|e| container(e) == container(symbol)).collect();
        if own && same.len() == 1 {
            return Ok((same[0].to_string(), "receiver"));
        }
        Err(edges.len())
    };
    let calls_of = |symbol: &str| -> Vec<&v2::Edge> {
        let language = &graph.nodes[symbol].language;
        let kinds = syntax.languages.get(language).map(|(k, _)| k.clone()).unwrap_or_default();
        index.observed.get(symbol).into_iter().flatten().filter(|e| kinds.contains(&index.kind(&e.to))).copied().collect()
    };

    let mut by_symbol: BTreeMap<&str, Vec<&Operation>> = BTreeMap::new();
    for op in &operations {
        by_symbol.entry(op.symbol.as_str()).or_default().push(op);
    }
    let mut symbols: Vec<SymbolOperations> = by_symbol
        .iter()
        .map(|(symbol, ops)| {
            let (path, line) = index.place(symbol);
            let mut calls: Vec<String> = calls_of(symbol)
                .iter()
                .filter_map(|e| follow(symbol, &e.to).ok())
                .map(|(to, _)| to)
                .filter(|to| by_symbol.contains_key(to.as_str()) && to != symbol)
                .collect();
            calls.sort();
            calls.dedup();
            SymbolOperations { symbol: symbol.to_string(), name: index.label(symbol).to_owned(), path, line, operations: ops.iter().map(|o| o.id.clone()).collect(), calls, called_by: vec![] }
        })
        .collect();
    let callers: Vec<(String, String)> = symbols.iter().flat_map(|s| s.calls.iter().map(|c| (c.clone(), s.symbol.clone()))).collect();
    for s in &mut symbols {
        s.called_by = callers.iter().filter(|(to, _)| *to == s.symbol).map(|(_, from)| from.clone()).collect();
    }

    // Step 3: what each loop holds, directly or through calls.
    let mut loops = vec![];
    let mut loops_without_operations = 0;
    for (symbol, symbol_loops) in &loops_of {
        if graph.nodes[*symbol].category != "symbol" {
            continue;
        }
        let calls = calls_of(symbol);
        for l in symbol_loops {
            let (start, end) = (l.evidence.start_byte, l.evidence.end_byte);
            let nesting = symbol_loops.iter().filter(|o| o.to != l.to && inside(&o.evidence, &l.evidence)).count();
            let inner_loops = symbol_loops.iter().filter(|o| o.to != l.to && inside(&l.evidence, &o.evidence)).count();
            let direct: Vec<String> = by_symbol.get(symbol).into_iter().flatten().filter(|o| start <= o.evidence.start_byte && o.evidence.end_byte <= end).map(|o| o.id.clone()).collect();
            let inner_calls: Vec<&&v2::Edge> = calls.iter().filter(|c| inside(&l.evidence, &c.evidence)).collect();
            let mut through_calls = vec![];
            let mut unfollowed = vec![];
            let mut repeated: BTreeMap<String, Vec<usize>> = BTreeMap::new();
            for call in &inner_calls {
                if let Some((_, name)) = call_parts(index.label(&call.to)) {
                    repeated.entry(name).or_default().push(call.evidence.start_line);
                }
                match follow(symbol, &call.to) {
                    Ok((to, resolution)) => {
                        let first = step(&index, symbol, &to, call, resolution);
                        through_calls.extend(reach(&index, &by_symbol, &follow, &calls_of, first, &in_loop));
                    }
                    Err(candidates) if candidates > 1 => unfollowed.push(Unfollowed { text: short(index.label(&call.to)), line: call.evidence.start_line, candidates }),
                    Err(_) => {}
                }
            }
            // One chain per operation: the shortest, then the first written.
            let mut seen = BTreeSet::new();
            through_calls.sort_by_key(|c: &Chain| c.steps.len());
            through_calls.retain(|c| seen.insert(c.operation.clone()));
            let repeated_calls: Vec<Repeated> = repeated
                .into_iter()
                .filter(|(_, lines)| lines.len() > 1)
                .map(|(name, mut lines)| {
                    lines.sort();
                    Repeated { name, lines }
                })
                .collect();
            if direct.is_empty() && through_calls.is_empty() && inner_loops == 0 {
                loops_without_operations += 1;
                continue;
            }
            loops.push(LoopFinding {
                occurrence: l.to.clone(),
                symbol: symbol.to_string(),
                symbol_name: index.label(symbol).to_owned(),
                path: l.evidence.path.clone(),
                line: l.evidence.start_line,
                end_line: l.evidence.end_line,
                head: short(index.label(&l.to)),
                nesting,
                inner_loops,
                direct,
                through_calls,
                repeated_calls,
                unfollowed,
            });
        }
    }
    loops.sort_by(|a, b| (&a.path, a.line, &a.occurrence).cmp(&(&b.path, b.line, &b.occurrence)));

    Ok(Inventory {
        version: VERSION.into(),
        profile: catalog.manifest().extractor.clone(),
        graph_version: graph.derived_from.clone(),
        catalog_sha256: graph.catalog_sha256.clone(),
        rules: rules
            .iter()
            .zip(counts)
            .map(|((rule, source), operations)| RuleSummary {
                id: rule.id.clone(),
                operation: rule.operation.clone(),
                description: rule.description.clone(),
                source: source.to_string(),
                operations,
            })
            .collect(),
        operations,
        symbols,
        loops,
        loops_without_operations,
        calls_unmatched,
        limits: LIMITS.iter().map(|l| l.to_string()).collect(),
    })
}

/// The inventory as text for a terminal: operations by declaration, then the loops, each
/// with `path:line` to navigate.
pub fn summary(inventory: &Inventory) -> String {
    use std::fmt::Write;
    let ops: BTreeMap<&str, &Operation> = inventory.operations.iter().map(|o| (o.id.as_str(), o)).collect();
    let names: BTreeMap<&str, &str> = inventory.symbols.iter().map(|s| (s.symbol.as_str(), s.name.as_str())).collect();
    // Written text on one line: a call can span several.
    let line = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    let _ = writeln!(out, "Operações: {} em {} declarações; laços com operação: {} (sem: {}); chamadas sem regra: {}",
        inventory.operations.len(), inventory.symbols.len(), inventory.loops.len(), inventory.loops_without_operations, inventory.calls_unmatched);
    for s in &inventory.symbols {
        let _ = writeln!(out, "\n{}  {}:{}", s.name, s.path, s.line);
        for id in &s.operations {
            let o = ops[id.as_str()];
            let _ = writeln!(out, "  {:<9} {:<22} {}:{}  {}{}", o.operation, o.rule, o.path, o.line, line(&o.text),
                if o.in_loop { "  [em laço]" } else { "" });
        }
        if !s.calls.is_empty() {
            let _ = writeln!(out, "  chama: {}", s.calls.iter().map(|c| names.get(c.as_str()).copied().unwrap_or(c)).collect::<Vec<_>>().join(", "));
        }
        if !s.called_by.is_empty() {
            let _ = writeln!(out, "  chamada por: {}", s.called_by.iter().map(|c| names.get(c.as_str()).copied().unwrap_or(c)).collect::<Vec<_>>().join(", "));
        }
    }
    if !inventory.loops.is_empty() {
        let _ = writeln!(out, "\nLaços");
    }
    for l in &inventory.loops {
        let _ = writeln!(out, "\n{}  {}:{}–{}  {}{}", l.symbol_name, l.path, l.line, l.end_line, line(&l.head),
            match (l.nesting, l.inner_loops) {
                (0, 0) => String::new(),
                (n, i) => format!("  [dentro de {n} laço(s); {i} laço(s) dentro]"),
            });
        for id in &l.direct {
            let o = ops[id.as_str()];
            let _ = writeln!(out, "  direto: {} {}:{}  {}", o.operation, o.path, o.line, line(&o.text));
        }
        // The operations a chain reaches in one declaration are one line: the chain, then each
        // kind of operation with how many and where.
        let mut chains: Vec<(String, BTreeMap<&str, Vec<&Operation>>, bool)> = vec![];
        for c in &l.through_calls {
            let o = ops[c.operation.as_str()];
            let path: Vec<String> = c.steps.iter().map(|s| format!("{} ({}:{})", s.to_name, s.path, s.line)).collect();
            let path = path.join(" → ");
            match chains.iter_mut().find(|(p, _, _)| *p == path) {
                Some((_, kinds, inner)) => {
                    kinds.entry(o.operation.as_str()).or_default().push(o);
                    *inner |= c.inner_loop;
                }
                None => chains.push((path, BTreeMap::from([(o.operation.as_str(), vec![o])]), c.inner_loop)),
            }
        }
        for (path, kinds, inner) in chains {
            let reached: Vec<String> = kinds
                .iter()
                .map(|(kind, found)| match found.as_slice() {
                    [o] => format!("{kind} {}:{}  {}", o.path, o.line, line(&o.text)),
                    many => format!("{kind} ×{} {}:{}–{}", many.len(), many[0].path, many[0].line, many[many.len() - 1].line),
                })
                .collect();
            let _ = writeln!(out, "  pela cadeia: {path} → {}{}", reached.join("; "), if inner { "  [em laço lá também]" } else { "" });
        }
        for r in &l.repeated_calls {
            let _ = writeln!(out, "  repetida: {} × {} (linhas {})", r.name, r.lines.len(), r.lines.iter().map(usize::to_string).collect::<Vec<_>>().join(", "));
        }
        for u in &l.unfollowed {
            let _ = writeln!(out, "  não seguida: {} (linha {}, {} homônimos)", line(&u.text), u.line, u.candidates);
        }
    }
    out
}

fn step(index: &Index, from: &str, to: &str, call: &v2::Edge, resolution: &str) -> Step {
    Step {
        from: from.to_owned(),
        to: to.to_owned(),
        to_name: index.label(to).to_owned(),
        text: short(index.label(&call.to)),
        path: call.evidence.path.clone(),
        line: call.evidence.start_line,
        resolution: resolution.into(),
    }
}

/// Every operation reachable from a first call, breadth first, up to MAX_CHAIN calls, with the
/// steps that lead to it. A declaration is entered once.
fn reach<'g>(
    index: &Index<'g>,
    by_symbol: &BTreeMap<&str, Vec<&Operation>>,
    follow: &dyn Fn(&str, &str) -> std::result::Result<(String, &'static str), usize>,
    calls_of: &dyn Fn(&str) -> Vec<&'g v2::Edge>,
    first: Step,
    in_loop: &dyn Fn(&str, &Evidence) -> bool,
) -> Vec<Chain> {
    let mut found = vec![];
    let mut visited = BTreeSet::from([first.from.clone()]);
    let mut queue = VecDeque::from([vec![first]]);
    while let Some(steps) = queue.pop_front() {
        let here = steps.last().expect("a chain has a step").to.clone();
        if !visited.insert(here.clone()) {
            continue;
        }
        for op in by_symbol.get(here.as_str()).into_iter().flatten() {
            found.push(Chain { operation: op.id.clone(), steps: steps.clone(), inner_loop: in_loop(&here, &op.evidence) });
        }
        if steps.len() >= MAX_CHAIN {
            continue;
        }
        for call in calls_of(&here) {
            if let Ok((to, resolution)) = follow(&here, &call.to) {
                if !visited.contains(&to) {
                    let mut next = steps.clone();
                    next.push(step(index, &here, &to, call, resolution));
                    queue.push_back(next);
                }
            }
        }
    }
    found
}
