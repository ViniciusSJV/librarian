//! P2: commands the analyzed project declares in its `librarian.json` (`commands`), run by the
//! Librarian in a copy of the project at the edition's commit, never in the project itself.
//! The Librarian knows argument lists, exit codes, wall time and text lines; it never knows
//! which tool a command calls or how that tool reports. A project that wants its own numbers
//! read prints `librarian-measure name=value [unit]` lines.
//! Commands are read from the profile on disk at each call, never from a request; each one
//! runs only after a person approved that exact command (its hash).
//! Layout: projects/<key>/commands.jsonl (approvals), <copies>/<key>/<run>/ (the copy, outside
//! the Librarian: see `copies`),
//! runs/<run>/executions/<NN-name>/{execution.json, stdout-N.txt, stderr-N.txt}.
use crate::store::{display, stamp, Store, PROFILE};
use librarian_ingest::{hash, Catalog, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command as Process, Stdio},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

const APPROVALS: &str = "commands.jsonl";
const MEASURE: &str = "librarian-measure";
const MAX_REPEAT: usize = 20;
const MAX_TIMEOUT_S: u64 = 3600;
/// Text kept in execution.json per stream; the files keep everything.
const TAIL: usize = 4000;
const LIMITS: &str = "Tempo de parede de um processo nesta máquina, com o que mais estiver rodando nela; não vale para outra máquina nem prova a causa de uma diferença. O Librarian não sabe o que o comando faz: guarda só os argumentos, o código de saída, o tempo e as linhas librarian-measure que o próprio projeto imprime.";

/// One run at a time: two measurements side by side would disturb each other.
static RUNNING: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    /// Program and arguments, passed as they are: no shell, no expansion.
    pub argv: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_s: u64,
    #[serde(default = "default_repeat")]
    pub repeat: usize,
    /// Files the command writes, relative to the copy; each one's sha256 is kept after every
    /// repetition. The Librarian does not know what they contain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
}

fn default_timeout() -> u64 {
    600
}
fn default_repeat() -> usize {
    1
}

impl Command {
    fn check(&self, name: &str) -> Result<()> {
        if self.argv.is_empty() || self.argv.iter().any(|a| a.is_empty() || a.contains('\0')) {
            return Err(format!("{name}: argv precisa de um programa e argumentos não vazios").into());
        }
        if self.timeout_s == 0 || self.timeout_s > MAX_TIMEOUT_S {
            return Err(format!("{name}: timeout_s entre 1 e {MAX_TIMEOUT_S}").into());
        }
        if self.repeat == 0 || self.repeat > MAX_REPEAT {
            return Err(format!("{name}: repeat entre 1 e {MAX_REPEAT}").into());
        }
        let inside = |p: &str| !p.is_empty() && !Path::new(p).is_absolute() && !p.contains(':')
            && Path::new(p).components().all(|c| matches!(c, std::path::Component::Normal(_)));
        if let Some(bad) = self.outputs.iter().find(|p| !inside(p)) {
            return Err(format!("{name}: output fora da cópia: {bad}").into());
        }
        Ok(())
    }

    /// What the approval is for: the name and every field, as one canonical JSON.
    fn sha256(&self, name: &str) -> String {
        hash(json!({ "name": name, "argv": self.argv, "timeout_s": self.timeout_s, "repeat": self.repeat, "outputs": self.outputs }).to_string().as_bytes())
    }
}

/// The profile's `instrument`, as written there now.
pub(crate) fn instrument_of(root: &Path) -> Result<crate::instrument::Instrument> {
    let profile: Value = serde_json::from_slice(&fs::read(root.join(PROFILE)).map_err(|_| format!("Sem {PROFILE} no projeto"))?)?;
    let found = profile.get("instrument").ok_or_else(|| format!("{PROFILE} não tem instrument (a instrução que conta uma passagem por uma função)"))?;
    let instrument: crate::instrument::Instrument = serde_json::from_value(found.clone()).map_err(|e| format!("{PROFILE}: instrument inválido: {e}"))?;
    instrument.check()?;
    Ok(instrument)
}

