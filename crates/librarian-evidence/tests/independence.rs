//! The Evidence Builder on its own: a consumer that is not the Studio reads a question's files
//! itself, hands them over, and gets the package built, checked (H1 to H3) and its suggestions made
//! again (H4), with nothing of the Studio compiled. The question is the frozen format 1 store the
//! Studio wrote before format 2 (`librarian-studio/tests/evidence-v1`); this crate only reads it.
//!
//! What a consumer has to do is what this file does: read the files, say where the evidence came
//! from, and read its own measures. The last one is why H4 of a suggestion made over a measure is not
//! reproduced here: the reading of an execution is the keeper's, and this one reads none.
use librarian_evidence::{
    check::{self, QuestionRecord, Remake, SelectionFile, TurnFiles},
    keeper::Keeper,
    package::{self, Asked, BuildInput, ClaimsFiles, Criteria, Origin, Prepared, Suffix},
    selection, suggestion,
};
use librarian_ingest::{
    evidence::{EvidencePackage, NotReplayable, OriginKind, PackageFormat, Reproduction, Route, SelectionInput, SelectionRecord, Sha256Hex, SuffixInput, SuggestionReplay},
    hash,
    prepare::{self, Request},
    Catalog,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// The fixture's run, as the Studio's tests use it.
const RUN: &str = "studio/projects/project-a7507800/runs/20261111-145426";
/// The two packages as the earlier code wrote them, by turn.
const PINNED: [(&str, &str); 2] = [("01", "b7fa86ffa485ad8f0167a9c1a7b3aedfdcbee752fcdf8780e26752fa93dcf9b1"), ("02", "b0147ed7a7305492994e8a0bcfa35a9832e78893f628cc13f401886869c9c9ac")];

/// A copy of the run in a folder of its own, removed at the end. The files are text: a checkout that
/// wrote CRLF gets back the LF they were written with, as the Studio's tests do.
struct Copy(PathBuf);
impl Drop for Copy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn copy_run(name: &str) -> Copy {
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap().flatten() {
            let (a, b) = (entry.path(), to.join(entry.file_name()));
            if a.is_dir() { copy(&a, &b) } else { fs::write(&b, String::from_utf8(fs::read(&a).unwrap()).unwrap().replace("\r\n", "\n")).unwrap() }
        }
    }
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../librarian-studio/tests/evidence-v1").join(RUN);
    let to = std::env::temp_dir().join(format!("librarian-evidence-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&to);
    copy(&fixture, &to);
    Copy(to)
}

/// The run's records, read from the copy. It reads no measure: `hot` gives nothing.
struct FixtureKeeper<'a>(&'a Path);
impl Keeper for FixtureKeeper<'_> {
    fn suggestion(&self, sha256: &str) -> Result<Vec<u8>, String> {
        fs::read(self.0.join("suggestions").join(format!("{sha256}.json"))).map_err(|e| e.to_string())
    }
    fn execution(&self, id: &str) -> Result<Vec<u8>, String> {
        fs::read(self.0.join("executions").join(id).join("execution.json")).map_err(|e| e.to_string())
    }
    fn hot(&self, _execution: &Value) -> Option<Value> {
        None
    }
    fn graph(&self) -> Result<Vec<u8>, String> {
        fs::read(self.0.join("graph-v2.json")).map_err(|e| e.to_string())
    }
    fn lacunas(&self, turn: &str, send: &str) -> Result<Vec<String>, String> {
        Err(format!("envio {turn}/{send} indisponível"))
    }
}

/// One question's files, as read.
struct Read {
    question: Value,
    package: Vec<u8>,
    request: Vec<u8>,
    search: Vec<u8>,
    dossier: Vec<u8>,
    bundle_query: Vec<u8>,
    bundle_origin: Vec<u8>,
    context: Vec<u8>,
    query: Vec<u8>,
    selection: Option<Vec<u8>>,
}

fn read(dir: &Path) -> Read {
    let file = |name: &str| fs::read(dir.join(name)).unwrap();
    Read {
        question: serde_json::from_slice(&file("question.json")).unwrap(),
        package: file("evidence.json"),
        request: file("prepared/request.json"),
        search: file("prepared/search.json"),
        dossier: file("prepared/bundle/dossier.json"),
        bundle_query: file("prepared/bundle/query.json"),
        bundle_origin: file("prepared/bundle/origin.json"),
        context: file("claims/context.json"),
        query: file("claims/query.json"),
        selection: fs::read(dir.join("selection.json")).ok(),
    }
}

