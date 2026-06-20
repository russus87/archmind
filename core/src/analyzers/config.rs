//! Analisi dei file di configurazione: rileva `.env`, `appsettings*.json`,
//! `application.properties` / `application*.yml` e li rappresenta come componenti
//! di tipo "modulo", cosi' compaiono nella documentazione e nella ricerca.

use super::{file_name_lc, rel};
use crate::model::{Component, ComponentKind, Language, Project};
use std::path::PathBuf;

/// Registra i file di configurazione riconosciuti come componenti del progetto.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf]) {
    for path in files {
        let name = file_name_lc(path);
        let is_config = name == ".env"
            || name.starts_with(".env.")
            || (name.starts_with("appsettings") && name.ends_with(".json"))
            || name == "application.properties"
            || (name.starts_with("application") && (name.ends_with(".yml") || name.ends_with(".yaml")));
        if !is_config {
            continue;
        }

        let where_ = rel(root, path);
        let keys = count_keys(path, &name);
        project.components.push(Component {
            id: format!("config:{where_}"),
            name: format!("{name} ({keys} chiavi)"),
            kind: ComponentKind::Module,
            language: Language::Other,
            path: where_,
            members: vec![],
        });
    }
}

/// Conta (grossolanamente) le chiavi di configurazione di un file.
fn count_keys(path: &PathBuf, name: &str) -> usize {
    let Ok(text) = std::fs::read_to_string(path) else {
        return 0;
    };
    if name.ends_with(".json") {
        return serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.as_object().map(|o| o.len()))
            .unwrap_or(0);
    }
    // .env / .properties: righe `chiave=valore` non commentate.
    text.lines()
        .filter(|l| {
            let l = l.trim();
            !l.is_empty() && !l.starts_with('#') && l.contains('=')
        })
        .count()
}
