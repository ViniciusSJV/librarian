//! Steps 2 and 3: the operation inventory. Operations found by declared rules over written
//! names, each with its rule, line and evidence, the calls between their declarations, and
//! the loops that hold them, directly or through a chain of calls without homonyms.
use librarian_ingest::{
    generate,
    inventory::{self, call_parts, Inventory, RuleSet},
    Catalog, Config,
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

// The N+1 of the logistics API, small: a lazy relationship read through two calls in loops.
const MODELS: &str = "from sqlmodel import Relationship, SQLModel


class Area(SQLModel, table=True):
    zone: str = ''


class Address(SQLModel, table=True):
    street: str = ''
    area: Area | None = Relationship()


class Delivery(SQLModel, table=True):
    weight: float = 0
    address: Address | None = Relationship()
";

const SERVICE: &str = "from math import sqrt

_seen = {}


def list_for_day(session, day):
    return session.exec(day).all()


def address_of(delivery):
    return delivery.address


def zone_of(address):
    return address.area.zone


def distance(a, b):
    return sqrt(a * a + b * b)


def plan(session, day):
    deliveries = list_for_day(session, day)
    addresses = [address_of(d) for d in deliveries]
    total = 0
    for a in addresses:
        for b in addresses:
            total += distance(a, b) + distance(b, a)
        _seen[a] = zone_of(a) or heavy(addresses)
    return total, deliveries[0].weight


def heavy(addresses):
    return sum(1 for x in addresses if x)


def weights(session, day):
    return [d.weight for d in list_for_day(session, day)]
";

const RUST: &str = "pub struct P {
    pub x: f64,
}

