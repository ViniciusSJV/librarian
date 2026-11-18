//! A suggestion: the envelope proposed for the declarations a client chose, over the edition, the
//! inventory and, when there is one, a measure. Its record names what it was made from (the nodes,
//! the budget, the rules, the measure's execution and bytes) so that H4 can make it again and tell
//! whether the same answer comes back. H4 tells, it never refuses: a suggestion made again is not
//! proof it was true or enough.
//!
//! The measure comes as its keeper read it (`Keeper::hot`): `rows` of functions with their `calls`,
//! `counted`, and `coverage`, whose `functions` say for each one its `coverage_state`, its
//! `exclusion_reason` and its `zero_evidence`. The facts of each item are the acervo's, attached by
//! whoever keeps it before the record is made; they are recorded as given and never made again.
use crate::{keeper::Keeper, package::edition_origin};
use librarian_ingest::{
    envelope, evidence::{self, EvidencePackage, InventoryRuleEntry, InventoryRuleOrigin, NotReplayable, Reproduction, Sha256Hex, SuggestionInput,
        SuggestionInventoryRules, SuggestionMeasure, SuggestionRecord, SuggestionReplay},
    hash, inventory, Catalog, Result,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The declarations a measure can cover: functions and methods.
pub const MEASURED_KINDS: [&str; 3] = ["function", "method", "trait_method"];
/// A function with no reason in the record: a measure from before they were kept.
pub const NOT_RECORDED: &str = "not_recorded";
/// A function with no reason in a record that keeps them: it could not be classified.
pub const NOT_CLASSIFIED: &str = "not_classified";

/// The reason of a function with none, by whether its record keeps reasons at all.
pub fn missing_reason(recorded: bool) -> &'static str {
    if recorded { NOT_CLASSIFIED } else { NOT_RECORDED }
}

/// What a measure says about each function, as the envelope's rule takes it.
pub fn measure_of(hot: &Value) -> Option<envelope::Measure> {
    let rows = hot["rows"].as_array()?;
    let mut measure = envelope::Measure::default();
    measure.counted.extend(hot["counted"].as_array().into_iter().flatten().filter_map(|s| s.as_str().map(String::from)));
    for row in rows {
        let (Some(symbol), Some(calls)) = (row["symbol"].as_str(), row["calls"].as_f64()) else { continue };
        measure.counted.insert(symbol.to_owned());
        let most = measure.calls.entry(symbol.to_owned()).or_insert(0.);
        *most = most.max(calls);
    }
    Some(measure)
}

/// Each item and each one left out of a suggestion with what the measure's coverage says of it, and
/// the coverage's summary beside them.
pub fn annotate(suggestion: &mut Value, coverage: Option<&Value>) {
    let functions: BTreeMap<&str, &Value> = coverage.and_then(|c| c["functions"].as_array()).into_iter().flatten()
        .filter_map(|f| Some((f["symbol"].as_str()?, f))).collect();
    for list in ["items", "left_out"] {
        for entry in suggestion[list].as_array_mut().into_iter().flatten() {
            let (state, reason, zero) = match functions.get(entry["id"].as_str().unwrap_or_default()) {
                Some(f) => (f["coverage_state"].clone(), f["exclusion_reason"].clone(), f["zero_evidence"].clone()),
                None if MEASURED_KINDS.contains(&entry["kind"].as_str().unwrap_or_default()) => (json!("no_coverage"), coverage.map_or(Value::Null, |c| json!(missing_reason(c["reasons_recorded"] == true))), Value::Null),
                None => (Value::Null, Value::Null, Value::Null),
            };
            entry["coverage_state"] = state;
            entry["exclusion_reason"] = reason;
            entry["zero_evidence"] = zero;
        }
    }
    let mut summary = coverage.cloned().unwrap_or(Value::Null);
    if let Some(fields) = summary.as_object_mut() {
        fields.remove("functions");
    }
    suggestion["coverage"] = summary;
}

