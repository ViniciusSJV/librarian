//! The identity of the Librarian code this binary was built from, as `build.rs` found it when it
//! was compiled. Nothing here asks Git now: the folder may have changed since, and the binary is
//! what it was built from. `Unknown` says the build could not tell, and promises nothing more.
use librarian_ingest::evidence::BuilderIdentity;

/// The identity compiled into this binary.
#[cfg_attr(not(test), allow(dead_code))] // Read by the evidence package, which a later step builds.
pub fn identity() -> BuilderIdentity {
    parse(env!("LIBRARIAN_BUILDER"))
}

/// `unknown`, or `recorded:<commit>:<dirty>` with a full lowercase commit. Anything else is unknown:
/// a value that cannot be read is not turned into one that says more than it does.
fn parse(text: &str) -> BuilderIdentity {
    let mut parts = text.split(':');
    let (Some("recorded"), Some(commit), Some(dirty), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return BuilderIdentity::Unknown {};
    };
    let full = (commit.len() == 40 || commit.len() == 64) && commit.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    match (full, dirty) {
        (true, "true") => BuilderIdentity::Recorded { git_commit: commit.to_owned(), git_dirty: true },
        (true, "false") => BuilderIdentity::Recorded { git_commit: commit.to_owned(), git_dirty: false },
        _ => BuilderIdentity::Unknown {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn what_the_build_wrote_is_read_back_and_nothing_else_is_taken_for_an_identity() {
        let commit = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(parse(&format!("recorded:{commit}:false")), BuilderIdentity::Recorded { git_commit: commit.into(), git_dirty: false });
        assert_eq!(parse(&format!("recorded:{commit}:true")), BuilderIdentity::Recorded { git_commit: commit.into(), git_dirty: true });
        let long = "a".repeat(64);
        assert_eq!(parse(&format!("recorded:{long}:false")), BuilderIdentity::Recorded { git_commit: long.clone(), git_dirty: false });
        for text in [
            "unknown".to_owned(),
            String::new(),
            format!("recorded:{commit}"),
            format!("recorded:{commit}:sim"),
            format!("recorded:{commit}:false:extra"),
            format!("recorded:{}:false", &commit[..7]),
            format!("recorded:{}:false", commit.to_uppercase()),
            "recorded:0.1.0:false".to_owned(),
            format!("gravado:{commit}:false"),
        ] {
            assert_eq!(parse(&text), BuilderIdentity::Unknown {}, "{text}");
        }
    }

    /// The identity of this very binary, in the contract's form. The scenarios of the step read it
    /// from this line.
    #[test]
    fn this_binary_has_an_identity_in_the_contracts_form() {
        let identity = identity();
        let value = serde_json::to_value(&identity).unwrap();
        println!("BUILDER|{value}");
        match identity {
            BuilderIdentity::Recorded { git_commit, .. } => assert!(git_commit.len() == 40 || git_commit.len() == 64),
            BuilderIdentity::Unknown {} => assert_eq!(value, json!({ "status": "unknown" })),
        }
        // Nothing of the machine, the moment or the folder: only the contract's fields.
        let names: Vec<&String> = value.as_object().unwrap().keys().collect();
        assert!(names.iter().all(|n| ["status", "git_commit", "git_dirty"].contains(&n.as_str())), "{names:?}");
    }
}
