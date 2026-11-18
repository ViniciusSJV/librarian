//! The model library: what to ask (a task), who answers (a profile) and in which language, from
//! one catalog. A Modelfile is not a file anyone keeps any more: it is rendered from the catalog
//! for each task × profile × language, and the copy a send uses is the one it records with its hash.
//!
//! The catalog (`catalog.json`, beside the task texts in `system/`) is the single source of:
//! - each task's instruction text, once per language (`system`), and, where two texts of the same
//!   task and language have not been reconciled, the one a profile keeps (`system_by_profile`);
//! - each profile's provider, model and parameters;
//! - which profile each action of the Studio uses by default (`actions`);
//! - which skills a task may take (`skills`): none is loaded where it is not listed.
//!
//! Texts are kept byte for byte: rendering a combination that existed as a file before gives the
//! same bytes and hash that file had.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The catalog's file name, in the library's folder.
pub const CATALOG: &str = "catalog.json";

/// Answering a question in claims, each item carrying its own proof.
pub const ANSWER: &str = "verificavel";
/// Proposing a change in claims: a HYPOTHESIS with edits and a measurement plan.
pub const CHANGE: &str = "performance";
/// Proposing words for the search lexicon.
pub const LEXICON: &str = "lexico";
/// Explaining in prose, with no mechanical check. Out of the default flow: only a question kept in
/// prose takes it.
pub const EXPLAIN: &str = "analista";

/// The language a project asks in: its claims contract, its task texts and its lexicon requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Pt,
    En,
}

impl Language {
    /// The project's setting: "en" is English; anything else, empty included, is Portuguese.
    pub fn of_setting(setting: &str) -> Self {
        if setting == "en" { Language::En } else { Language::Pt }
    }
    pub fn code(self) -> &'static str {
        match self {
            Language::Pt => "pt",
            Language::En => "en",
        }
    }
}

/// The task a prepared question asks for: a prose question explains; a claims question proposes a
/// change when its policy allows edits or a patch, and otherwise answers.
pub fn task_for_question(claims_policy: Option<(bool, bool)>) -> &'static str {
    match claims_policy {
        None => EXPLAIN,
        Some((allow_patch, allow_edits)) if allow_patch || allow_edits => CHANGE,
        Some(_) => ANSWER,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Labels {
    pub pt: String,
    pub en: String,
}

/// Who answers: a provider, the model it runs and how.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub label: Labels,
    /// `ollama` or `claude-cli`.
    pub provider: String,
    /// The Modelfile's `FROM`: the model for Ollama; for Claude only a name, since a Modelfile has to have one.
    pub from: String,
    /// `PARAMETER` lines, in order.
    #[serde(default)]
    pub parameters: Vec<(String, String)>,
    /// Parameters each task sets for this profile (`num_predict`), after the profile's own.
    #[serde(default)]
    pub task_parameters: Vec<String>,
    /// A comment line written before `FROM`.
    #[serde(default)]
    pub comment: Option<String>,
    /// The models a Claude profile may run, and the one it runs when none is chosen.
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub default_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    num_predict: u64,
    /// The instruction text of each language, as a file of the library.
    system: BTreeMap<String, String>,
    /// `profile.language` → the text that profile keeps in that language, where it differs.
    #[serde(default)]
    system_by_profile: BTreeMap<String, String>,
    /// `profile.language` → the comment line that describes the rendered Modelfile.
    #[serde(default)]
    notes: BTreeMap<String, String>,
    /// The skills this task may take; any other is refused.
    #[serde(default)]
    skills: Vec<String>,
}

/// The profile an action uses unless the person chooses another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub profile: String,
    #[serde(default)]
    pub claude_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CatalogFile {
    format: u32,
    header: String,
    profiles: BTreeMap<String, Profile>,
    tasks: BTreeMap<String, Task>,
    actions: BTreeMap<String, Action>,
}

/// A Modelfile rendered for one combination: its name (`task-profile[-en]`, the name the files had)
/// and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub name: String,
    pub text: String,
}

/// The catalog, read and checked, with the folder its texts are in.
#[derive(Debug, Clone)]
pub struct Library {
    dir: PathBuf,
    catalog: CatalogFile,
}

