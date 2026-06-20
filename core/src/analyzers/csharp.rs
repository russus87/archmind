//! Analisi C# (euristica): da ogni file `.cs` estrae namespace, classi e
//! interfacce, con i metodi pubblici come membri per i Class Diagram.
//!
//! Niente compilatore Roslyn: si usano regex robuste per l'MVP. L'analisi
//! semantica completa (via sidecar Roslyn) e' prevista in roadmap.

use super::{ext, rel};
use crate::model::{Component, ComponentKind, Language, Project};
use std::path::PathBuf;

/// Estrae i componenti C# da tutti i file `.cs`.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf]) {
    let ns_re = regex::Regex::new(r"(?m)^\s*namespace\s+([\w.]+)").unwrap();
    let type_re =
        regex::Regex::new(r"(?m)^\s*(?:public|internal|sealed|abstract|static|partial|\s)*\b(class|interface)\s+(\w+)")
            .unwrap();
    let method_re =
        regex::Regex::new(r"(?m)^\s*public\s+(?:static\s+|async\s+|virtual\s+|override\s+)*[\w<>\[\],?]+\s+(\w+)\s*\(")
            .unwrap();

    for path in files {
        if ext(path) != "cs" {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let where_ = rel(root, path);

        if let Some(c) = ns_re.captures(&text) {
            let ns = c[1].to_string();
            push_unique(project, Component {
                id: format!("ns:{ns}"),
                name: ns,
                kind: ComponentKind::Namespace,
                language: Language::CSharp,
                path: where_.clone(),
                members: vec![],
            });
        }

        let methods: Vec<String> = method_re
            .captures_iter(&text)
            .map(|m| m[1].to_string())
            .collect();

        for c in type_re.captures_iter(&text) {
            let kind = if &c[1] == "interface" {
                ComponentKind::Interface
            } else {
                ComponentKind::Class
            };
            let name = c[2].to_string();
            push_unique(project, Component {
                id: format!("cs:{name}"),
                name,
                kind,
                language: Language::CSharp,
                path: where_.clone(),
                members: methods.clone(),
            });
        }
    }
}

/// Aggiunge un componente solo se l'id non e' gia' presente.
fn push_unique(project: &mut Project, comp: Component) {
    if !project.components.iter().any(|c| c.id == comp.id) {
        project.components.push(comp);
    }
}