/// What the profile says about its own measure, for the gate. No `instrument` is fine: a project
/// may print its counts its own way, and the statement need not be one "Medir tudo" can place.
fn measurement_of(root: &Path) -> (Option<String>, Option<String>, Vec<String>) {
    let profile: Value = fs::read(root.join(PROFILE)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let text = |field: &str| profile["instrument"][field].as_str().map(str::to_owned);
    let mut protect: Vec<String> = profile["instrument"]["protect"].as_array().into_iter().flatten()
        .filter_map(|p| p.as_str().map(str::to_owned)).collect();
    protect.push(PROFILE.to_owned());
    (text("statement"), text("command"), protect)
}

/// The profile's `measures`: what a number the project prints means, and which way is better.
/// Without it, only times and counts tell the gate anything.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Measure {
    better: Better,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    means: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Better {
    Lower,
    Higher,
}

fn measures_of(root: &Path) -> Result<BTreeMap<String, Measure>> {
    let Ok(bytes) = fs::read(root.join(PROFILE)) else { return Ok(BTreeMap::new()) };
    let profile: Value = serde_json::from_slice(&bytes)?;
    let Some(found) = profile.get("measures") else { return Ok(BTreeMap::new()) };
    let measures: BTreeMap<String, Measure> = serde_json::from_value(found.clone()).map_err(|e| format!("{PROFILE}: measures inválido: {e}"))?;
    if let Some(bad) = measures.keys().find(|k| k.contains('@') || *k == "wall_ms") {
        return Err(format!("{PROFILE}: measures: {bad} não é uma medida do projeto para a execução inteira").into());
    }
    Ok(measures)
}

/// The commands of the profile in `root`, as written there now.
fn declared(root: &Path) -> Result<BTreeMap<String, Command>> {
    let Ok(bytes) = fs::read(root.join(PROFILE)) else { return Ok(BTreeMap::new()) };
    let profile: Value = serde_json::from_slice(&bytes)?;
    let Some(commands) = profile.get("commands") else { return Ok(BTreeMap::new()) };
    let commands: BTreeMap<String, Command> = serde_json::from_value(commands.clone())
        .map_err(|e| format!("{PROFILE}: commands inválido: {e}"))?;
    for (name, command) in &commands {
        crate::store::safe_id(name)?;
        command.check(name)?;
    }
    Ok(commands)
}

impl Store {
    /// The profile's commands, each with its hash and whether that exact command is approved.
    pub fn commands(&self, key: &str) -> Result<Value> {
        let root = PathBuf::from(self.settings(key)?.root);
        let approvals = self.approvals(key)?;
        let list: Vec<Value> = declared(&root)?
            .iter()
            .map(|(name, c)| {
                let sha256 = c.sha256(name);
                let approved = approvals.iter().rev().find(|a| a["name"] == name.as_str() && a["sha256"] == sha256.as_str());
                json!({ "name": name, "argv": c.argv, "timeout_s": c.timeout_s, "repeat": c.repeat, "outputs": c.outputs, "sha256": sha256,
                    "approved": approved.map(|a| a["approved"].clone()) })
            })
            .collect();
        Ok(json!({ "profile": display(&root.join(PROFILE)), "commands": list, "limits": LIMITS }))
    }

    /// A person's approval of the command `name` exactly as the profile declares it now.
    /// Editing the command in the profile changes its hash and needs a new approval.
    pub fn approve_command(&self, key: &str, name: &str, sha256: &str) -> Result<Value> {
        let root = PathBuf::from(self.settings(key)?.root);
        let command = declared(&root)?.remove(name).ok_or_else(|| format!("Comando não declarado em {PROFILE}: {name}"))?;
        if command.sha256(name) != sha256 {
            return Err("O comando mudou no perfil depois de mostrado; abra de novo e confira".into());
        }
        let approval = json!({ "format_version": 1, "name": name, "sha256": sha256, "argv": command.argv,
            "timeout_s": command.timeout_s, "repeat": command.repeat, "outputs": command.outputs, "approved": stamp() });
        let mut file = OpenOptions::new().create(true).append(true).open(self.project_dir(key)?.join(APPROVALS))?;
        writeln!(file, "{approval}")?;
        Ok(approval)
    }

    fn approvals(&self, key: &str) -> Result<Vec<Value>> {
        let path = self.project_dir(key)?.join(APPROVALS);
        if !path.is_file() {
            return Ok(vec![]);
        }
        fs::read_to_string(path)?.lines().filter(|l| !l.trim().is_empty()).map(|l| Ok(serde_json::from_str(l)?)).collect()
    }

    /// Runs an approved command in the copy of `run`'s edition and records every repetition.
    /// With `change`, the copy is a separate one with that answer's derived diff applied, and
    /// the record is compared with the latest baseline of the same command.
    pub fn execute(&self, key: &str, run: &str, name: &str, change: Option<&ChangeRef>) -> Result<Value> {
        let root = PathBuf::from(self.settings(key)?.root);
        let command = declared(&root)?.remove(name).ok_or_else(|| format!("Comando não declarado em {PROFILE}: {name}"))?;
        let sha256 = command.sha256(name);
        let approval = self.approvals(key)?.into_iter().rev().find(|a| a["name"] == name && a["sha256"] == sha256.as_str())
            .ok_or("Comando sem aprovação para esta versão; aprove-o antes de rodar")?;
        let diff = change.map(|c| self.diff(key, run, c)).transpose()?;
        let _one = RUNNING.lock().map_err(|_| "Uma execução anterior falhou no meio; reinicie o studio")?;
        let edition = self.edition(key, run)?;
        let catalog = Catalog::load(&edition)?;
        let suffix = change.map(ChangeRef::folder);
        let copy = self.copy(key, run, &root, &edition, &catalog, suffix.as_deref().zip(diff.as_deref()))?;
        let (id, dir) = crate::store::reserve(&self.run_dir(key, run)?.join("executions"), &format!("{:02}-{name}", self.executions(key, run)?.len() + 1))?;
        let started = stamp();
        let mut runs = vec![];
        for index in 1..=command.repeat {
            let one = once(&command, &copy.dir, &dir, index)?;
            let failed = one["exit_code"] != 0 || one["timed_out"] == true;
            runs.push(one);
            if failed {
                break;
            }
        }
        let mut record = json!({
            "format_version": 1, "id": id, "name": name, "argv": command.argv, "timeout_s": command.timeout_s,
            "repeat": command.repeat, "outputs": command.outputs, "command_sha256": sha256, "approved": approval["approved"],
            "edition": format!("edition:{}", hash(&serde_json::to_vec(catalog.manifest())?)),
            "git_head": copy.head, "copy": display(&copy.dir), "sources_verified": copy.verified,
            "sources_line_endings_only": copy.line_endings,
            "machine": machine(), "started": started, "finished": stamp(),
            "runs": runs, "summary": summary(&runs), "limits": LIMITS,
        });
        let mut baseline_stats = None;
        if let (Some(c), Some(diff)) = (change, &diff) {
            record["change"] = json!({ "turn": c.turn, "send": c.send, "claim": c.claim,
                "diff_sha256": hash(diff.as_bytes()), "files": copy.touched });
            let baseline = self.executions(key, run)?.into_iter().rev()
                .find(|x| x["name"] == name && x["command_sha256"] == sha256.as_str() && x["change"].is_null()
                    && x["instrumentation"].is_null() && x["summary"]["succeeded"].as_u64() > Some(0));
            baseline_stats = baseline.as_ref().map(|b| b["summary"]["stats"].clone());
            record["comparison"] = match baseline {
                Some(b) => compare(&b, &record),
                None => json!({ "baseline": null, "note": "Sem linha de base: rode o mesmo comando sem mudança nesta edition para comparar." }),
            };
        }
        if let Some(diff) = &diff {
            let (statement, command, protect) = measurement_of(&root);
            let holder_start = |path: &str, line: usize| crate::instrument::holder(&catalog, path, line)
                .and_then(|s| catalog.chunk(&s.chunk_id)).map(|c| c.start_line);
            crate::ladder::review(&mut record, baseline_stats.as_ref(), diff, statement.as_deref(), &protect, holder_start);
            record["measures"] = json!(measures_of(&root)?);
            if command.as_deref() == Some(name) {
                record["counts_itself"] = json!(true);
            }
        }
        fs::write(dir.join("execution.json"), serde_json::to_vec_pretty(&record)?)?;
        Ok(record)
    }

    /// "Medir tudo": the approved command of the profile's `instrument`, run in a copy where the
    /// Librarian put the project's counting statement at the start of every function body. The
    /// record keeps the diff and the functions; it is never a baseline for a change.
    pub fn execute_instrumented(&self, key: &str, run: &str) -> Result<Value> {
        let root = PathBuf::from(self.settings(key)?.root);
        let instrument = instrument_of(&root)?;
        let name = instrument.command.as_str();
        let command = declared(&root)?.remove(name).ok_or_else(|| format!("Comando não declarado em {PROFILE}: {name}"))?;
        let sha256 = command.sha256(name);
        let approval = self.approvals(key)?.into_iter().rev().find(|a| a["name"] == name && a["sha256"] == sha256.as_str())
            .ok_or("Comando sem aprovação para esta versão; aprove-o antes de rodar")?;
        let _one = RUNNING.lock().map_err(|_| "Uma execução anterior falhou no meio; reinicie o studio")?;
        let edition = self.edition(key, run)?;
        let catalog = Catalog::load(&edition)?;
        let plan = crate::instrument::plan(&catalog, &instrument)?;
        let copy = self.copy(key, run, &root, &edition, &catalog, Some(("medir-tudo", &plan.diff)))?;
        let (id, dir) = crate::store::reserve(&self.run_dir(key, run)?.join("executions"), &format!("{:02}-{name}-tudo", self.executions(key, run)?.len() + 1))?;
        fs::write(dir.join("instrument.diff"), &plan.diff)?;
        let started = stamp();
        let mut runs = vec![];
        for index in 1..=command.repeat {
            let one = once(&command, &copy.dir, &dir, index)?;
            let failed = one["exit_code"] != 0 || one["timed_out"] == true;
            runs.push(one);
            if failed {
                break;
            }
        }
        let record = json!({
            "format_version": 1, "id": id, "name": name, "argv": command.argv, "timeout_s": command.timeout_s,
            "repeat": command.repeat, "outputs": command.outputs, "command_sha256": sha256, "approved": approval["approved"],
            "edition": format!("edition:{}", hash(&serde_json::to_vec(catalog.manifest())?)),
            "git_head": copy.head, "copy": display(&copy.dir), "sources_verified": copy.verified,
            "sources_line_endings_only": copy.line_endings,
            "machine": machine(), "started": started, "finished": stamp(),
            "runs": runs, "summary": summary(&runs), "limits": LIMITS,
            "instrumentation": { "statement": instrument.statement, "functions": plan.functions, "skipped": plan.skipped,
                "diff_sha256": hash(plan.diff.as_bytes()), "files": copy.touched },
        });
        fs::write(dir.join("execution.json"), serde_json::to_vec_pretty(&record)?)?;
        Ok(record)
    }

    /// The latest successful "Medir tudo" of `run`, if any.
    pub(crate) fn latest_measure(&self, key: &str, run: &str) -> Result<Option<Value>> {
        Ok(self.executions(key, run)?.into_iter().rev()
            .find(|x| !x["instrumentation"].is_null() && x["summary"]["succeeded"].as_u64() > Some(0)))
    }

    /// How the latest "Medir tudo" was made: what it inserted in the copy and what the program
    /// printed there, as recorded. Repetitions stop at the first failure, so the first one ran well.
    pub fn measure_detail(&self, key: &str, run: &str) -> Result<Value> {
        let latest = self.latest_measure(key, run)?.ok_or("Ainda não há medição de tudo nesta edition: rode Medir tudo")?;
        let dir = self.run_dir(key, run)?.join("executions").join(latest["id"].as_str().unwrap_or_default());
        Ok(json!({
            "execution": latest["id"], "argv": latest["argv"], "statement": latest["instrumentation"]["statement"],
            "files": latest["instrumentation"]["files"], "functions": latest["instrumentation"]["functions"].as_array().map_or(0, |f| f.len()),
            "diff": String::from_utf8_lossy(&fs::read(dir.join("instrument.diff"))?),
            "stdout": String::from_utf8_lossy(&fs::read(dir.join("stdout-1.txt"))?),
        }))
    }

    /// "Onde pesa": the latest successful "Medir tudo" of `run`, as a ranking.
    pub fn hotspots(&self, key: &str, run: &str) -> Result<Value> {
        let latest = self.latest_measure(key, run)?.ok_or("Ainda não há medição de tudo nesta edition: rode Medir tudo")?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let mut hot = crate::instrument::hotspots(&catalog, &latest);
        // What is costly by its kind, not by how often its function runs: an inventory operation
        // other than arithmetic (a lock, a sleep, I/O) inside a loop of a function that ran.
        let (found, _, _) = self.inventory_of(key, run)?;
        let calls: std::collections::BTreeMap<String, f64> = hot["rows"].as_array().into_iter().flatten()
            .filter_map(|r| Some((r["symbol"].as_str()?.to_owned(), r["calls"].as_f64()?))).collect();
        let structural: Vec<Value> = found.operations.iter()
            .filter(|o| o.operation != "math" && o.in_loop && calls.get(&o.symbol).is_some_and(|c| *c > 0.))
            .map(|o| json!({ "symbol": o.symbol, "name": o.symbol_name, "path": o.path, "line": o.line, "text": o.text.split_whitespace().collect::<Vec<_>>().join(" "),
                "operation": o.operation, "rule": o.rule, "calls": calls[&o.symbol] }))
            .collect();
        hot["structural"] = json!(structural);
        Ok(hot)
    }

    /// Where a hot line's value comes from (values.rs), over the latest "Medir tudo" of `run`.
    pub fn value_sources(&self, key: &str, run: &str, symbol: &str, text: &str) -> Result<Value> {
        let hot = self.hotspots(key, run)?;
        let catalog = Catalog::load(&self.edition(key, run)?)?;
        let rows: Vec<Value> = hot["rows"].as_array().cloned().unwrap_or_default();
        crate::values::sources(&catalog, &rows, symbol, text)
    }

    /// Executions recorded for `run`, oldest first; each carries its folder name as `id`.
    pub fn executions(&self, key: &str, run: &str) -> Result<Vec<Value>> {
        let dir = self.run_dir(key, run)?.join("executions");
        let Ok(entries) = fs::read_dir(&dir) else { return Ok(vec![]) };
        let mut names: Vec<_> = entries.flatten().map(|e| e.path()).filter(|p| p.join("execution.json").is_file()).collect();
        names.sort();
        names.iter().map(|p| {
            let mut x: Value = serde_json::from_slice(&fs::read(p.join("execution.json"))?)?;
            x["id"] = json!(p.file_name().map(|n| n.to_string_lossy().into_owned()));
            Ok(x)
        }).collect()
    }

    /// The diff a check derived from one claim's line edits; only an answer whose edits passed
    /// has one. It is applied as written, never edited by the Librarian.
    pub(crate) fn diff(&self, key: &str, run: &str, change: &ChangeRef) -> Result<String> {
        let send = self.turn_dir(key, run, &change.turn)?.join("sends").join(crate::store::safe_id(&change.send)?);
        let check: Value = serde_json::from_slice(&fs::read(send.join("check.json")).map_err(|_| "Envio sem conferência de claims")?)?;
        // The whole answer passed, or at least this change: its own edits derived a diff, no
        // issue names it, and no issue is about the whole answer.
        let issues: Vec<&Value> = check["report"]["issues"].as_array().into_iter().flatten().collect();
        let own_ok = !issues.iter().any(|i| i["claim_id"].is_null() || i["claim_id"] == change.claim.as_str());
        if check["report"]["mechanical_status"] != "passed" && !own_ok {
            return Err("A resposta não passou na conferência mecânica; a mudança não é aplicada".into());
        }
        Ok(check["diffs"][&change.claim].as_str().ok_or_else(|| format!("{} não tem mudança derivada", change.claim))?.to_owned())
    }

    /// The copy of the project at the edition's commit, made once per run (and once per change)
    /// and checked against the edition: every source the edition read must be there with the
    /// same bytes, except the files the change touches.
    fn copy(&self, key: &str, run: &str, root: &Path, edition: &Path, catalog: &Catalog, change: Option<(&str, &str)>) -> Result<Copy> {
        let head = catalog.manifest().git.head.clone()
            .ok_or("A edition não tem commit (o projeto não é um repositório git); não há cópia para rodar")?;
        let folder = match change {
            Some((suffix, _)) => format!("{run}--{suffix}"),
            None => run.to_owned(),
        };
        let dir = copies().join(crate::store::safe_id(key)?).join(crate::store::safe_id(&folder)?);
        let git = |at: &Path, args: &[&str]| Process::new("git").arg("-C").arg(at).args(args).output()
            .map_err(|e| format!("git não foi encontrado: {e}"));
        if !dir.join(".git").exists() {
            fs::create_dir_all(dir.parent().unwrap())?;
            // A copy whose folder was deleted (a cleaned temporary folder) is still registered.
            git(root, &["worktree", "prune"])?;
            // Its diff may have outlived it, and the new copy has no change applied yet.
            let _ = fs::remove_file(dir.with_extension("diff"));
            // The files as the repository stores them (no CRLF conversion on checkout): the
            // diffs are written with the edition's lines, and a converted file would not match
            // their context. The source check below already tolerates line endings.
            let output = git(root, &["-c", "core.autocrlf=false", "worktree", "add", "--detach", &display(&dir), &head])?;
            if !output.status.success() {
                return Err(format!("git worktree falhou: {}", String::from_utf8_lossy(&output.stderr).trim()).into());
            }
        }
        // The project may be a folder inside its repository (`backend/`): the copy is the whole
        // worktree, and the project is the same folder in it. Sources and diffs are relative to it.
        let prefix = git(root, &["rev-parse", "--show-prefix"])?;
        let prefix = String::from_utf8_lossy(&prefix.stdout).trim().trim_end_matches('/').to_owned();
        let project = if prefix.is_empty() { dir.clone() } else { dir.join(&prefix) };
        // The applied diff is kept beside the copy; a copy holds one change, applied once.
        let mut touched = vec![];
        if let Some((_, diff)) = change {
            touched = diff_files(diff);
            let applied = dir.with_extension("diff");
            match fs::read_to_string(&applied) {
                Ok(previous) if previous == diff => {}
                Ok(_) => return Err("Esta cópia já tem outra mudança aplicada".into()),
                Err(_) => {
                    fs::write(&applied, diff)?;
                    let directory = format!("--directory={prefix}");
                    // Context never leaves an evidence block, so a change on a block's last line
                    // has none after it; --unidiff-zero keeps git from reading that as the end
                    // of the file. The context lines that are there still have to match.
                    let mut args = vec!["apply", "--whitespace=nowarn", "--unidiff-zero"];
                    if !prefix.is_empty() {
                        args.push(&directory);
                    }
                    let applied_path = display(&applied);
                    args.push(&applied_path);
                    let output = git(&dir, &args)?;
                    if !output.status.success() {
                        let _ = fs::remove_file(&applied);
                        let _ = git(root, &["worktree", "remove", "--force", &display(&dir)]);
                        return Err(format!("git apply recusou a mudança: {}", String::from_utf8_lossy(&output.stderr).trim()).into());
                    }
                }
            }
        }
        // Git may check a file out with other line endings than a tool left in the project
        // (`core.autocrlf`); such a file is the same text, and is counted apart, not hidden.
        let (mut differ, mut line_endings) = (vec![], vec![]);
        for s in catalog.sources().values().filter(|s| !touched.contains(&s.path)) {
            let copied = fs::read(project.join(&s.path)).ok();
            if copied.as_deref().is_some_and(|b| hash(b) == s.sha256) {
                continue;
            }
            let read = fs::read(edition.join("snapshots").join(&s.sha256)).ok();
            match (copied, read) {
                (Some(c), Some(r)) if lf(&c) == lf(&r) => line_endings.push(s.path.clone()),
                _ => differ.push(s.path.as_str()),
            }
        }
        if !differ.is_empty() {
            return Err(format!("A cópia no commit {} difere da edition em {} arquivo(s), por exemplo {}; a edition foi gerada com mudanças sem commit. Faça commit e gere o grafo de novo.",
                &head[..head.len().min(12)], differ.len(), differ.iter().take(3).copied().collect::<Vec<_>>().join(", ")).into());
        }
        Ok(Copy { dir: project, head, verified: catalog.sources().len() - touched.len(), line_endings, touched })
    }
}

/// Which answer's change to apply: the claim of one send of one question of the run.
#[derive(Debug, Deserialize)]
pub struct ChangeRef {
    pub turn: String,
    pub send: String,
    pub claim: String,
}

impl ChangeRef {
    fn folder(&self) -> String {
        format!("{}-{}-{}", self.turn, self.send, self.claim)
    }
}

/// Paths a unified diff writes, from its `+++ b/` headers.
fn diff_files(diff: &str) -> Vec<String> {
    diff.lines().filter_map(|l| l.strip_prefix("+++ b/")).map(|p| p.trim().to_owned()).collect()
}

/// A change's numbers next to its baseline's. Ranges that overlap say the difference is
/// within what the repetitions vary; they do not say there is none, nor why.
fn compare(baseline: &Value, current: &Value) -> Value {
    let (b, c) = (&baseline["summary"], &current["summary"]);
    let mut stats = serde_json::Map::new();
    for (name, s) in c["stats"].as_object().into_iter().flatten() {
        let Some(base) = b["stats"].get(name) else { continue };
        let (median, base_median) = (s["median"].as_f64().unwrap_or(0.0), base["median"].as_f64().unwrap_or(0.0));
        let overlap = !(s["max"].as_f64() < base["min"].as_f64() || s["min"].as_f64() > base["max"].as_f64());
        stats.insert(name.clone(), json!({ "baseline_median": base_median, "median": median, "unit": s["unit"],
            "change_pct": (base_median != 0.0).then(|| ((median - base_median) / base_median * 1000.0).round() / 10.0),
            "ranges_overlap": overlap }));
    }
    let mut outputs = serde_json::Map::new();
    for (path, shas) in c["outputs"].as_object().into_iter().flatten() {
        let base = &b["outputs"][path];
        let one = |v: &Value| v.as_array().filter(|a| a.len() == 1).and_then(|a| a[0].as_str().map(str::to_owned));
        outputs.insert(path.clone(), json!(match (one(base), one(shas)) {
            (Some(x), Some(y)) if x == y => "same",
            (Some(_), Some(_)) => "different",
            _ => "unstable",
        }));
    }
    json!({ "baseline": baseline["id"], "stats": stats, "outputs": outputs })
}

/// Where copies live: `LIBRARIAN_COPIES`, or the system's temporary folder. Never inside the
/// Librarian's own tree: tools look for their project files in parent folders, and a copy
/// inside another project would be read as part of it.
fn copies() -> PathBuf {
    let base = std::env::var_os("LIBRARIAN_COPIES").map(PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join("librarian-copies"));
    // A plain path: tools given the Windows verbatim form (\\?\) refuse or misread it.
    PathBuf::from(display(&base))
}

/// The bytes with CRLF read as LF.
fn lf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        if !(b == b'\r' && bytes.get(i + 1) == Some(&b'\n')) {
            out.push(b);
        }
    }
    out
}

