//! L4: gates from a checked claim to a reviewed fact. A FACT item is promoted only when its
//! answer passes the mechanical check again, the send is linked to the question's bundle and
//! that bundle still verifies, and a person approves it with a note. Promoted facts form the
//! project's collection (`facts.jsonl`, append-only), rechecked by text against any run's
//! edition. They are not evidence: no query reads them yet.
//! Layout: sends/<NN>/reviews.jsonl (each decision), projects/<key>/facts.jsonl (promotions and
//! revocations).
use crate::store::{safe_id, stamp, Store};
use librarian_ingest::{hash, Catalog, Result};
use librarian_llm_engine::claims::{self, Claim, Kind};
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

const FACTS: &str = "facts.jsonl";
const REVIEWS: &str = "reviews.jsonl";
pub const AUTHORSHIP: &str = "revisão humana no Librarian Studio";
const LIMITS: &str = "Fato revisado por uma pessoa, com as citações conferidas contra a edition. Vigente quer dizer que os trechos citados continuam iguais nas mesmas linhas; não quer dizer que o sentido foi conferido de novo.";

impl Store {
    /// Records one person's decision on one claim of a send. `approved` passes every gate and
    /// promotes the claim; `rejected` only records the decision and revokes an earlier promotion.
    pub fn review(&self, key: &str, run: &str, turn: &str, send: &str, claim_id: &str, approved: bool, note: &str) -> Result<Value> {
        let send_dir = self.turn_dir(key, run, turn)?.join("sends").join(safe_id(send)?);
        let note = note.trim();
        let (claim, response_sha256) = preserved_claim(&send_dir, claim_id)?;
        let claim_sha256 = hash(&serde_json::to_vec(&claim)?);
        let active = self.active_fact(key, |f| f["run"] == run && f["turn"] == turn && f["send"] == send && f["claim_id"] == claim_id)?;
        let mut review = json!({ "format_version": 1, "claim_id": claim_id, "claim_sha256": claim_sha256,
            "decision": if approved { "approved" } else { "rejected" }, "note": note, "reviewed": stamp() });
        if approved {
            if let Some(fact) = &active {
                return Err(format!("{claim_id} já foi promovido como {}", fact["id"].as_str().unwrap_or("?")).into());
            }
            if note.is_empty() {
                return Err("Escreva na nota por que o item é verdadeiro; a aprovação fica registrada com ela".into());
            }
            let link = self.gates(key, run, turn, &send_dir, &claim)?;
            let location = claim.location.as_ref().ok_or("Portão 1: FACT sem location")?;
            let id = format!("F{}", self.facts_log(key)?.iter().filter(|e| e["event"] == "promoted").count() + 1);
            let fact = json!({
                "format_version": 1, "event": "promoted", "id": id, "kind": "FACT",
                "statement": claim.text, "authorship": AUTHORSHIP,
                "path": location.path, "symbol_name": location.symbol_name, "declaration_line": location.declaration_line,
                "line": location.operation_start_line, "end_line": location.operation_end_line,
                "citations": claim.citations.iter().map(|c| json!({ "path": c.path, "start_line": c.start_line,
                    "end_line": c.end_line, "quote": c.quote })).collect::<Vec<_>>(),
                "edition": link["dossier_origin"], "run": run, "turn": turn, "send": send, "claim_id": claim_id,
                "claim_sha256": claim_sha256, "response_sha256": response_sha256, "link": link,
                "note": note, "reviewed": review["reviewed"],
                "gates": ["mechanical_passed", "link_verified", "human_review"],
            });
            review["fact_id"] = json!(id);
            append(&send_dir.join(REVIEWS), &review)?;
            append(&self.project_dir(key)?.join(FACTS), &fact)?;
        } else {
            append(&send_dir.join(REVIEWS), &review)?;
            if let Some(fact) = active {
                review["revoked"] = fact["id"].clone();
                append(&self.project_dir(key)?.join(FACTS), &json!({ "format_version": 1, "event": "revoked",
                    "id": fact["id"], "note": note, "reviewed": review["reviewed"] }))?;
            }
        }
        Ok(review)
    }

    /// Gates 1 and 2, recomputed now instead of trusting the stored records: the question still
    /// verifies, the send was linked to that same query, the preserved query has the linked
    /// hash, and the preserved answer passes the mechanical check against the verified context.
    fn gates(&self, key: &str, run: &str, turn: &str, send_dir: &Path, claim: &Claim) -> Result<Value> {
        if !matches!(claim.kind, Kind::Fact) {
            return Err("Só FACT vira fato; LACUNA, INFERENCE e HYPOTHESIS não são promovidos".into());
        }
        let summary: Value = serde_json::from_slice(&fs::read(send_dir.join("send.json"))?)?;
        let link = summary["link"].clone();
        if !link.is_object() {
            return Err("Portão 2: envio sem vínculo com o bundle (anterior à L3); pergunte e envie de novo".into());
        }
        let verified = self.verify_for_send(key, run, turn).map_err(|e| format!("Portão 2: a pergunta não confere mais: {e}"))?;
        if verified.link["query_sha256"] != link["query_sha256"] {
            return Err("Portão 2: a query da pergunta não é mais a que foi enviada".into());
        }
        let sent = ["query.json", "attempt/query.json"].iter().map(|f| send_dir.join(f)).find(|p| p.is_file())
            .ok_or("Portão 2: a query enviada não foi preservada")?;
        if link["query_sha256"].as_str() != Some(hash(&fs::read(sent)?).as_str()) {
            return Err("Portão 2: a query preservada no envio difere do vínculo".into());
        }
        let context = verified.context.ok_or("Portão 1: só perguntas em claims têm conferência mecânica")?;
        let text = fs::read_to_string(response_path(send_dir)?)?;
        let report = claims::check(&context, claims::extract_fenced(&text).unwrap_or(&text))?;
        if report.mechanical_status != "passed" {
            return Err("Portão 1: a resposta não passa na conferência mecânica".into());
        }
        Ok(link)
    }

