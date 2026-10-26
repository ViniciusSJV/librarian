//! Immutable file-backed source editions. Hashes check consistency, not authenticity.
pub mod cli;
mod extractors;
pub mod graph;
pub mod owner;
pub mod prepare;
pub mod search;

use extractors::Registry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const SCHEMA_VERSION: u32 = 1;
/// Frozen v1 profile (Rust only); still read and reproducible with `generate_with_profile`.
pub const EXTRACTOR: &str = "librarian-ingest/0.1.0;syn/3.0.5;rust-structural-v1";
/// Rust (same adapter as v1) and Python; still read and reproducible with `generate_with_profile`.
pub const EXTRACTOR_V2: &str =
    "librarian-ingest/0.2.0;syn/3.0.5;rustpython-parser/0.4.0;rust-structural-v1;python-structural-v1";
/// G6: fields as symbols, attribute accesses, and calls and bases as name candidates.
pub const EXTRACTOR_V3: &str =
    "librarian-ingest/0.3.0;syn/3.0.5;rustpython-parser/0.4.0;rust-structural-v2;python-structural-v2";
/// Current profile (G6d): v3 plus, in Rust, struct literals, field writes and impl self types.
/// The Python adapter is the v3 one.
pub const EXTRACTOR_V4: &str =
    "librarian-ingest/0.4.0;syn/3.0.5;rustpython-parser/0.4.0;rust-structural-v3;python-structural-v2";
const RECORDS: [&str; 4] = [
    "sources.jsonl",
    "symbols.jsonl",
    "chunks.jsonl",
    "diagnostics.json",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub project: String,
    /// Relative directories or files. Directories contribute .rs files recursively.
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub extractor: String,
    pub config: Config,
    pub git: GitMetadata,
    /// Hash of each record file; snapshots are addressed by their content hash.
    pub files: BTreeMap<String, String>,
    pub source_count: usize,
    pub symbol_count: usize,
    pub diagnostic_count: usize,
    pub limits: Vec<String>,
}

/// Best-effort metadata observed before source reads, not proof of source membership.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GitMetadata {
    pub head: Option<String>,
    pub dirty: Option<bool>,
}

fn git_metadata(root: &Path) -> GitMetadata {
    let output = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
    };
    GitMetadata {
        head: output(&["rev-parse", "HEAD"]).map(|s| s.trim().to_owned()),
        dirty: output(&["status", "--porcelain", "--untracked-files=normal"])
            .map(|s| !s.trim().is_empty()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub path: String,
    pub sha256: String,
    pub byte_len: usize,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Symbol {
    pub id: String,
    pub source_id: String,
    pub name: String,
    /// File-local syntactic scope, not a resolved path.
    pub qualified_name: String,
    pub kind: String,
    pub is_test: bool,
    pub chunk_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Chunk {
    pub id: String,
    pub source_id: String,
    /// Zero-based byte range, exclusive end; lines are one-based and inclusive.
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub source_id: String,
    pub code: String,
    pub message: String,
}

/// Verified records and indexes. Constructed only by `Catalog::load`.
pub struct Catalog {
    manifest: Manifest,
    registry: &'static Registry,
    sources: BTreeMap<String, Source>,
    paths: BTreeMap<String, String>,
    symbols: BTreeMap<String, Symbol>,
    names: BTreeMap<String, Vec<String>>,
    chunks: BTreeMap<String, Chunk>,
    snapshots: BTreeMap<String, Vec<u8>>,
    diagnostics: Vec<Diagnostic>,
}

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn line_at(bytes: &[u8], offset: usize) -> usize {
    1 + bytes[..offset].iter().filter(|b| **b == b'\n').count()
}

fn relative(path: &str) -> Result<PathBuf> {
    // Use one portable spelling on disk. Refuse Windows drive/ADS syntax even on Unix.
    if path.is_empty()
        || path.contains(['\\', ':'])
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(format!("Non-canonical relative path: {path}").into());
    }
    let p = Path::new(path);
    if p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(format!("Unsafe relative path: {path}").into());
    }
    Ok(p.to_owned())
}

fn checked_path(root: &Path, path: &str) -> Result<PathBuf> {
    let rel = relative(path)?;
    let mut full = root.to_owned();
    for c in rel.components() {
        full.push(c);
        if fs::symlink_metadata(&full)?.file_type().is_symlink() {
            return Err(format!("Symbolic links are not accepted: {}", full.display()).into());
        }
    }
    if !full.canonicalize()?.starts_with(root) {
        return Err("Path escapes the project/edition root".into());
    }
    Ok(full)
}

fn config_check(config: &Config) -> Result<()> {
    if config.project.trim().is_empty() || config.inputs.is_empty() {
        return Err("Project and inputs are required".into());
    }
    for input in &config.inputs {
        relative(input)?;
    }
    Ok(())
}

fn walk(
    registry: &Registry,
    root: &Path,
    rel: &str,
    explicit: bool,
    found: &mut BTreeSet<String>,
) -> Result<()> {
    let full = checked_path(root, rel)?;
    let meta = fs::metadata(&full)?;
    if meta.is_dir() {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        if !explicit && registry.skipped_dirs.contains(&name) {
            return Ok(());
        }
        let mut children = Vec::new();
        for entry in fs::read_dir(full)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "Non-UTF-8 path")?;
            children.push(format!("{rel}/{name}"));
        }
        children.sort();
        for child in children {
            walk(registry, root, &child, false, found)?;
        }
    } else if meta.is_file() {
        if explicit || registry.discovers(rel) {
            found.insert(rel.into());
        }
    } else {
        return Err(format!("Not a regular file/directory: {rel}").into());
    }
    Ok(())
}