struct Copy {
    /// The project's folder inside the copy, where the command runs.
    dir: PathBuf,
    /// Sources whose copy differs from the edition only in line endings.
    line_endings: Vec<String>,
    /// Files the applied change writes; they are not compared with the edition.
    touched: Vec<String>,
    head: String,
    verified: usize,
}

/// One repetition: the program started in the copy, its streams saved whole, killed at the timeout.
fn once(command: &Command, cwd: &Path, out: &Path, index: usize) -> Result<Value> {
    // An output left by an earlier repetition must not pass for this one's.
    for output in &command.outputs {
        let _ = fs::remove_file(cwd.join(output));
    }
    let begin = Instant::now();
    let mut child = Process::new(&command.argv[0]).args(&command.argv[1..]).current_dir(cwd)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        .map_err(|e| format!("Não foi possível iniciar {:?}: {e}", command.argv[0]))?;
    let drain = |mut stream: Box<dyn Read + Send>| thread::spawn(move || {
        let mut bytes = vec![];
        let _ = stream.read_to_end(&mut bytes);
        bytes
    });
    let stdout = drain(Box::new(child.stdout.take().unwrap()));
    let stderr = drain(Box::new(child.stderr.take().unwrap()));
    let limit = Duration::from_secs(command.timeout_s);
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if begin.elapsed() >= limit {
            let _ = child.kill();
            timed_out = true;
            break child.wait().ok();
        }
        thread::sleep(Duration::from_millis(20));
    };
    let wall_ms = begin.elapsed().as_secs_f64() * 1000.0;
    let (stdout, stderr) = (stdout.join().unwrap_or_default(), stderr.join().unwrap_or_default());
    fs::write(out.join(format!("stdout-{index}.txt")), &stdout)?;
    fs::write(out.join(format!("stderr-{index}.txt")), &stderr)?;
    let text = String::from_utf8_lossy(&stdout);
    Ok(json!({
        "index": index, "exit_code": status.and_then(|s| s.code()), "timed_out": timed_out,
        "wall_ms": (wall_ms * 1000.0).round() / 1000.0,
        "stdout_sha256": hash(&stdout), "stderr_sha256": hash(&stderr),
        "measures": measures(&text),
        "outputs": command.outputs.iter().map(|p| (p.clone(), json!(fs::read(cwd.join(p)).ok().map(|b| hash(&b))))).collect::<serde_json::Map<_, _>>(),
        "stdout_tail": tail(&text), "stderr_tail": tail(&String::from_utf8_lossy(&stderr)),
    }))
}

