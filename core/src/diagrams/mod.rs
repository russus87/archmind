//! Generazione diagrammi a partire dal modello di progetto.
//!
//! Per l'MVP si genera testo [Mermaid](https://mermaid.js.org) ([`mermaid`]),
//! che la UI rende a video e che e' incorporabile in Markdown/Wiki. L'export
//! verso PlantUML e Graphviz (DOT) e' previsto in roadmap.

pub mod dot;
pub mod mermaid;
pub mod plantuml;

use crate::model::Project;
use crate::{Error, Result};

/// Genera un diagramma nel formato richiesto ("mermaid" | "plantuml" | "dot").
pub fn render(project: &Project, kind: &str, format: &str) -> Result<String> {
    match format {
        "mermaid" => mermaid::render(project, kind),
        "plantuml" => plantuml::render(project, kind),
        "dot" => dot::render(project, kind),
        other => Err(Error::UnknownDiagram(format!("formato {other}"))),
    }
}

/// Estensione file per un formato diagramma.
pub fn extension(format: &str) -> &'static str {
    match format {
        "plantuml" => "puml",
        "dot" => "dot",
        _ => "mmd",
    }
}
