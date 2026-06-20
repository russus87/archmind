//! # archmind-core
//!
//! Tutta la logica di analisi di ArchMind, in Rust puro (niente Tauri).
//!
//! Il flusso e' sempre lo stesso: [`project::analyze`] scansiona la cartella,
//! lancia gli [`analyzers`] in cascata e popola un unico [`model::Project`]
//! (il "grafo di conoscenza"). Da quel modello si generano poi documentazione
//! ([`docs`]), diagrammi ([`diagrams`]) e si fanno ricerche ([`search`]).
//!
//! - [`analyzers`]  estrattori per Git, C#, Java, DB, OpenAPI, Compose, K8s, config
//! - [`model`]      il modello dati condiviso (entita' + relazioni)
//! - [`project`]    orchestrazione: scansione e merge dei risultati
//! - [`docs`]       generazione documentazione (Markdown; HTML/PDF in roadmap)
//! - [`diagrams`]   generazione diagrammi (Mermaid: dipendenze, componenti, ER...)
//! - [`search`]     ricerca full-text sugli elementi del progetto

pub mod analyzers;
pub mod assistant;
pub mod db;
pub mod diagrams;
pub mod docs;
#[cfg(feature = "embeddings")]
pub mod embed;
pub mod evolution;
pub mod index;
pub mod model;
pub mod project;
pub mod search;
pub mod store;

/// Errori della libreria.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("errore di I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("percorso non valido: {0}")]
    BadPath(String),
    #[error("parsing fallito ({context}): {source}")]
    Parse {
        context: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("tipo di diagramma non supportato: {0}")]
    UnknownDiagram(String),
    #[error("errore dell'indice di ricerca: {0}")]
    Index(String),
    #[error("errore dell'assistente LLM: {0}")]
    Llm(String),
    #[error("serializzazione JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("errore database/connessione: {0}")]
    Db(String),
    #[error("errore di esportazione: {0}")]
    Export(String),
}

/// Alias comodo.
pub type Result<T> = std::result::Result<T, Error>;