    /// The collection: each promotion with its current state, rechecked by text against the
    /// edition of `run`. Revoked facts stay listed as revoked.
    pub fn facts(&self, key: &str, run: &str) -> Result<Value> {
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let log = self.facts_log(key)?;
        let facts: Vec<Value> = log
            .iter()
            .filter(|e| e["event"] == "promoted")
            .map(|fact| {
                let mut fact = fact.clone();
                let revoked = log.iter().rev().find(|e| e["event"] == "revoked" && e["id"] == fact["id"]);
                let (status, reason) = match revoked {
                    Some(r) => ("revoked", Some(format!("Revogado em {}: {}", r["reviewed"].as_str().unwrap_or("?"), r["note"].as_str().unwrap_or("")))),
                    None => match recheck(&catalog, &fact["citations"]) {
                        Ok(()) => ("current", None),
                        Err(reason) => ("stale", Some(reason)),
                    },
                };
                fact["status"] = json!(status);
                fact["status_reason"] = json!(reason);
                fact
            })
            .collect();
        let origin = format!("edition:{}", hash(&serde_json::to_vec(catalog.manifest())?));
        Ok(json!({ "run": run, "edition": origin, "facts": facts, "limits": LIMITS }))
    }

    /// Decisions on the claims of one send, last one per claim.
    pub fn reviews(send_dir: &Path) -> Value {
        let mut latest = serde_json::Map::new();
        for review in read_jsonl(&send_dir.join(REVIEWS)).unwrap_or_default() {
            if let Some(id) = review["claim_id"].as_str() {
                latest.insert(id.to_owned(), review);
            }
        }
        Value::Object(latest)
    }

    fn facts_log(&self, key: &str) -> Result<Vec<Value>> {
        read_jsonl(&self.project_dir(key)?.join(FACTS))
    }

    fn active_fact(&self, key: &str, matches: impl Fn(&Value) -> bool) -> Result<Option<Value>> {
        let log = self.facts_log(key)?;
        Ok(log.iter().filter(|e| e["event"] == "promoted" && matches(e)).last().cloned()
            .filter(|f| !log.iter().any(|e| e["event"] == "revoked" && e["id"] == f["id"])))
    }
}

/// The claim as written in the preserved answer, and that answer's hash, which must be the
/// one the stored check recorded.
fn preserved_claim(send_dir: &Path, claim_id: &str) -> Result<(Claim, String)> {
    let text = fs::read_to_string(response_path(send_dir)?)?;
    let response_sha256 = hash(text.as_bytes());
    let check: Value = serde_json::from_slice(&fs::read(send_dir.join("check.json")).map_err(|_| "Envio sem conferência de claims")?)?;
    if check["response_sha256"].as_str() != Some(response_sha256.as_str()) {
        return Err("A resposta preservada difere da que foi conferida".into());
    }
    let parsed: claims::Response = serde_json::from_str(claims::extract_fenced(&text).unwrap_or(&text))
        .map_err(|_| "A resposta não é um JSON de claims legível")?;
    let claim = parsed.claims.into_iter().find(|c| c.id == claim_id).ok_or_else(|| format!("Item desconhecido: {claim_id}"))?;
    Ok((claim, response_sha256))
}

fn response_path(send_dir: &Path) -> Result<std::path::PathBuf> {
    Ok(["response.txt", "attempt/response.txt"].iter().map(|f| send_dir.join(f)).find(|p| p.is_file())
        .ok_or("A resposta do envio não foi preservada")?)
}

/// Every cited quote is still the text of the same lines of the same file. Nothing is moved
/// or searched for: a line that shifted makes the fact stale.
fn recheck(catalog: &Catalog, citations: &Value) -> std::result::Result<(), String> {
    for citation in citations.as_array().into_iter().flatten() {
        let path = citation["path"].as_str().unwrap_or_default();
        let source = catalog.source_by_path(path).ok_or_else(|| format!("{path} não está nesta edition"))?;
        let bytes = catalog.snapshot(&source.id).ok_or_else(|| format!("{path} sem snapshot"))?;
        let lines: Vec<&str> = std::str::from_utf8(bytes).map_err(|_| format!("{path} não é UTF-8"))?.lines().collect();
        let (start, end) = (citation["start_line"].as_u64().unwrap_or(0) as usize, citation["end_line"].as_u64().unwrap_or(0) as usize);
        let current = (start >= 1 && end >= start && end <= lines.len()).then(|| lines[start - 1..end].join("\n"));
        if current.as_deref() != citation["quote"].as_str() {
            return Err(format!("{path}:{start}–{end} mudou desde a revisão"));
        }
    }
    Ok(())
}

fn read_jsonl(path: &Path) -> Result<Vec<Value>> {
    if !path.is_file() {
        return Ok(vec![]);
    }
    fs::read_to_string(path)?.lines().filter(|l| !l.trim().is_empty()).map(|l| Ok(serde_json::from_str(l)?)).collect()
}

fn append(path: &Path, value: &Value) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(format!("{}\n", serde_json::to_string(value)?).as_bytes())?;
    file.sync_all()?;
    Ok(())
}