/// `librarian-measure name=value [unit] [at=path:line]` lines, in order; any other line is the
/// program's own. `at` is where in the code the project says the number comes from (the line
/// that ran those queries, say); the Librarian keeps it as written and never checks it.
fn measures(text: &str) -> Vec<Value> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.trim().strip_prefix(MEASURE)?.split_whitespace();
            let (name, value) = words.next()?.split_once('=')?;
            let value: f64 = value.parse().ok().filter(|v: &f64| v.is_finite())?;
            let (at, rest): (Vec<&str>, Vec<&str>) = words.partition(|w| w.starts_with("at="));
            let at = match at.as_slice() {
                [] => None,
                [one] => Some(origin(&one[3..])?),
                _ => return None,
            };
            let unit = rest.first();
            let valid = !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.');
            valid.then(|| match at {
                Some(at) => json!({ "name": name, "value": value, "unit": unit, "at": at }),
                None => json!({ "name": name, "value": value, "unit": unit }),
            })
        })
        .collect()
}

/// `path:line` with a relative path and a line number; anything else makes the line invalid.
fn origin(at: &str) -> Option<&str> {
    let (path, line) = at.rsplit_once(':')?;
    let relative = !path.is_empty() && !path.starts_with(['/', '\\']) && !path.contains(':') && !path.split(['/', '\\']).any(|p| p == "..");
    (relative && line.parse::<usize>().is_ok_and(|n| n > 0)).then_some(at)
}