/// What a suggestion answers, but for the facts of each item: the selection over the edition, with
/// the inventory found and the measure read (`hot`), its coverage told. True when the measure was used.
pub fn answer(catalog: &Catalog, found: &inventory::Inventory, nodes: &[String], hot: Option<&Value>, budget: usize, max_items: usize) -> Result<(Value, bool)> {
    let measure = hot.and_then(measure_of);
    let mut candidates = envelope::candidates(catalog, nodes, measure.as_ref())?;
    envelope::trait_impls(catalog, found, nodes, measure.as_ref(), &mut candidates);
    let mut selection = envelope::select(catalog, &candidates, measure.as_ref(), budget, max_items);
    selection.behavior = envelope::behavior(catalog, found, &selection);
    selection.notes = envelope::notes(&selection);
    let mut out = serde_json::to_value(&selection)?;
    annotate(&mut out, hot.map(|h| &h["coverage"]));
    Ok((out, measure.is_some()))
}

/// The rules of a rule set, each with where it came from.
fn rule_entries(origin: InventoryRuleOrigin, set: &inventory::RuleSet) -> Result<Vec<InventoryRuleEntry>> {
    set.rules.iter().map(|rule| Ok(InventoryRuleEntry { origin, rule: serde_json::to_value(rule)? })).collect()
}

/// The record of a suggestion answered (`out`, its facts attached): the built-in rules as this code
/// has them and the project's as they were read for it, the measure as it was read.
pub fn record(catalog: &Catalog, input: SuggestionInput, project_rules: &inventory::RuleSet, measure: SuggestionMeasure, out: &Value) -> Result<SuggestionRecord> {
    let mut applied = rule_entries(InventoryRuleOrigin::Builtin, &inventory::builtin())?;
    applied.extend(rule_entries(InventoryRuleOrigin::Project, project_rules)?);
    Ok(SuggestionRecord::new(edition_origin(catalog)?, envelope::RULES_VERSION, input, SuggestionInventoryRules::new(inventory::VERSION, applied)?, measure, out)?)
}

/// A suggestion's record, if it is what its name says and still fits the run: the bytes are the
/// ones the hash names, the edition is this one, and the execution it used has the same bytes.
pub fn verify(keeper: &dyn Keeper, catalog: &Catalog, sha256: &str) -> Result<SuggestionRecord> {
    let sha256 = Sha256Hex::try_from(sha256.to_owned())?;
    let record = SuggestionRecord::from_bytes(&keeper.suggestion(sha256.as_str())?, sha256.as_str())?;
    if record.edition_origin != edition_origin(catalog)? {
        return Err(format!("Sugestão {} é de outra edition", sha256.as_str()).into());
    }
    if let SuggestionMeasure::Ok { execution, execution_sha256 } = &record.measure {
        if Sha256Hex::try_from(hash(&keeper.execution(execution)?))? != *execution_sha256 {
            return Err(format!("Sugestão {}: a execução {execution} mudou", sha256.as_str()).into());
        }
    }
    Ok(record)
}

