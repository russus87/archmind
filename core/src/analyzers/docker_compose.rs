//! Analisi Docker Compose: estrae i servizi (immagine, porte, depends_on)
//! dai file `docker-compose*.yml` / `compose*.yml`.

use super::{file_name_lc, rel};
use crate::model::{Project, ServiceUnit};
use serde_yaml::Value;
use std::path::PathBuf;

/// Estrae i servizi da tutti i file compose riconosciuti.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf]) {
    for path in files {
        let name = file_name_lc(path);
        let is_compose = (name.starts_with("docker-compose") || name.starts_with("compose"))
            && (name.ends_with(".yml") || name.ends_with(".yaml"));
        if !is_compose {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(doc) = serde_yaml::from_str::<Value>(&text) else {
            continue;
        };
        let Some(services) = doc.get("services").and_then(|s| s.as_mapping()) else {
            continue;
        };

        let source = rel(root, path);
        for (key, svc) in services {
            let Some(sname) = key.as_str() else { continue };
            project.services.push(ServiceUnit {
                id: format!("service:{sname}"),
                name: sname.to_string(),
                image: svc.get("image").and_then(|v| v.as_str()).map(String::from),
                ports: seq_of_strings(svc.get("ports")),
                depends_on: depends_on(svc.get("depends_on")),
                source: format!("docker-compose:{source}"),
            });
        }
    }
}

/// Converte una sequenza YAML in `Vec<String>` (numeri inclusi).
fn seq_of_strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|i| match i {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `depends_on` puo' essere una lista oppure una mappa (forma estesa).
fn depends_on(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Sequence(_)) => seq_of_strings(v),
        Some(Value::Mapping(m)) => m.keys().filter_map(|k| k.as_str().map(String::from)).collect(),
        _ => vec![],
    }
}
