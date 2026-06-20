//! Ricerca full-text semplice sugli elementi del progetto.
//!
//! L'MVP fa un match case-insensitive su nomi e percorsi di ogni entita'. In
//! roadmap questo modulo verra' sostituito da un indice [tantivy](https://github.com/quickwit-oss/tantivy)
//! per la ricerca full-text vera e da un indice vettoriale per quella semantica.

use crate::model::Project;
use serde::Serialize;

/// Un risultato di ricerca.
#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    /// Categoria dell'elemento: "component", "endpoint", "service", "table", "dependency".
    pub kind: String,
    /// Etichetta da mostrare.
    pub label: String,
    /// Posizione (file o sorgente) dell'elemento.
    pub location: String,
}

/// Cerca `query` tra tutte le entita' del progetto e restituisce i risultati.
pub fn search(project: &Project, query: &str) -> Vec<Hit> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return vec![];
    }
    let mut hits = Vec::new();
    let m = |s: &str| s.to_lowercase().contains(&q);

    for c in &project.components {
        if m(&c.name) || m(&c.path) || c.members.iter().any(|x| m(x)) {
            hits.push(Hit {
                kind: "component".into(),
                label: c.name.clone(),
                location: c.path.clone(),
            });
        }
    }
    for e in &project.endpoints {
        if m(&e.path) || m(&e.method) || e.operation_id.as_deref().map(m).unwrap_or(false) {
            hits.push(Hit {
                kind: "endpoint".into(),
                label: format!("{} {}", e.method, e.path),
                location: e.source.clone(),
            });
        }
    }
    for s in &project.services {
        if m(&s.name) || s.image.as_deref().map(m).unwrap_or(false) {
            hits.push(Hit {
                kind: "service".into(),
                label: s.name.clone(),
                location: s.source.clone(),
            });
        }
    }
    for t in &project.tables {
        if m(&t.name) || t.columns.iter().any(|col| m(&col.name)) {
            hits.push(Hit {
                kind: "table".into(),
                label: t.name.clone(),
                location: t.schema.clone().unwrap_or_default(),
            });
        }
    }
    for d in &project.dependencies {
        if m(&d.name) {
            hits.push(Hit {
                kind: "dependency".into(),
                label: d.name.clone(),
                location: d.declared_in.clone(),
            });
        }
    }

    hits
}
