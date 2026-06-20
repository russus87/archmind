//! Render di diagrammi Mermaid dal modello di progetto.
//!
//! Tipi supportati: `dependency` (grafo dipendenze), `component` (servizi e
//! relazioni), `er` (Entity-Relationship del DB), `class` (Class Diagram),
//! `sequence` (bozza di flusso dagli endpoint).

use crate::model::{Project, RelationKind};
use crate::{Error, Result};
use std::fmt::Write;

/// Genera il sorgente Mermaid per il tipo di diagramma richiesto.
pub fn render(p: &Project, kind: &str) -> Result<String> {
    match kind {
        "dependency" => Ok(dependency(p)),
        "component" => Ok(component(p)),
        "er" => Ok(er(p)),
        "class" => Ok(class(p)),
        "sequence" => Ok(sequence(p)),
        other => Err(Error::UnknownDiagram(other.to_string())),
    }
}

/// Identificatore sicuro per i nodi Mermaid (solo alfanumerici e underscore).
fn id(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    if cleaned.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        format!("n_{cleaned}")
    } else {
        cleaned
    }
}

/// Grafo delle dipendenze: progetto -> pacchetti (raggruppati per ecosistema).
fn dependency(p: &Project) -> String {
    let mut s = String::from("graph LR\n");
    let root = id(&p.name);
    let _ = writeln!(s, "  {root}([\"{}\"])", p.name);
    // Limita per leggibilita': i grafi enormi si filtrano nella UI.
    for d in p.dependencies.iter().take(60) {
        let n = id(&format!("{}_{}", d.ecosystem, d.name));
        let ver = d.version.clone().unwrap_or_default();
        let _ = writeln!(s, "  {root} --> {n}[\"{}<br/>{}\"]", d.name, ver);
    }
    s
}

/// Diagramma dei componenti: servizi e relazioni depends-on/references.
fn component(p: &Project) -> String {
    let mut s = String::from("graph TD\n");
    for sv in &p.services {
        let n = id(&sv.id);
        let img = sv.image.clone().unwrap_or_default();
        let _ = writeln!(s, "  {n}[\"{}<br/><small>{}</small>\"]", sv.name, img);
    }
    for r in &p.relations {
        if matches!(r.kind, RelationKind::DependsOn) {
            let _ = writeln!(s, "  {} --> {}", id(&r.from), id(&r.to));
        }
    }
    if p.services.is_empty() {
        let _ = writeln!(s, "  empty[\"Nessun servizio rilevato\"]");
    }
    s
}

/// Diagramma ER: tabelle, colonne e relazioni da foreign key.
fn er(p: &Project) -> String {
    let mut s = String::from("erDiagram\n");
    for t in &p.tables {
        let name = id(&t.name);
        let _ = writeln!(s, "  {name} {{");
        for c in t.columns.iter().take(30) {
            let pk = if c.primary_key { " PK" } else { "" };
            let ty = id(&c.data_type);
            let _ = writeln!(s, "    {ty} {}{pk}", id(&c.name));
        }
        let _ = writeln!(s, "  }}");
    }
    for t in &p.tables {
        for fk in &t.foreign_keys {
            let _ = writeln!(
                s,
                "  {} ||--o{{ {} : \"{}\"",
                id(&fk.references_table),
                id(&t.name),
                fk.column
            );
        }
    }
    if p.tables.is_empty() {
        let _ = writeln!(s, "  NESSUNA_TABELLA {{ string nota }}");
    }
    s
}

/// Class Diagram: classi/interfacce con i loro metodi.
fn class(p: &Project) -> String {
    let mut s = String::from("classDiagram\n");
    let mut count = 0;
    for c in &p.components {
        use crate::model::ComponentKind::*;
        if !matches!(c.kind, Class | Interface) {
            continue;
        }
        count += 1;
        let name = id(&c.name);
        let _ = writeln!(s, "  class {name} {{");
        if matches!(c.kind, Interface) {
            let _ = writeln!(s, "    <<interface>>");
        }
        for m in c.members.iter().take(15) {
            let _ = writeln!(s, "    +{}()", id(m));
        }
        let _ = writeln!(s, "  }}");
        if count >= 40 {
            break; // i diagrammi giganti vanno filtrati nella UI
        }
    }
    if count == 0 {
        let _ = writeln!(s, "  class NessunaClasse");
    }
    s
}

/// Bozza di Sequence Diagram: il client che chiama gli endpoint dell'API.
fn sequence(p: &Project) -> String {
    let mut s = String::from("sequenceDiagram\n  participant Client\n  participant API\n");
    for e in p.endpoints.iter().take(25) {
        let _ = writeln!(s, "  Client->>API: {} {}", e.method, e.path);
        let _ = writeln!(s, "  API-->>Client: 200 OK");
    }
    if p.endpoints.is_empty() {
        let _ = writeln!(s, "  Client->>API: (nessun endpoint rilevato)");
    }
    s
}