impl Library {
    /// Reads the catalog and checks that every task has a text in each language, every profile a
    /// known provider and every action a known profile.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(CATALOG);
        let catalog: CatalogFile = serde_json::from_slice(&fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if catalog.format != 1 {
            return Err(format!("{CATALOG}: formato {} desconhecido", catalog.format).into());
        }
        let library = Library { dir: dir.to_owned(), catalog };
        for (id, profile) in &library.catalog.profiles {
            if !["ollama", "claude-cli"].contains(&profile.provider.as_str()) {
                return Err(format!("{CATALOG}: perfil {id} com provedor desconhecido {}", profile.provider).into());
            }
        }
        for (task, def) in &library.catalog.tasks {
            for language in [Language::Pt, Language::En] {
                if !def.system.contains_key(language.code()) {
                    return Err(format!("{CATALOG}: tarefa {task} sem texto em {}", language.code()).into());
                }
                for profile in library.catalog.profiles.keys() {
                    library.system(task, profile, language)?;
                }
            }
        }
        for (action, chosen) in &library.catalog.actions {
            library.profile(&chosen.profile).ok_or_else(|| format!("{CATALOG}: ação {action} com perfil desconhecido {}", chosen.profile))?;
        }
        Ok(library)
    }

    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.catalog.profiles.get(id)
    }

    /// The profiles, the local ones (Ollama) first, each group by id.
    pub fn profile_ids(&self) -> Vec<String> {
        let mut ids: Vec<&String> = self.catalog.profiles.keys().collect();
        ids.sort_by_key(|id| self.catalog.profiles[*id].provider != "ollama");
        ids.into_iter().cloned().collect()
    }

    /// The profile and Claude model an action uses by default.
    pub fn action(&self, action: &str) -> Option<&Action> {
        self.catalog.actions.get(action)
    }

    /// The skills a task may take.
    pub fn skills_of(&self, task: &str) -> &[String] {
        self.catalog.tasks.get(task).map_or(&[], |t| t.skills.as_slice())
    }

    /// The instruction text of one task for one profile in one language, as kept.
    pub fn system(&self, task: &str, profile: &str, language: Language) -> Result<String> {
        let def = self.catalog.tasks.get(task).ok_or_else(|| format!("Tarefa desconhecida: {task}"))?;
        let file = def.system_by_profile.get(&format!("{profile}.{}", language.code())).or_else(|| def.system.get(language.code()))
            .ok_or_else(|| format!("Tarefa {task} sem texto em {}", language.code()))?;
        if file.contains("..") || Path::new(file).is_absolute() {
            return Err(format!("{CATALOG}: caminho de texto inválido: {file}").into());
        }
        let path = self.dir.join(file);
        Ok(fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?)
    }

    /// The Modelfile of one task for one profile in one language.
    pub fn render(&self, task: &str, profile: &str, language: Language) -> Result<Rendered> {
        let def = self.catalog.tasks.get(task).ok_or_else(|| format!("Tarefa desconhecida: {task}"))?;
        let p = self.profile(profile).ok_or_else(|| format!("Perfil desconhecido: {profile}"))?;
        let system = self.system(task, profile, language)?;
        if system.contains("\"\"\"") {
            return Err(format!("O texto de {task} não pode conter \"\"\"").into());
        }
        let note = def.notes.get(&format!("{profile}.{}", language.code())).cloned()
            .unwrap_or_else(|| format!("{task} para {}, SYSTEM em {}.", p.label.pt, language.code()));
        let mut lines = vec![self.catalog.header.clone(), format!("# {note}")];
        if let Some(comment) = &p.comment {
            lines.push(format!("# {comment}"));
        }
        lines.push(format!("FROM {}", p.from));
        for (name, value) in &p.parameters {
            lines.push(format!("PARAMETER {name} {value}"));
        }
        for name in &p.task_parameters {
            match name.as_str() {
                "num_predict" => lines.push(format!("PARAMETER num_predict {}", def.num_predict)),
                other => return Err(format!("{CATALOG}: parâmetro de tarefa desconhecido: {other}").into()),
            }
        }
        let suffix = if language == Language::En { "-en" } else { "" };
        Ok(Rendered { name: format!("{task}-{profile}{suffix}"), text: format!("{}\n\nSYSTEM \"\"\"\n{system}\"\"\"\n", lines.join("\n")) })
    }

    /// What a page shows of the library: profiles, tasks with their skills, the defaults of each
    /// action, and every rendered Modelfile, by name, read only.
    pub fn summary(&self) -> Result<Value> {
        let mut rendered = vec![];
        for task in self.catalog.tasks.keys() {
            for profile in self.catalog.profiles.keys() {
                for language in [Language::Pt, Language::En] {
                    let r = self.render(task, profile, language)?;
                    rendered.push(json!({ "name": r.name, "task": task, "profile": profile, "language": language, "content": r.text }));
                }
            }
        }
        let tasks: BTreeMap<&String, Value> = self.catalog.tasks.iter().map(|(id, t)| (id, json!({ "skills": t.skills }))).collect();
        Ok(json!({ "profiles": self.catalog.profiles, "tasks": tasks, "actions": self.catalog.actions, "modelfiles": rendered }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> Library {
        Library::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modelfiles")).unwrap()
    }

    fn sha(text: &str) -> String {
        crate::ollama::client::hash(text.as_bytes())
    }

    /// Every Modelfile the library had as a file is rendered again to the byte: the three validated
    /// ones (performance-claude, verificavel-qwen3coder, lexico-qwen3coder) and the others kept.
    #[test]
    fn the_catalog_renders_the_files_it_replaced_to_the_byte() {
        let pinned = [
            ("analista-claude-en", "5f568fc2c79b168cf3a28ba72a7c7ed79c7ccab733f2fc59a37398dbdac790df"),
            ("analista-claude", "ccbf140418442d5a77a2b88b9b2fa949ea93d453d719fa84af575c4f72d8ffb8"),
            ("analista-qwen3coder", "af9f5f561cd3eed5fb085f417f2a77130c518358f9445d1aff7833abe49b86ab"),
            ("lexico-claude-en", "b3fab7a8cab90a5d827d3a94834975ff29ed4c34ea92d72f52ff1f00bc8f120e"),
            ("lexico-claude", "3b4f419cbf787ec85fbd23d9a084c606ca331814b2f73756093994e278840072"),
            ("lexico-qwen3coder", "e1b71e78659cf80fd20e0d6cfdf675483558ab98f0311064e6e8eb9096e620bd"),
            ("performance-claude-en", "0cb082a5e5dfe217a915b473c2bfb1db11c6743bd4a511bf7404d6236680e109"),
            ("performance-claude", "fb60d59cd9d3ba8134226b96e39e677e426863fa5ff71594831bfebbbea0a21b"),
            ("performance-qwen3coder-en", "ad11f2b12b7337d9a95d60e4f37b89112c5b60e6e9744351e074605cf7a275ad"),
            ("performance-qwen3coder", "3a6ad7ec3a85c681ff37051473ac4d7c3f9b09147121d1298aec55b73b7c9d91"),
            ("verificavel-claude-en", "71f619d7b503accbb9735dd599e3a7e68c75d4098472566165e8d0b8e85e2755"),
            ("verificavel-claude", "60cdf13e1883205e2c9916bad29a4adb8a13d9a888b65f95576e47b45639f4eb"),
            ("verificavel-qwen3coder-en", "13edf8fc21741073cd13fb2aba995da360ece9cee54765f84f5e1d88c922a50c"),
            ("verificavel-qwen3coder", "64bf8c2d6eaafb59efb6f2fed0dc19fa498cf5886639612f8ad859e9fb4bc03a"),
        ];
        let library = library();
        for (name, hash) in pinned {
            let parts: Vec<&str> = name.split('-').collect();
            let language = if parts.len() == 3 { Language::En } else { Language::Pt };
            let rendered = library.render(parts[0], parts[1], language).unwrap();
            assert_eq!((rendered.name.as_str(), sha(&rendered.text).as_str()), (name, hash), "{name}");
        }
    }

    /// Each task has one text per language, the same for every profile; the one exception is said
    /// in the catalog (two translations of verificavel in English not reconciled yet) and is the
    /// only one.
    #[test]
    fn one_text_per_task_and_language_but_the_one_said_apart() {
        let library = library();
        let mut apart = vec![];
        for task in [ANSWER, CHANGE, LEXICON, EXPLAIN] {
            for language in [Language::Pt, Language::En] {
                let texts: Vec<String> = ["qwen3coder", "claude"].iter().map(|p| library.system(task, p, language).unwrap()).collect();
                assert!(texts.iter().all(|t| !t.trim().is_empty()), "{task} {language:?}");
                if texts[0] != texts[1] {
                    apart.push((task, language));
                }
            }
        }
        assert_eq!(apart, [(ANSWER, Language::En)]);
    }

    /// Two profiles, the local one first in every action but the ladder, and no skill offered by default.
    #[test]
    fn two_profiles_one_place_for_the_defaults_and_no_skill_by_default() {
        let library = library();
        assert_eq!(library.profile_ids(), ["qwen3coder", "claude"]);
        assert_eq!(library.profile("qwen3coder").unwrap().provider, "ollama");
        assert_eq!(library.profile("claude").unwrap().provider, "claude-cli");
        for action in ["locate", "improve", "chain", "lexicon", "facts"] {
            assert_eq!(library.action(action).unwrap().profile, "qwen3coder", "{action}");
        }
        let ladder = library.action("ladder").unwrap();
        assert_eq!((ladder.profile.as_str(), ladder.claude_model.as_deref()), ("claude", Some("haiku")));
        for task in [ANSWER, CHANGE, LEXICON, EXPLAIN] {
            assert!(library.skills_of(task).is_empty(), "{task}");
        }
        // Nothing of the library asks Ollama to drop the response schema any more.
        let summary = library.summary().unwrap();
        assert!(summary["modelfiles"].as_array().unwrap().iter().all(|m| !m["content"].as_str().unwrap().contains("structured-output")));
    }

    #[test]
    fn the_task_follows_the_questions_policy_and_the_language_the_projects_setting() {
        assert_eq!(task_for_question(None), EXPLAIN);
        assert_eq!(task_for_question(Some((false, false))), ANSWER);
        assert_eq!(task_for_question(Some((true, false))), CHANGE);
        assert_eq!(task_for_question(Some((false, true))), CHANGE);
        assert_eq!(Language::of_setting("en"), Language::En);
        for other in ["", "pt", "EN", "es"] {
            assert_eq!(Language::of_setting(other), Language::Pt, "{other}");
        }
    }

    #[test]
    fn a_catalog_that_names_what_it_does_not_have_is_refused() {
        let dir = std::env::temp_dir().join(format!("librarian-models-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("system")).unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modelfiles");
        for entry in fs::read_dir(source.join("system")).unwrap().flatten() {
            fs::copy(entry.path(), dir.join("system").join(entry.file_name())).unwrap();
        }
        let catalog: Value = serde_json::from_slice(&fs::read(source.join(CATALOG)).unwrap()).unwrap();
        let write = |c: &Value| fs::write(dir.join(CATALOG), serde_json::to_vec(c).unwrap()).unwrap();
        write(&catalog);
        assert!(Library::load(&dir).is_ok());
        let mut bad = catalog.clone();
        bad["actions"]["locate"]["profile"] = json!("gemma4");
        write(&bad);
        assert!(Library::load(&dir).unwrap_err().to_string().contains("perfil desconhecido gemma4"));
        let mut bad = catalog.clone();
        bad["profiles"]["qwen3coder"]["provider"] = json!("deepseek");
        write(&bad);
        assert!(Library::load(&dir).unwrap_err().to_string().contains("provedor desconhecido"));
        let mut bad = catalog.clone();
        bad["tasks"]["lexico"]["system"]["en"] = json!("../fora.txt");
        write(&bad);
        assert!(Library::load(&dir).unwrap_err().to_string().contains("caminho de texto inválido"));
        let _ = fs::remove_dir_all(&dir);
    }
}
