//! Profile v8 (R5 of the renderer phase): the base of a field access or write gets its written
//! type, and a local bound to a call of a free function of the same file gets the type its
//! signature returns. The suggested envelope then finds the writers of a field through another
//! receiver (`group.transform = ..`) and leaves out writes of same-named fields of other types.
use librarian_ingest::{envelope, generate_with_profile, Catalog, Config, EXTRACTOR_V7, EXTRACTOR_V8};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

// The renderer's case, small: the group's transform is set after the group is built, by a
// typed local and by the result of a function of the file; a camera has a homonymous field.
const GROUPS: &str = "pub struct Group {
    pub transform: f64,
}

impl Group {
    pub fn new() -> Self {
        Group { transform: 1.0 }
    }

    pub fn hit(&self) -> f64 {
        self.transform * 2.0
    }
}

pub struct Camera {
    pub transform: f64,
}

pub fn build() -> Group {
    Group::new()
}

pub fn load(t: f64) -> Group {
    let mut group = Group::new();
    group.transform = t;
    group
}

pub fn load_built(t: f64) -> Group {
    let mut model = build();
    model.transform = t;
    model
}

pub fn aim(camera: &mut Camera, t: f64) {
    camera.transform = t;
}
";

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("librarian-v8-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/groups.rs"), GROUPS).unwrap();
        Self(dir)
    }

    fn edition(&self, name: &str, profile: &str) -> Catalog {
        let path = self.0.join(name);
        let config = Config { project: "v8".into(), inputs: vec!["src".into()] };
        generate_with_profile(profile, &self.0, &config, &path).unwrap();
        Catalog::load(&path).unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The writers the suggested envelope adds for the field the method reads, by name.
fn writers(catalog: &Catalog) -> Vec<String> {
    let hit = catalog.symbols().values().find(|s| s.qualified_name == "impl Group::hit").unwrap().id.clone();
    let suggestion = envelope::suggest(catalog, &[hit]).unwrap();
    let mut found: Vec<String> = suggestion
        .items
        .iter()
        .filter(|i| i.reasons.iter().any(|r| r.code == "field_writer"))
        .map(|i| i.name.clone())
        .collect();
    found.sort();
    found
}

#[test]
fn a_field_written_through_a_typed_receiver_is_a_writer_of_its_owner() {
    let project = Project::new();
    let current = project.edition("v8", EXTRACTOR_V8);
    // `group` by `Group::new()`, `model` by `build() -> Group`; `camera.transform` is Camera's.
    assert_eq!(writers(&current), ["impl Group::new", "load", "load_built"]);

    // v7 had no type for a field's base: only the constructor, by its literal.
    let before = project.edition("v7", EXTRACTOR_V7);
    assert_eq!(writers(&before), ["impl Group::new"]);
}