/// One suggestion made again, or why not: its record, then the rules it names, then the execution
/// it read, then the suggestion itself.
pub fn replay(keeper: &dyn Keeper, catalog: &Catalog, sha256: &Sha256Hex) -> Result<Option<NotReplayable>> {
    use NotReplayable as Not;
    let read = keeper.suggestion(sha256.as_str());
    let Some(record) = read.ok().and_then(|bytes| SuggestionRecord::from_bytes(&bytes, sha256.as_str()).ok()) else { return Ok(Some(Not::SuggestionMissing)) };
    // The rules: of the selection, and of the inventory, the built-in ones as this code has them.
    let builtin_entries = rule_entries(InventoryRuleOrigin::Builtin, &inventory::builtin())?;
    let recorded_builtin: Vec<&InventoryRuleEntry> = record.inventory_rules.rules.iter().filter(|r| r.origin == InventoryRuleOrigin::Builtin).collect();
    if record.rules_version.0 != envelope::RULES_VERSION || record.inventory_rules.version != inventory::VERSION || recorded_builtin.into_iter().ne(builtin_entries.iter()) {
        return Ok(Some(Not::RulesVersionDiffers));
    }
    // The execution read then, with the bytes it had then.
    let hot = match &record.measure {
        SuggestionMeasure::Ok { execution, execution_sha256 } => {
            let Ok(bytes) = keeper.execution(execution) else { return Ok(Some(Not::ExecutionMissing)) };
            if hash(&bytes) != execution_sha256.as_str() {
                return Ok(Some(Not::ExecutionDiffers));
            }
            keeper.hot(&serde_json::from_slice(&bytes)?)
        }
        SuggestionMeasure::NoMeasure {} | SuggestionMeasure::Unreadable {} => None,
    };
    // The project's rules as the record copied them, not as the profile has them now.
    let project: Vec<&InventoryRuleEntry> = record.inventory_rules.rules.iter().filter(|r| r.origin == InventoryRuleOrigin::Project).collect();
    let project_rules: inventory::RuleSet = serde_json::from_value(json!({ "rules": project.iter().map(|r| &r.rule).collect::<Vec<_>>() }))?;
    let found = inventory::inventory(catalog, &project_rules)?;
    let (mut out, measured) = answer(catalog, &found, &record.input.nodes, hot.as_ref(), record.input.budget_bytes, record.input.max_items)?;
    // The facts of each item are the record's, as given: not made again.
    let recorded_facts = |id: &Value| record.output.items.iter().chain(&record.output.left_out).find(|i| i["id"] == *id).map_or(json!([]), |i| i["facts"].clone());
    for list in ["items", "left_out"] {
        for value in out[list].as_array_mut().into_iter().flatten() {
            value["facts"] = recorded_facts(&value["id"]);
        }
    }
    let measure = match (&record.measure, measured) {
        (SuggestionMeasure::Ok { .. }, true) => record.measure.clone(),
        (SuggestionMeasure::Ok { .. }, false) | (SuggestionMeasure::Unreadable {}, _) => SuggestionMeasure::Unreadable {},
        (SuggestionMeasure::NoMeasure {}, _) => SuggestionMeasure::NoMeasure {},
    };
    let mut applied = builtin_entries;
    applied.extend(project.into_iter().cloned());
    let again = SuggestionRecord::new(edition_origin(catalog)?, envelope::RULES_VERSION, record.input.clone(), SuggestionInventoryRules::new(inventory::VERSION, applied)?, measure, &out)?;
    Ok((!evidence::same_but_facts(&record, &again)).then_some(Not::OutputDiffers))
}

/// H4 of a question: each suggestion its selection cites, made again by this code from what its
/// record names, and compared with the record but for the facts. Told, never enforced. A question
/// with no package, or a package with no selection, has nothing to make again.
pub fn reproduce(keeper: &dyn Keeper, catalog: std::result::Result<&Catalog, String>, package: Option<&[u8]>) -> Result<Reproduction> {
    let Some(bytes) = package else { return Ok(Reproduction::NotApplicable {}) };
    let package: EvidencePackage = serde_json::from_slice(bytes)?;
    let Some(selection) = &package.selection else { return Ok(Reproduction::NotApplicable {}) };
    let catalog = catalog?;
    let mut replays = vec![];
    for sha256 in &selection.suggestions {
        replays.push(match replay(keeper, catalog, sha256)? {
            None => SuggestionReplay::Reproduced { suggestion: sha256.clone() },
            Some(reason) => SuggestionReplay::NotReplayable { suggestion: sha256.clone(), reason },
        });
    }
    Ok(Reproduction::of(replays, &package.versions.builder))
}

#[cfg(test)]
mod tests {
    use super::annotate;
    use serde_json::json;

    /// A function the measure's list does not have: the reason depends on the kind of record,
    /// as it does in the measure's coverage, and with no measure there is none.
    #[test]
    fn a_function_missing_from_the_coverage_follows_the_kind_of_record() {
        let annotated = |coverage: Option<serde_json::Value>| {
            let mut suggestion = json!({ "items": [{ "id": "x", "kind": "function" }, { "id": "y", "kind": "struct" }], "left_out": [] });
            annotate(&mut suggestion, coverage.as_ref());
            assert_eq!(suggestion["items"][0]["coverage_state"], "no_coverage");
            assert_eq!(suggestion["items"][1]["coverage_state"], serde_json::Value::Null);
            assert_eq!(suggestion["items"][1]["exclusion_reason"], serde_json::Value::Null);
            suggestion["items"][0]["exclusion_reason"].clone()
        };
        assert_eq!(annotated(Some(json!({ "functions": [], "reasons_recorded": false }))), "not_recorded");
        assert_eq!(annotated(Some(json!({ "functions": [], "reasons_recorded": true }))), "not_classified");
        assert_eq!(annotated(None), serde_json::Value::Null);
    }
}