/// The key of a measure's series: its name, and where it comes from when the line says so.
fn series_key(measure: &Value) -> String {
    let name = measure["name"].as_str().unwrap_or_default();
    match measure["at"].as_str() {
        Some(at) => format!("{name}@{at}"),
        None => name.to_owned(),
    }
}

fn tail(text: &str) -> String {
    let start = text.char_indices().rev().nth(TAIL - 1).map_or(0, |(i, _)| i);
    text[start..].to_owned()
}

/// Wall time and each measure over the repetitions that finished with exit code 0.
fn summary(runs: &[Value]) -> Value {
    let ok: Vec<&Value> = runs.iter().filter(|r| r["exit_code"] == 0 && r["timed_out"] == false).collect();
    // A measure with `at` is its own series, `name@path:line`, and keeps both parts.
    let mut series: BTreeMap<String, (Vec<f64>, Value, Value)> = BTreeMap::new();
    series.insert("wall_ms".into(), (ok.iter().filter_map(|r| r["wall_ms"].as_f64()).collect(), json!("ms"), Value::Null));
    for run in &ok {
        for m in run["measures"].as_array().into_iter().flatten() {
            let origin = if m["at"].is_string() { json!({ "name": m["name"], "at": m["at"] }) } else { Value::Null };
            let entry = series.entry(series_key(m)).or_insert((vec![], m["unit"].clone(), origin));
            entry.0.extend(m["value"].as_f64());
        }
    }
    let stats: serde_json::Map<String, Value> = series.into_iter().filter(|(_, (v, _, _))| !v.is_empty()).map(|(key, (mut values, unit, origin))| {
        values.sort_by(f64::total_cmp);
        let n = values.len();
        let median = if n % 2 == 1 { values[n / 2] } else { (values[n / 2 - 1] + values[n / 2]) / 2.0 };
        let mut stat = json!({ "n": n, "median": median, "min": values[0], "max": values[n - 1], "unit": unit });
        if !origin.is_null() {
            stat["measure"] = origin["name"].clone();
            stat["at"] = origin["at"].clone();
        }
        (key, stat)
    }).collect();
    // Each output's distinct hashes over the successful repetitions: one means the same bytes.
    let mut outputs: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for run in &ok {
        for (path, sha) in run["outputs"].as_object().into_iter().flatten() {
            let seen = outputs.entry(path.clone()).or_default();
            if !seen.contains(sha) {
                seen.push(sha.clone());
            }
        }
    }
    json!({ "succeeded": ok.len(), "attempted": runs.len(), "stats": stats, "outputs": outputs })
}

