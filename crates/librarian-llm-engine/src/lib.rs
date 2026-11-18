//! Everything the Librarian does with a model: the claims contract a query carries and the
//! mechanical check of each answer (`claims`), the library of tasks, profiles and languages a
//! Modelfile is rendered from (`models`), and the providers a query runs on (`transport`, `ollama`).
//! Transport completion is not semantic approval. No model or domain is implicit.
pub mod claims;
pub mod cli;
pub mod models;
pub mod ollama;
pub mod transport;
