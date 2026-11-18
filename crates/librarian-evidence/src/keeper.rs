//! What the builder asks of whoever keeps a run's records, when what it needs is only known while
//! checking: the suggestion a selection cites, the execution a suggestion read, the LACUNA of a send
//! a lacuna round names. The keeper reads them where it keeps them and hands them over as they are;
//! the builder never reads a file. An error is the keeper's own message, and the builder refuses
//! with it as it is.
use serde_json::Value;

pub trait Keeper {
    /// The bytes of the suggestion record kept under this hash.
    fn suggestion(&self, sha256: &str) -> Result<Vec<u8>, String>;
    /// The bytes of the execution record with this id.
    fn execution(&self, id: &str) -> Result<Vec<u8>, String>;
    /// What an execution record says of each function, as a suggestion reads it (`rows`, `counted`
    /// and `coverage`); None when it says nothing a suggestion can use. The record's format is the
    /// keeper's, and so is reading it.
    fn hot(&self, execution: &Value) -> Option<Value>;
    /// The bytes of the run's graph (graph v2), for the labels a selection is shown with.
    fn graph(&self) -> Result<Vec<u8>, String>;
    /// The LACUNA texts of one send of one question.
    fn lacunas(&self, turn: &str, send: &str) -> Result<Vec<String>, String>;
}