impl Read {
    fn files(&self) -> TurnFiles<'_> {
        TurnFiles {
            package: Some(&self.package),
            prepared: Prepared { request: Some(&self.request), search: Some(&self.search), dossier: Some(&self.dossier), bundle_query: Some(&self.bundle_query) },
            bundle_origin: Ok(&self.bundle_origin),
            claims: ClaimsFiles { context: Some(&self.context), query: Some(&self.query) },
            criteria: Criteria::Absent,
            selection: self.selection.as_deref().map_or(SelectionFile::Absent, SelectionFile::Bytes),
        }
    }

    fn record(&self) -> QuestionRecord<'_> {
        let q = &self.question;
        let asked = (q["asked"]["original"].as_str().unwrap().to_owned(), Vec::<Suffix>::new());
        assert_eq!(q["asked"]["suffixes"], serde_json::json!([]), "this store's questions have nothing added");
        QuestionRecord {
            mark: &q["evidence"],
            prepared: q["question"].as_str().unwrap(),
            asked: Ok(asked),
            criteria_sha256: q["criteria_sha256"].as_str(),
            claims: q["answer_format"] == "claims",
            sent_selection: SelectionInput::from_value(&q["gathered"]["origin"]["selection"]).ok(),
            additions: Ok(vec![]),
            nodes: serde_json::from_value(q["gathered"]["nodes"].clone()).unwrap(),
        }
    }

    /// Where the evidence came from, as this consumer reads the question: gathered, from a selection
    /// when its origin says so.
    fn origin(&self) -> Origin {
        let origin = &self.question["gathered"]["origin"];
        Origin {
            route: Route::Gathered,
            kind: (origin["kind"] == "selection").then_some(OriginKind::Selection),
            selection_sent: origin.get("selection").is_some_and(|s| !s.is_null()),
            additions: Ok(vec![]),
        }
    }

    /// The instructions the claims query was written with: none recorded is none.
    fn instructions(&self) -> Vec<String> {
        let context: Value = serde_json::from_slice(&self.context).unwrap();
        serde_json::from_value(context["project_instructions"].clone()).unwrap_or_default()
    }
}

#[test]
fn a_package_written_by_the_studio_passes_h1_to_h3_with_nothing_of_the_studio() {
    let copy = copy_run("checks");
    let run = &copy.0;
    let catalog = Catalog::load(&run.join("edition")).unwrap();
    let keeper = FixtureKeeper(run);
    for (turn, pinned) in PINNED {
        let turn_read = read(&run.join("turns").join(turn));
        assert_eq!(hash(&turn_read.package), pinned, "{turn}");
        let record = turn_read.record();
        let files = turn_read.files();
        // H1 and H2.
        let (sha256, package, bytes) = check::check_package(&keeper, Ok(&catalog), &files, &record).unwrap();
        assert_eq!((sha256.as_str(), package.format_version), (pinned, PackageFormat::V1), "{turn}");
        // H3: made again, the same bytes but for who built it.
        let instructions = turn_read.instructions();
        let remake = Remake { catalog: &catalog, files: &files, question: &record, origin: &turn_read.origin(), recorded_instructions: Some(&instructions), fact_lines: Ok(&[]) };
        check::rederive(&keeper, &remake, &package, &bytes).unwrap();
    }
}

#[test]
fn build_makes_the_studios_bytes_from_what_the_consumer_hands_over() {
    let copy = copy_run("build");
    let run = &copy.0;
    let catalog = Catalog::load(&run.join("edition")).unwrap();
    let keeper = FixtureKeeper(run);
    for (turn, pinned) in PINNED {
        let turn_read = read(&run.join("turns").join(turn));
        let stored: EvidencePackage = serde_json::from_slice(&turn_read.package).unwrap();
        // The preparation and the selection made again from what was recorded.
        let request: Request = serde_json::from_slice(&turn_read.request).unwrap();
        let composition = prepare::prepare(&catalog, &request).unwrap().composition;
        let accepted = stored.selection.as_ref().map(|reference| {
            let bytes = turn_read.selection.as_deref().unwrap();
            let record = SelectionRecord::from_bytes(bytes, reference.record_sha256.as_str()).unwrap();
            let nodes: Vec<String> = record.received.included.iter().map(|e| e.id.clone()).collect();
            let accepted = selection::accept(&keeper, &catalog, &nodes, record.received.clone()).unwrap();
            assert_eq!(accepted.bytes(), bytes, "{turn}: the selection's record is made again to the byte");
            accepted
        });
        let original = turn_read.question["asked"]["original"].as_str().unwrap();
        let suffixes: [SuffixInput; 0] = [];
        let instructions = turn_read.instructions();
        let files = turn_read.files();
        let bytes = package::build(&BuildInput {
            catalog: &catalog,
            composition: &composition,
            asked: Asked { original, suffixes: &suffixes },
            origin: &turn_read.origin(),
            prepared: files.prepared,
            claims: Some(files.claims),
            criteria: Criteria::Absent,
            project_instructions: Some(&instructions),
            fact_lines: Ok(&[]),
            at_question: Ok(stored.measure.at_question.clone()),
            selection: accepted.as_ref().map(|a| (a.sha256(), a.record())),
            format: stored.format_version,
        })
        .unwrap();
        // Who built it is this crate: the one field that may differ. The rest is the Studio's package.
        let mut built: EvidencePackage = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(built.versions.builder, librarian_evidence::identity::identity());
        built.versions.builder = stored.versions.builder.clone();
        let mut again = serde_json::to_vec_pretty(&built).unwrap();
        again.push(b'\n');
        assert_eq!(hash(&again), pinned, "{turn}");
    }
}