fn discover(registry: &Registry, root: &Path, config: &Config) -> Result<Vec<String>> {
    config_check(config)?;
    let mut paths = BTreeSet::new();
    for input in &config.inputs {
        walk(registry, root, input, true, &mut paths)?;
    }
    if paths.is_empty() {
        return Err("No source files selected".into());
    }
    Ok(paths.into_iter().collect())
}

fn source(registry: &Registry, path: String, bytes: &[u8]) -> Source {
    let digest = hash(bytes);
    let language = registry.language(&path);
    Source {
        id: format!("src:{}:{}", hash(path.as_bytes()), digest),
        sha256: digest,
        byte_len: bytes.len(),
        language: language.into(),
        path,
    }
}

fn jsonl<T: Serialize>(items: &[T]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for item in items {
        serde_json::to_writer(&mut out, item)?;
        out.push(b'\n');
    }
    Ok(out)
}

fn read_jsonl<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<Vec<T>> {
    std::str::from_utf8(bytes)?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Reads each source once, derives records from those bytes and writes a new edition.
/// manifest.json is published last. I/O errors can leave an incomplete directory.
pub fn generate(root: &Path, config: &Config, destination: &Path) -> Result<Manifest> {
    generate_with(Registry::current(), root, config, destination)
}

/// Top-level entries for a whole-folder edition: directories the current profile traverses
/// and files it extracts. Hidden and skipped directories are left out.
pub fn suggested_inputs(root: &Path) -> Result<Vec<String>> {
    let registry = Registry::current();
    let mut inputs = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let Ok(name) = entry.file_name().into_string() else { continue };
        let kind = entry.file_type()?;
        let keep = if kind.is_dir() {
            !name.starts_with('.') && !registry.skipped_dirs.contains(&name.as_str())
        } else {
            kind.is_file() && registry.discovers(&name)
        };
        if keep {
            inputs.push(name);
        }
    }
    inputs.sort();
    Ok(inputs)
}

/// `generate` with an earlier recorded profile, e.g. to reproduce frozen v1 editions.
pub fn generate_with_profile(
    identity: &str,
    root: &Path,
    config: &Config,
    destination: &Path,
) -> Result<Manifest> {
    let registry =
        Registry::for_identity(identity).ok_or_else(|| format!("Unknown extractor profile: {identity}"))?;
    generate_with(registry, root, config, destination)
}

