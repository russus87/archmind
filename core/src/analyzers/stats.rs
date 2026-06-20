//! Statistiche di base: conteggio file, righe di codice e file per estensione.

use super::ext;
use crate::model::Project;
use std::path::PathBuf;

/// Estensioni considerate "codice" ai fini del conteggio LOC.
const CODE_EXT: &[&str] = &[
    "cs", "java", "sql", "ts", "js", "py", "go", "rs", "kt", "xml", "yaml", "yml", "json",
];

/// Popola [`Project::stats`] a partire dall'elenco dei file.
pub fn collect(project: &mut Project, files: &[PathBuf]) {
    let stats = &mut project.stats;
    stats.files = files.len();

    for path in files {
        let e = ext(path);
        if !e.is_empty() {
            *stats.by_extension.entry(e.clone()).or_insert(0) += 1;
        }
        if CODE_EXT.contains(&e.as_str()) {
            if let Ok(text) = std::fs::read_to_string(path) {
                stats.lines_of_code += text.lines().count();
            }
        }
    }
}
