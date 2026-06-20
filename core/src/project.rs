//! Orchestrazione dell'analisi: scansiona la cartella di progetto, lancia tutti
//! gli analyzer e fonde i risultati in un unico [`Project`].

use crate::analyzers;
use crate::model::{Project, RelationKind, Relation};
use crate::Result;
use ignore::WalkBuilder;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Cartelle da ignorare durante la scansione (rumore o artefatti di build).
const SKIP_DIRS: &[&str] = &[
    ".git", ".archmind", "node_modules", "target", "bin", "obj", "dist", ".vs",
    ".idea", "__pycache__", ".gradle", "build",
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
    // Rispetta `.gitignore` (via la crate `ignore`): si analizza ciò che è
    // versionato, non gli artefatti di build (target/, gen/, node_modules/...).
    // `hidden(false)` mantiene i dotfile utili (.env, appsettings).
    // L'ordinamento rende l'analisi — e quindi la documentazione generata —
    // deterministica a prescindere dal filesystem: indispensabile per il
    // confronto "docs-as-code" in CI.
    let mut files: Vec<_> = WalkBuilder::new(root_path)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .require_git(false)
        .filter_entry(|e| !is_skipped(e.path()))
        .build()
        .filter_map(|r| r.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.into_path())
        .collect();
    files.sort();

    analyzers::stats::collect(&mut project, &files);
    analyzers::git::collect(&mut project, root_path);
    analyzers::deps::collect(&mut project, root, &files);
    analyzers::csharp::collect(&mut project, root, &files);
    analyzers::java::collect(&mut project, root, &files);
    analyzers::treesitter::collect(&mut project, root, &files, analyzers::treesitter::Lang::TypeScript);
    analyzers::treesitter::collect(&mut project, root, &files, analyzers::treesitter::Lang::Python);
    analyzers::treesitter::collect(&mut project, root, &files, analyzers::treesitter::Lang::Go);
    analyzers::openapi::collect(&mut project, root, &files);
    analyzers::docker_compose::collect(&mut project, root, &files);
    analyzers::kubernetes::collect(&mut project, root, &files);
    analyzers::config::collect(&mut project, root, &files);
    analyzers::database::collect(&mut project, root, &files);

    derive_relations(&mut project);
    link_layers(&mut project);
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

/// Collega i livelli dell'applicazione (il "flusso applicativo"):
/// endpoint → componente che lo gestisce → tabella che il componente usa.
///
/// È un linking euristico ma utile: l'endpoint si lega al componente il cui
/// metodo combacia con l'`operationId` (o l'ultimo segmento del path); il
/// componente si lega a una tabella se il suo file ne nomina il nome.
fn link_layers(project: &mut Project) {
    // Mappa nome-metodo (minuscolo) -> id del componente proprietario.
    let mut method_owner: HashMap<String, String> = HashMap::new();
    for c in &project.components {
        for m in &c.members {
            method_owner
                .entry(m.to_lowercase())
                .or_insert_with(|| c.id.clone());
        }
    }

    let mut rels: HashSet<(String, String, RelationKind)> = HashSet::new();

    // Endpoint -> componente che lo espone.
    for e in &project.endpoints {
        let mut handler = e
            .operation_id
            .as_ref()
            .and_then(|op| method_owner.get(&op.to_lowercase()))
            .cloned();
        if handler.is_none() {
            // Fallback: ultimo segmento "parlante" del path (non un {param}).
            if let Some(seg) = e
                .path
                .rsplit('/')
                .find(|s| !s.is_empty() && !s.starts_with('{'))
            {
                handler = method_owner.get(&seg.to_lowercase()).cloned();
            }
        }
        if let Some(cid) = handler {
            rels.insert((cid, e.id.clone(), RelationKind::Exposes));
        }
    }

    // Componente -> tabella se il file del componente ne nomina il nome.
    if !project.tables.is_empty() {
        let tables: Vec<(String, String)> = project
            .tables
            .iter()
            .filter(|t| t.name.len() >= 3)
            .map(|t| (t.name.to_lowercase(), t.id.clone()))
            .collect();

        let mut file_cache: HashMap<String, String> = HashMap::new();
        for c in &project.components {
            if c.path.is_empty() {
                continue;
            }
            let content = file_cache.entry(c.path.clone()).or_insert_with(|| {
                std::fs::read_to_string(Path::new(&project.root).join(&c.path))
                    .unwrap_or_default()
                    .to_lowercase()
            });
            for (tname, tid) in &tables {
                if contains_word(content, tname) {
                    rels.insert((c.id.clone(), tid.clone(), RelationKind::References));
                }
            }
        }
    }

    for (from, to, kind) in rels {
        project.relations.push(Relation { from, to, kind });
    }
}

/// Verifica se `needle` compare in `haystack` come parola intera
/// (delimitata da caratteri non alfanumerici). Entrambi gia' in minuscolo.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let i = start + pos;
        let before_ok = i == 0 || !is_word_byte(bytes[i - 1]);
        let after = i + needle.len();
        let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
        start = i + needle.len();
    }
    false
}

/// Un byte fa parte di una "parola" (alfanumerico o underscore)?
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