fn generate_with(
    registry: &'static Registry,
    root: &Path,
    config: &Config,
    destination: &Path,
) -> Result<Manifest> {
    let root = root.canonicalize()?;
    let paths = discover(registry, &root, config)?;
    let git = git_metadata(&root);
    let mut sources = Vec::new();
    let mut symbols = Vec::new();
    let mut chunks = Vec::new();
    let mut diagnostics = Vec::new();
    let mut snapshots = BTreeMap::new();
    for path in paths {
        let bytes = fs::read(checked_path(&root, &path)?)?;
        let source = source(registry, path, &bytes);
        if let Some(extractor) = registry.extractor(&source.language) {
            let mut found = extractor.extract(&source, &bytes);
            symbols.append(&mut found.symbols);
            chunks.append(&mut found.chunks);
            diagnostics.append(&mut found.diagnostics);
        }
        snapshots.insert(source.sha256.clone(), bytes);
        sources.push(source);
    }
    let records = [
        jsonl(&sources)?,
        jsonl(&symbols)?,
        jsonl(&chunks)?,
        serde_json::to_vec_pretty(&diagnostics)?,
    ];
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        extractor: registry.identity.into(),
        config: config.clone(),
        git,
        files: RECORDS
            .iter()
            .zip(&records)
            .map(|(name, bytes)| (name.to_string(), hash(bytes)))
            .collect(),
        source_count: sources.len(),
        symbol_count: symbols.len(),
        diagnostic_count: diagnostics.len(),
        limits: registry.limits(),
    };
    // Destination must not be inside a recursively selected source directory.
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let target = parent.canonicalize()?.join(
        destination
            .file_name()
            .ok_or("Destination needs a directory name")?,
    );
    for input in &config.inputs {
        let input = checked_path(&root, input)?;
        if input.is_dir() && target.starts_with(input) {
            return Err("Destination overlaps source inputs".into());
        }
    }
    fs::create_dir(&target)?;
    fs::create_dir(target.join("snapshots"))?;
    for (digest, bytes) in snapshots {
        write_new(&target.join("snapshots").join(digest), &bytes)?;
    }
    for (name, bytes) in RECORDS.iter().zip(records) {
        write_new(&target.join(name), &bytes)?;
    }
    write_new(
        &target.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(manifest)
}

