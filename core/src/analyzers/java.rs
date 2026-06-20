//! Analisi Java (euristica): da ogni file `.java` estrae il package, le classi
//! e le interfacce, con i metodi pubblici come membri.
//!
//! Niente compilatore: regex per l'MVP. L'analisi semantica completa (via
//! sidecar basato su JavaParser/symbol-solver) e' prevista in roadmap.

use super::{ext, rel};
use crate::model::{Component, ComponentKind, Language, Project};
use std::path::PathBuf;

/// Estrae i componenti Java da tutti i file `.java`.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf]) {
    let pkg_re = regex::Regex::new(r"(?m)^\s*package\s+([\w.]+)\s*;").unwrap();
    let type_re =
        regex::Regex::new(r"(?m)^\s*(?:public|final|abstract|\s)*\b(class|interface)\s+(\w+)")
            .unwrap();
    let method_re =
        regex::Regex::new(r"(?m)^\s*public\s+(?:static\s+|final\s+|synchronized\s+)*[\w<>\[\],?]+\s+(\w+)\s*\(")
            .unwrap();

    for path in files {
        if ext(path) != "java" {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let where_ = rel(root, path);

        if let Some(c) = pkg_re.captures(&text) {
            let pkg = c[1].to_string();
            push_unique(project, Component {
                id: format!("pkg:{pkg}"),
                name: pkg,
                kind: ComponentKind::Package,
                language: Language::Java,
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
                id: format!("java:{name}"),
                name,
                kind,
                language: Language::Java,
                path: where_.clone(),
                members: methods.clone(),
            });
        }
    }
}

fn push_unique(project: &mut Project, comp: Component) {
    if !project.components.iter().any(|c| c.id == comp.id) {
        project.components.push(comp);
    }
}