#[test]
fn h4_tells_what_it_could_make_again_and_says_why_not_the_rest() {
    let copy = copy_run("h4");
    let run = &copy.0;
    let catalog = Catalog::load(&run.join("edition")).unwrap();
    let keeper = FixtureKeeper(run);
    // 02 was asked with no selection: nothing to make again, as the Studio answered.
    let plain = read(&run.join("turns/02"));
    assert_eq!(suggestion::reproduce(&keeper, Ok(&catalog), Some(&plain.package)).unwrap(), Reproduction::NotApplicable {});
    // 01 rests on a suggestion made over a measure. This keeper reads no measure, so the same answer
    // does not come back, and H4 says so instead of passing.
    let selected = read(&run.join("turns/01"));
    let Reproduction::NotReplayable { suggestions, .. } = suggestion::reproduce(&keeper, Ok(&catalog), Some(&selected.package)).unwrap() else { panic!() };
    assert!(matches!(suggestions.as_slice(), [SuggestionReplay::NotReplayable { reason: NotReplayable::OutputDiffers, .. }]), "{suggestions:?}");
    // A question with no package has nothing to make again.
    assert_eq!(suggestion::reproduce(&keeper, Err("sem edition".into()), None).unwrap(), Reproduction::NotApplicable {});
}

#[test]
fn what_does_not_agree_is_refused_by_the_level_that_finds_it() {
    let copy = copy_run("refusals");
    let run = &copy.0;
    let catalog = Catalog::load(&run.join("edition")).unwrap();
    let keeper = FixtureKeeper(run);
    let mut turn_read = read(&run.join("turns/02"));
    // Another package under the same mark: H1.
    let original = turn_read.package.clone();
    turn_read.package.push(b' ');
    let refused = check::check_package(&keeper, Ok(&catalog), &turn_read.files(), &turn_read.record()).unwrap_err().to_string();
    assert_eq!(refused, "H1: o pacote não é o que a marca nomeia; nada foi enviado");
    // A unit whose text the snapshot does not give, under a mark that names it: H2.
    let mut package: EvidencePackage = serde_json::from_slice(&original).unwrap();
    package.units[0].text_sha256 = Sha256Hex::try_from("0".repeat(64)).unwrap();
    let mut forged = serde_json::to_vec_pretty(&package).unwrap();
    forged.push(b'\n');
    turn_read.package = forged;
    turn_read.question["evidence"]["sha256"] = Value::String(hash(&turn_read.package));
    let refused = check::check_package(&keeper, Ok(&catalog), &turn_read.files(), &turn_read.record()).unwrap_err().to_string();
    assert!(refused.starts_with("H2: TextDiffers"), "{refused}");
    // No edition to check against: H2 says so.
    turn_read.package = original.clone();
    turn_read.question["evidence"]["sha256"] = Value::String(hash(&original));
    let refused = check::check_package(&keeper, Err("sem edition".into()), &turn_read.files(), &turn_read.record()).unwrap_err().to_string();
    assert_eq!(refused, "H2: edition indisponível (sem edition); nada foi enviado");
}

/// The core reads no file, runs nothing, asks for no variable and knows no consumer: whatever it
/// needs is handed over. `build.rs` asks Git when the crate is compiled, which is how the identity of
/// the code that builds a package is known; it is not part of what runs.
#[test]
fn the_core_reads_nothing_by_itself_and_knows_no_consumer() {
    const SOURCES: [(&str, &str); 8] = [
        ("lib.rs", include_str!("../src/lib.rs")),
        ("check.rs", include_str!("../src/check.rs")),
        ("identity.rs", include_str!("../src/identity.rs")),
        ("keeper.rs", include_str!("../src/keeper.rs")),
        ("lacuna.rs", include_str!("../src/lacuna.rs")),
        ("package.rs", include_str!("../src/package.rs")),
        ("selection.rs", include_str!("../src/selection.rs")),
        ("suggestion.rs", include_str!("../src/suggestion.rs")),
    ];
    const FORBIDDEN: [&str; 12] = ["std::fs", "fs::", "File::open", "File::create", "OpenOptions", "Command", "env::var", "std::net", "librarian_studio", "librarian_llm_engine", "reqwest", ".studio"];
    for (file, text) in SOURCES {
        for word in FORBIDDEN {
            assert!(!text.contains(word), "{file} mentions {word:?}");
        }
    }
    let manifest = include_str!("../Cargo.toml");
    for crate_name in ["librarian-studio", "librarian-llm-engine", "reqwest"] {
        assert!(!manifest.contains(crate_name), "Cargo.toml depends on {crate_name}");
    }
}
