//! Analisi manifest Kubernetes: riconosce i documenti YAML con `apiVersion` e
//! `kind` ed estrae l'unita' corrispondente (Deployment, Service, StatefulSet...).
//!
//! Un singolo file puo' contenere piu' documenti separati da `---`: vengono
//! processati tutti.

use super::{ext, rel};
use crate::model::{Project, ServiceUnit};
use serde_yaml::Value;
use std::path::PathBuf;

/// Estrae le risorse Kubernetes da tutti i manifest YAML.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf]) {
    for path in files {
        if !matches!(ext(path).as_str(), "yaml" | "yml") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let source = rel(root, path);

        // serde_yaml legge i multi-documento un pezzo alla volta.
        for doc in serde_yaml::Deserializer::from_str(&text) {
            let Ok(value) = Value::deserialize(doc) else {
                continue;
            };
            let (Some(kind), Some(name)) = (
                value.get("kind").and_then(|v| v.as_str()),
                value
                    .get("metadata")
                    .and_then(|m| m.get("name"))
                    .and_then(|v| v.as_str()),
            ) else {
                continue;
            };
            if value.get("apiVersion").is_none() {
                continue;
            }
            let image = first_image(&value);
            project.services.push(ServiceUnit {
                id: format!("k8s:{kind}/{name}"),
                name: format!("{kind}/{name}"),
                image,
                ports: vec![],
                depends_on: vec![],
                source: format!("kubernetes:{source}"),
            });
        }
    }
}

use serde::Deserialize;

/// Cerca la prima `image:` dichiarata nei container del manifest.
fn first_image(value: &Value) -> Option<String> {
    let containers = value
        .get("spec")?
        .get("template")?
        .get("spec")?
        .get("containers")?
        .as_sequence()?;
    containers
        .iter()
        .find_map(|c| c.get("image").and_then(|v| v.as_str()).map(String::from))
}
