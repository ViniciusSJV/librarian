//! The Studio's sends. Which task a send asks for comes from what is sent (a question's claims
//! policy, or the lexicon); which profile answers is the person's choice, or the action's default
//! in the library; the language is the project's. The library of tasks and profiles and the
//! transport to each provider are the LLM Engine's: nothing here knows a model.
//!
//! Each send preserves what was sent and what came back; `completed` describes the transport,
//! never a semantic approval of the answer.
use crate::store::{display, now_ms, stamp};
use librarian_llm_engine::{
    models::{Language, Library},
    transport::{self, Request},
};
pub use librarian_llm_engine::transport::Skill;
use serde_json::{json, Value};
use std::{fs, path::Path};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Who answers, chosen on the page: a profile of the library, the Claude model, and the skills.
pub struct Choice {
    pub profile: String,
    pub claude_model: Option<String>,
    pub skills: Vec<Skill>,
}

/// The choices the page offers: copying the query by hand, and each profile of the library.
pub fn providers(library: &Library) -> Value {
    let mut list = vec![json!({ "id": "manual", "label": { "pt": "Nenhum (só preparar para copiar)", "en": "None (only prepare to copy)" }, "provider": null })];
    for id in library.profile_ids() {
        if let Some(p) = library.profile(&id) {
            list.push(json!({ "id": id, "label": p.label, "provider": p.provider, "models": p.models, "default_model": p.default_model }));
        }
    }
    json!(list)
}

/// What is installed and running, and the profiles the page offers.
pub fn status(library: &Library) -> Value {
    let mut status = transport::status(library);
    status["providers"] = providers(library);
    status
}

/// A question's send: only a question that passed its checks (`Store::verify_for_send`) has a
/// `Verified`, and only one can be sent. Its link, with the package's hash, goes with the send.
pub fn send_question(library: &Library, choice: &Choice, task: &str, language: Language, verified: &crate::store::Verified, dir: &Path) -> Result<Value> {
    send(library, choice, task, language, verified.query(), dir, verified.criteria_sha256(), Some(verified.link()), "question")
}

/// A lexicon's send: its own request, with no question and no evidence package behind it.
pub fn send_lexicon(library: &Library, choice: &Choice, language: Language, query: &Path, dir: &Path) -> Result<Value> {
    send(library, choice, librarian_llm_engine::models::LEXICON, language, query, dir, None, None, "lexicon")
}

/// The provider a choice sends with: the profile's.
pub fn provider_of(library: &Library, profile: &str) -> Result<String> {
    if cfg!(test) && profile == "test" {
        return Ok("test".into());
    }
    Ok(library.profile(profile).ok_or_else(|| format!("Perfil desconhecido: {profile}"))?.provider.clone())
}

/// Runs the profile's provider with the task's Modelfile rendered for it and writes `send.json`, the
/// summary the page shows, with what the send was for.
#[allow(clippy::too_many_arguments)]
fn send(library: &Library, choice: &Choice, task: &str, language: Language, query: &Path, dir: &Path,
    criteria_sha256: Option<&str>, link: Option<&Value>, purpose: &str) -> Result<Value> {
    // Only the skills the task takes go with it: an instruction that does not apply is not loaded.
    if let Some(skill) = choice.skills.iter().find(|s| !library.skills_of(task).contains(&s.name)) {
        return Err(format!("A skill {} não se aplica à tarefa {task}; nada foi enviado", skill.name).into());
    }
    let provider = provider_of(library, &choice.profile)?;
    fs::create_dir(dir)?;
    let started = now_ms();
    let mut summary = match provider.as_str() {
        // Tests stand a provider that answers at once for one that would call a model.
        #[cfg(test)]
        "test" => json!({ "state": "completed", "text": "" }),
        _ => {
            let rendered = library.render(task, &choice.profile, language)?;
            let request = Request { query, dir, modelfile: Some(&rendered), claude_model: choice.claude_model.as_deref(), link, skills: &choice.skills };
            transport::run(&provider, library, &choice.profile, &request)?
        }
    };
    summary["provider"] = json!(provider);
    summary["profile"] = json!(choice.profile);
    summary["task"] = json!(task);
    summary["language"] = json!(language);
    summary["created"] = json!(stamp());
    summary["elapsed_ms"] = json!(now_ms() - started);
    summary["directory"] = json!(display(dir));
    summary["semantic_evaluation"] = json!("pending");
    summary["criteria_sha256"] = json!(criteria_sha256);
    summary["link"] = json!(link);
    summary["skills"] = transport::skills_record(&choice.skills);
    summary["purpose"] = json!(purpose);
    // Not recorded is not linked: the send stands only once its send.json is written.
    if crate::store::failing_at("send_record") {
        return Err("O registro do envio não foi gravado; a resposta não está vinculada".into());
    }
    fs::write(dir.join("send.json"), serde_json::to_vec_pretty(&summary)?)?;
    Ok(summary)
}
