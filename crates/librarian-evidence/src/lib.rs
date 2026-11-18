//! The Evidence Builder: builds, checks and makes again the evidence package of a question, from
//! the data it is given and nothing else. It does not read files, ask Git, run a command or call a
//! model, and it knows nothing of who keeps the question or how: the Studio, an agent or any other
//! consumer reads what is needed and hands it over through the types here.
//!
//! The contract of the package (its types, `lines-v1`, the pure checks) is `librarian_ingest::evidence`,
//! which `prepare` also needs; this crate is what uses that contract.
pub mod check;
pub mod identity;
pub mod keeper;
pub mod lacuna;
pub mod package;
pub mod selection;
pub mod suggestion;
