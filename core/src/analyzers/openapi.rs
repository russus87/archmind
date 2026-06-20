//! Analisi OpenAPI/Swagger: estrae gli endpoint (metodo + path + operationId)
//! da specifiche in YAML o JSON.
//!
//! Riconosce un file come spec se contiene una chiave `openapi`/`swagger` di
//! primo livello e un oggetto `paths`. Cosi' evita di scambiare un YAML
//! qualsiasi per una API.

use super::ext;
use super::rel;
use crate::model::{Endpoint, Project};
use serde_json::Value;
use std::path::PathBuf;

const METHODS: &[&str] = &["get", "post", "put", "delete", "patch", "head", "options"];

/// Estrae gli endpoint da tutte le specifiche OpenAPI/Swagger trovate.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf]) {
    for path in files {
        let e = ext(path);
        if !matches!(e.as_str(), "yaml" | "yml" | "json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };

        let Some(spec) = parse(&text, &e) else {
            continue;
        };
        let is_spec = spec.get("openapi").is_some() || spec.get("swagger").is_some();
        let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) else {
            continue;
        };
        if !is_spec {
            continue;
        }

        let source = rel(root, path);
        for (route, item) in paths {
            let Some(ops) = item.as_object() else { continue };
            for (method, op) in ops {
                if !METHODS.contains(&method.to_lowercase().as_str()) {
                    continue;
                }
                project.endpoints.push(Endpoint {
                    id: format!("ep:{} {}", method.to_uppercase(), route),
                    method: method.to_uppercase(),
                    path: route.clone(),
                    operation_id: op
                        .get("operationId")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    summary: op.get("summary").and_then(|v| v.as_str()).map(String::from),
                    source: source.clone(),
                });
            }
        }
    }
}

/// Converte testo YAML o JSON nel modello generico `serde_json::Value`.
fn parse(text: &str, ext: &str) -> Option<Value> {
    if ext == "json" {
        serde_json::from_str(text).ok()
    } else {
        serde_yaml::from_str(text).ok()
    }
}