pub fn norm(points: &[P]) -> f64 {
    let mut total = 0.0;
    for p in points {
        total += p.x.sqrt();
    }
    total
}
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-inventory-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("app")).unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("app/models.py"), MODELS).unwrap();
        fs::write(dir.join("app/service.py"), SERVICE).unwrap();
        fs::write(dir.join("src/norm.rs"), RUST).unwrap();
        Self(dir)
    }

    fn edition(&self) -> Catalog {
        let path = self.0.join("edition");
        let config = Config { project: "inventory".into(), inputs: vec!["app".into(), "src".into()] };
        generate(&self.0, &config, &path).unwrap();
        Catalog::load(&path).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn rules() -> RuleSet {
    serde_json::from_value(json!({ "rules": [
        { "id": "db.session", "operation": "database", "match": "call", "names": ["exec", "get"], "receivers": ["session"] },
        { "id": "db.relationship", "operation": "database", "match": "field_declared_with", "calls": ["Relationship"] }
    ] }))
    .unwrap()
}

/// Each operation as `symbol line rule text`.
fn operations(inventory: &Inventory) -> Vec<String> {
    inventory.operations.iter().map(|o| format!("{} {} {} {}", o.symbol_name, o.line, o.rule, o.text)).collect()
}

#[test]
fn call_parts_read_the_written_name_and_its_receiver() {
    let parts = |t: &str| call_parts(t).map(|(r, n)| format!("{r}|{n}"));
    assert_eq!(parts("self.session.exec(select(X).where(a))"), Some("session|exec".into()));
    assert_eq!(parts("session.exec(statement).all()"), Some("()|all".into()));
    assert_eq!(parts("File::open"), Some("File|open".into()));
    assert_eq!(parts("open"), Some("|open".into()));
    assert_eq!(parts("items.iter().collect::<Vec<_>>()"), Some("()|collect".into()));
    assert_eq!(parts("p.x.sqrt()"), Some("x|sqrt".into()));
    assert_eq!(parts("(a)"), None);
}

#[test]
fn operations_are_found_by_declared_rules_with_their_evidence() {
    let project = Project::new();
    let catalog = project.edition();
    let found = inventory::inventory(&catalog, &rules()).unwrap();
    assert_eq!(found.version, "operation-inventory-v1");
    assert_eq!(operations(&found), [
        "list_for_day 7 db.session session.exec(day)",
        "address_of 11 db.relationship delivery.address",
        "zone_of 15 db.relationship address.area",
        "distance 19 math.call sqrt",
        "plan 29 state.module_write _seen",
        "norm 8 math.method p.x.sqrt()",
    ]);
    // A plain field (`weight`, `zone`) is not a relationship; the rule names what it matched.
    let area = found.operations.iter().find(|o| o.text == "address.area").unwrap();
    assert_eq!(area.through.len(), 1);
    assert!(area.through[0].contains("field"));
    assert!(!area.in_loop);
    let write = found.operations.iter().find(|o| o.rule == "state.module_write").unwrap();
    assert!(write.in_loop);
    assert_eq!((write.evidence.path.as_str(), write.evidence.start_line), ("app/service.py", 29));
    // Every rule is reported with where it came from and what it found.
    let rule = |id: &str| found.rules.iter().find(|r| r.id == id).unwrap();
    assert_eq!((rule("db.relationship").source.as_str(), rule("db.relationship").operations), ("project", 2));
    assert_eq!((rule("math.call").source.as_str(), rule("math.call").operations), ("builtin", 1));
    assert!(found.calls_unmatched > 0);

    // Who calls whom among the declarations that hold operations, without homonyms.
    let plan = found.symbols.iter().find(|s| s.name == "plan").unwrap();
    let mut names: Vec<&str> = plan.calls.iter().map(|c| found.symbols.iter().find(|s| &s.symbol == c).unwrap().name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["address_of", "distance", "list_for_day", "zone_of"]);
    let zone = found.symbols.iter().find(|s| s.name == "zone_of").unwrap();
    assert_eq!(zone.called_by, [plan.symbol.clone()]);
}

#[test]
fn loops_show_what_they_hold_directly_and_through_calls() {
    let project = Project::new();
    let catalog = project.edition();
    let found = inventory::inventory(&catalog, &rules()).unwrap();
    let ops = |ids: &[String]| -> Vec<String> { ids.iter().map(|id| found.operations.iter().find(|o| &o.id == id).unwrap().text.clone()).collect() };
    let looped = |head: &str| found.loops.iter().find(|l| l.head == head).unwrap_or_else(|| panic!("{head}: {:?}", found.loops.iter().map(|l| &l.head).collect::<Vec<_>>()));

    // Level one: the comprehension calls address_of, which reads the relationship.
    let comprehension = looped("[address_of(d) for d in deliveries]");
    assert!(comprehension.direct.is_empty());
    let chain = &comprehension.through_calls[0];
    assert_eq!(chain.steps.iter().map(|s| (s.to_name.as_str(), s.line, s.resolution.as_str())).collect::<Vec<_>>(), [("address_of", 24, "unique")]);
    assert_eq!(ops(&[chain.operation.clone()]), ["delivery.address"]);

    // Level two: the outer loop writes the module dict and calls zone_of; the inner loop is
    // inside it and calls distance twice, which reaches math.
    let outer = looped("for a in addresses");
    assert_eq!((outer.nesting, outer.inner_loops), (0, 1));
    assert_eq!(ops(&outer.direct), ["_seen"]);
    let reached: Vec<String> = outer.through_calls.iter().map(|c| format!("{} -> {}", c.steps.iter().map(|s| s.to_name.as_str()).collect::<Vec<_>>().join(" -> "), ops(&[c.operation.clone()])[0])).collect();
    assert_eq!(reached, ["distance -> sqrt", "zone_of -> address.area"]);
    // A loop of another declaration, reached through a call: a loop in a loop through a chain.
    let through: Vec<String> = outer.loops_through_calls.iter().map(|c| format!("{} -> {}", c.steps[0].to_name, c.head)).collect();
    assert_eq!(through, ["heavy -> 1 for x in addresses if x"]);
    let inner = looped("for b in addresses");
    assert_eq!((inner.nesting, inner.inner_loops), (1, 0));
    assert_eq!(inner.repeated_calls.len(), 1);
    assert_eq!((inner.repeated_calls[0].name.as_str(), inner.repeated_calls[0].lines.clone()), ("distance", vec![28, 28]));

    // Rust: a method call in a loop is a direct operation.
    assert_eq!(ops(&looped("for p in points").direct), ["p.x.sqrt()"]);
    // The loop inside heavy holds no operation: by itself it is only counted. So does the one
    // in weights: its iterable, list_for_day(...), runs once, before the loop.
    assert_eq!(found.loops_without_operations, 2);
    assert!(!found.loops.iter().any(|l| l.symbol_name == "weights"));

    // The text summary names each place as path:line.
    let text = inventory::summary(&found);
    assert!(text.contains("pela cadeia: zone_of (app/service.py:29) → database app/service.py:15  address.area"), "{text}");
}

#[test]
fn project_rules_are_checked() {
    let project = Project::new();
    let catalog = project.edition();
    let bad = |rules: serde_json::Value| inventory::inventory(&catalog, &serde_json::from_value(rules).unwrap()).unwrap_err().to_string();
    assert!(bad(json!({ "rules": [{ "id": "math.call", "operation": "x", "match": "module_write" }] })).contains("built-in"));
    assert!(bad(json!({ "rules": [{ "id": "a", "operation": "x", "match": "call", "names": [] }] })).contains("matches no name"));
    assert!(serde_json::from_value::<RuleSet>(json!({ "rules": [{ "id": "a", "operation": "x", "match": "guess" }] })).is_err());
    // Without project rules, only the built-in ones apply.
    let plain = inventory::inventory(&catalog, &RuleSet::default()).unwrap();
    assert!(plain.operations.iter().all(|o| o.operation != "database"));
}