/// What distinguishes this machine for a reader of the numbers; nothing is probed beyond this.
fn machine() -> Value {
    json!({
        "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
        "parallelism": thread::available_parallelism().map(|n| n.get()).ok(),
        "host": std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).ok(),
    })
}

#[cfg(test)]
mod tests {
    use super::{measures, summary};
    use serde_json::json;

    #[test]
    fn measure_lines_are_read_and_everything_else_ignored() {
        let text = "warming up\nlibrarian-measure render_ms=812.5 ms\n  librarian-measure rays=4096\nlibrarian-measure bad\nlibrarian-measure x=NaN\nmedian_ns=5\n";
        assert_eq!(measures(text), vec![json!({"name": "render_ms", "value": 812.5, "unit": "ms"}), json!({"name": "rays", "value": 4096.0, "unit": null})]);
    }

    #[test]
    fn a_measure_may_say_where_in_the_code_it_comes_from() {
        let text = "librarian-measure q=400 at=app/repositories/attempts.py:15\nlibrarian-measure t=3 ms at=src/a.rs:9\n\
            librarian-measure q=1 at=/etc/x.py:1\nlibrarian-measure q=1 at=../x.py:1\nlibrarian-measure q=1 at=a.py:x\nlibrarian-measure q=1 at=a.py:1 at=b.py:2\n";
        assert_eq!(measures(text), vec![
            json!({"name": "q", "value": 400.0, "unit": null, "at": "app/repositories/attempts.py:15"}),
            json!({"name": "t", "value": 3.0, "unit": "ms", "at": "src/a.rs:9"}),
        ]);
        // Each origin is its own series, next to the total of the same name.
        let runs = [
            json!({"exit_code": 0, "timed_out": false, "wall_ms": 1.0, "measures": [{"name": "q", "value": 805.0, "unit": null},
                {"name": "q", "value": 400.0, "unit": null, "at": "app/a.py:15"}, {"name": "q", "value": 200.0, "unit": null, "at": "app/b.py:26"}]}),
            json!({"exit_code": 0, "timed_out": false, "wall_ms": 1.0, "measures": [{"name": "q", "value": 805.0, "unit": null},
                {"name": "q", "value": 400.0, "unit": null, "at": "app/a.py:15"}]}),
        ];
        let s = summary(&runs);
        assert_eq!(s["stats"]["q"]["median"], 805.0);
        assert_eq!(s["stats"]["q@app/a.py:15"], json!({"n": 2, "median": 400.0, "min": 400.0, "max": 400.0, "unit": null, "measure": "q", "at": "app/a.py:15"}));
        assert_eq!(s["stats"]["q@app/b.py:26"]["n"], 1);
    }

    #[test]
    fn summary_uses_only_successful_repetitions() {
        let runs = [
            json!({"exit_code": 0, "timed_out": false, "wall_ms": 30.0, "measures": [{"name": "t", "value": 3.0, "unit": "ms"}]}),
            json!({"exit_code": 0, "timed_out": false, "wall_ms": 10.0, "measures": [{"name": "t", "value": 1.0, "unit": "ms"}]}),
            json!({"exit_code": 1, "timed_out": false, "wall_ms": 99.0, "measures": [{"name": "t", "value": 99.0, "unit": "ms"}]}),
        ];
        let s = summary(&runs);
        assert_eq!((s["succeeded"].as_u64(), s["attempted"].as_u64()), (Some(2), Some(3)));
        assert_eq!(s["stats"]["wall_ms"]["median"], 20.0);
        assert_eq!(s["stats"]["t"]["max"], 3.0);
    }
}
