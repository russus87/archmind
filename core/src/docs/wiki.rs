//! Export "Wiki": Markdown arricchito con i diagrammi incorporati come blocchi
//! ```mermaid```, pronto per i wiki che li renderizzano (GitHub/GitLab Wiki,
//! Azure DevOps Wiki).

use crate::diagrams::mermaid;
use crate::model::Project;
use std::fmt::Write;

/// Diagrammi inclusi nell'export Wiki, con titolo.
const DIAGRAMS: &[(&str, &str)] = &[
    ("flow", "Flusso applicativo"),
    ("component", "Componenti e servizi"),
    ("dependency", "Dipendenze"),
    ("er", "Modello dati (ER)"),
    ("class", "Class diagram"),
];

/// Produce la documentazione in formato Wiki (Markdown + Mermaid).
pub fn render(p: &Project) -> String {
    let mut s = super::markdown::render(p);

    let _ = writeln!(s, "\n## Diagrammi\n");
    for (kind, title) in DIAGRAMS {
        if let Ok(diagram) = mermaid::render(p, kind) {
            let _ = writeln!(s, "### {title}\n");
            let _ = writeln!(s, "```mermaid\n{}\n```\n", diagram.trim_end());
        }
    }
    s
}
