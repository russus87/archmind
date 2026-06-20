//! Orchestrazione dell'analisi: scansiona la cartella di progetto, lancia tutti
//! gli analyzer e fonde i risultati in un unico [`Project`].

use crate::analyzers;
use crate::model::{Project, RelationKind, Relation};
use crate::Result;
use std::path::Path;
use walkdir::WalkDir;

/// Cartelle da ignorare durante la scansione (rumore o artefatti di build).
const SKIP_DIRS: &[&str] = &[
    ".git", "node_modules", "target", "bin", "obj", "dist", ".vs", ".idea",
    "__pycache__", ".gradle", "build",
];

/// Analizza un progetto a partire dalla sua cartella radice.
///
/// Esegue, in ordine: statistiche dei file, dipendenze, codice (C#/Java),
/// OpenAPI, Docker Compose, Kubernetes, config e DDL del database. Ogni
/// analyzer e' tollerante agli errori: un file malformato non blocca l'analisi.
pub fn analyze(root: &str) -> Result<Project> {
    let root_path = Path::new(root);
    if !root_path.is_dir() {
        return Err(crate::Error::BadPath(root.to_string()));
    }

    let name = root_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("progetto")
        .to_string();

    let mut project = Project::new(root, name);

    // Raccoglie tutti i file una volta sola e li passa agli analyzer.
    // L'ordinamento rende l'analisi (e quindi la documentazione generata)
    // deterministica a prescindere dall'ordine del filesystem: indispensabile
    // per il confronto "docs-as-code" in CI.
    let mut files: Vec<_> = WalkDir::new(root_path)
        .into_iter()
        .filter_entry(|e| !is_skipped(e.path()))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();
    files.sort();

    analyzers::stats::collect(&mut project, &files);
    analyzers::git::collect(&mut project, root_path);
    analyzers::deps::collect(&mut project, root, &files);
    analyzers::csharp::collect(&mut project, root, &files);
    analyzers::java::collect(&mut project, root, &files);
    analyzers::openapi::collect(&mut project, root, &files);
    analyzers::docker_compose::collect(&mut project, root, &files);
    analyzers::kubernetes::collect(&mut project, root, &files);
    analyzers::config::collect(&mut project, root, &files);
    analyzers::database::collect(&mut project, root, &files);

    derive_relations(&mut project);
    Ok(project)
}

/// Verifica se un percorso ricade in una cartella da saltare.
fn is_skipped(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| SKIP_DIRS.contains(&n))
        .unwrap_or(false)
}

/// Deriva archi del grafo dalle entita' gia' raccolte (es. service -> service
/// per i `depends_on`, table -> table per le foreign key).
fn derive_relations(project: &mut Project) {
    let mut rels: Vec<Relation> = Vec::new();

    for s in &project.services {
        for dep in &s.depends_on {
            rels.push(Relation {
                from: s.id.clone(),
                to: format!("service:{dep}"),
                kind: RelationKind::DependsOn,
            });
        }
    }
    for t in &project.tables {
        for fk in &t.foreign_keys {
            rels.push(Relation {
                from: t.id.clone(),
                to: format!("table:{}", fk.references_table.to_lowercase()),
                kind: RelationKind::References,
            });
        }
    }

    project.relations.extend(rels);
}