impl Catalog {
    /// Checks metadata hashes, snapshots and extraction against the recorded parser version.
    /// Does not need the original project. Refuses unknown schemas/extractors.
    pub fn load(edition: &Path) -> Result<Self> {
        let root = edition.canonicalize()?;
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(checked_path(&root, "manifest.json")?)?)?;
        config_check(&manifest.config)?;
        // The edition's own profile verifies it; a newer extractor is never tried instead.
        let registry = Registry::for_identity(&manifest.extractor)
            .filter(|r| manifest.schema_version == SCHEMA_VERSION && manifest.limits == r.limits())
            .ok_or("Unsupported schema, extractor or limits")?;
        if manifest
            .files
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != RECORDS.into_iter().collect()
        {
            return Err("Unexpected record files".into());
        }
        let mut records = BTreeMap::new();
        for (name, expected) in &manifest.files {
            let bytes = fs::read(checked_path(&root, name)?)?;
            if hash(&bytes) != *expected {
                return Err(format!("Record hash mismatch: {name}").into());
            }
            records.insert(name.clone(), bytes);
        }
        let sources: Vec<Source> = read_jsonl(&records["sources.jsonl"])?;
        let symbols: Vec<Symbol> = read_jsonl(&records["symbols.jsonl"])?;
        let chunks: Vec<Chunk> = read_jsonl(&records["chunks.jsonl"])?;
        let diagnostics: Vec<Diagnostic> = serde_json::from_slice(&records["diagnostics.json"])?;
        if sources.is_empty()
            || sources.len() != manifest.source_count
            || symbols.len() != manifest.symbol_count
            || diagnostics.len() != manifest.diagnostic_count
        {
            return Err("Record counts do not match manifest".into());
        }
        let mut snapshots = BTreeMap::new();
        let mut expected_symbols = Vec::new();
        let mut expected_chunks = Vec::new();
        let mut expected_diagnostics = Vec::new();
        let mut paths = BTreeMap::new();
        let mut source_map = BTreeMap::new();
        for s in sources {
            relative(&s.path)?;
            if s.sha256.len() != 64
                || !s
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err("Invalid snapshot hash".into());
            }
            let bytes = fs::read(checked_path(&root, &format!("snapshots/{}", s.sha256))?)?;
            if source(registry, s.path.clone(), &bytes) != s {
                return Err(format!("Snapshot/source mismatch: {}", s.path).into());
            }
            if paths.insert(s.path.clone(), s.id.clone()).is_some() {
                return Err("Duplicate source path".into());
            }
            if let Some(extractor) = registry.extractor(&s.language) {
                let mut found = extractor.extract(&s, &bytes);
                expected_symbols.append(&mut found.symbols);
                expected_chunks.append(&mut found.chunks);
                expected_diagnostics.append(&mut found.diagnostics);
            }
            snapshots.insert(s.id.clone(), bytes);
            if source_map.insert(s.id.clone(), s).is_some() {
                return Err("Duplicate source ID".into());
            }
        }
        if symbols != expected_symbols
            || chunks != expected_chunks
            || diagnostics != expected_diagnostics
        {
            return Err("Extraction records do not match the preserved source bytes".into());
        }
        let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut symbol_map = BTreeMap::new();
        for s in symbols {
            names.entry(s.name.clone()).or_default().push(s.id.clone());
            if symbol_map.insert(s.id.clone(), s).is_some() {
                return Err("Duplicate symbol ID".into());
            }
        }
        let mut chunk_map = BTreeMap::new();
        for c in chunks {
            if chunk_map.insert(c.id.clone(), c).is_some() {
                return Err("Duplicate chunk ID".into());
            }
        }
        Ok(Self {
            manifest,
            registry,
            sources: source_map,
            paths,
            symbols: symbol_map,
            names,
            chunks: chunk_map,
            snapshots,
            diagnostics,
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    /// Whether this edition's profile records what a chain step needs for an owner context (G6d).
    pub fn owner_context(&self) -> bool {
        self.registry.owner_context
    }
    /// What "what does it call?" needs from this edition's profile, for each language present:
    /// the occurrence kinds that are calls and, where the profile records owner syntax, the
    /// receiver of a method's own calls.
    pub fn call_syntax(&self) -> graph::v2::CallSyntax {
        let mut syntax = graph::v2::CallSyntax::default();
        for source in self.sources.values() {
            let Some(extractor) = self.registry.extractor(&source.language) else { continue };
            let receiver = self.registry.owner_syntax(&source.language).map(|o| o.receiver);
            syntax.languages.insert(source.language.clone(), (extractor.call_kinds().to_vec(), receiver));
        }
        syntax
    }
    pub fn sources(&self) -> &BTreeMap<String, Source> {
        &self.sources
    }
    pub fn symbols(&self) -> &BTreeMap<String, Symbol> {
        &self.symbols
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    pub fn source_by_path(&self, path: &str) -> Option<&Source> {
        self.sources.get(self.paths.get(path)?)
    }
    pub fn symbols_named(&self, name: &str) -> Vec<&Symbol> {
        self.names
            .get(name)
            .into_iter()
            .flatten()
            .filter_map(|id| self.symbols.get(id))
            .collect()
    }
    pub fn chunk(&self, id: &str) -> Option<&Chunk> {
        self.chunks.get(id)
    }
    pub fn excerpt(&self, chunk_id: &str) -> Option<&[u8]> {
        let c = self.chunks.get(chunk_id)?;
        self.snapshots
            .get(&c.source_id)?
            .get(c.start_byte..c.end_byte)
    }

    /// Optional comparison with the current project. Historical records are never rewritten.
    pub fn compare_current(&self, root: &Path) -> Result<Vec<Change>> {
        let root = root.canonicalize()?;
        let mut changes = Vec::new();
        for source in self.sources.values() {
            match checked_path(&root, &source.path).and_then(|p| Ok(fs::read(p)?)) {
                Ok(bytes) if hash(&bytes) == source.sha256 => {}
                Ok(_) => changes.push(Change {
                    path: source.path.clone(),
                    status: "changed".into(),
                }),
                Err(e) => changes.push(Change {
                    path: source.path.clone(),
                    status: format!("unavailable: {e}"),
                }),
            }
        }
        // Discovery errors are returned, not silently treated as an unchanged project.
        // The edition's own discovery policy applies, so newer languages are not reported as added.
        for path in discover(self.registry, &root, &self.manifest.config)? {
            if !self.paths.contains_key(&path) {
                changes.push(Change {
                    path,
                    status: "added".into(),
                });
            }
        }
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(changes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Change {
    pub path: String,
    pub status: String,
}
