//! Gli analyzer: ognuno estrae un tipo di informazione e la scrive nel
//! [`crate::model::Project`]. Tutti seguono la stessa firma `collect(...)` e
//! sono progettati per fallire in silenzio sui singoli file malformati: un
//! input sporco non deve mai interrompere l'analisi complessiva.

pub mod config;
pub mod csharp;
pub mod database;
pub mod deps;
pub mod docker_compose;
pub mod git;
pub mod java;
pub mod kubernetes;
pub mod openapi;
pub mod stats;

use std::path::Path;

/// Percorso relativo alla radice del progetto, con separatori `/` normalizzati.
/// Utile per identificatori stabili e leggibili a prescindere dal sistema.
pub(crate) fn rel(root: &str, path: &Path) -> String {
    let p = path.strip_prefix(root).unwrap_or(path);
    p.to_string_lossy().replace('\\', "/")
}

/// Estensione del file in minuscolo, senza punto ("" se assente).
pub(crate) fn ext(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Nome del file in minuscolo ("" se assente).
pub(crate) fn file_name_lc(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase()
}
