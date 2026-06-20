//! Export diagrammi in formato PlantUML. Adatto a class diagram, componenti e
//! sequence diagram.

use crate::model::{ComponentKind, Project, RelationKind};
use crate::{Error, Result};
use std::collections::HashMap;
use std::fmt::Write;

/// Genera il sorgente PlantUML per il tipo di diagramma richiesto.
pub fn render(p: &Project, kind: &str) -> Result<String> {
    match kind {
        "class" => Ok(class(p)),
        "component" => Ok(component(p)),
        "sequence" => Ok(sequence(p)),
        other => Err(Error::UnknownDiagram(format!("plantuml: {other}"))),
    }
}

/// Nome sicuro per PlantUML (alfanumerico/underscore).
fn id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

fn class(p: &Project) -> String {
    let mut s = String::from("@startuml\n");
    let mut count = 0;
    for c in &p.components {
        if !matches!(c.kind, ComponentKind::Class | ComponentKind::Interface) {
            continue;
        }
        let kw = if matches!(c.kind, ComponentKind::Interface) {
            "interface"
        } else {
            "class"
        };
        let _ = writeln!(s, "{kw} {} {{", id(&c.name));
        for m in c.members.iter().take(15) {
            let _ = writeln!(s, "  +{}()", id(m));
        }
        let _ = writeln!(s, "}}");
        count += 1;
        if count >= 50 {
            break;
        }
    }
    // Archi delle chiamate.
    let by_id: HashMap<&str, &str> = p.components.iter().map(|c| (c.id.as_str(), c.name.as_str())).collect();
    for r in p.relations.iter().filter(|r| matches!(r.kind, RelationKind::Calls)) {
        if let (Some(f), Some(t)) = (by_id.get(r.from.as_str()), by_id.get(r.to.as_str())) {
            let _ = writeln!(s, "{} ..> {}", id(f), id(t));
        }
    }
    s.push_str("@enduml\n");
    s
}

fn component(p: &Project) -> String {
    let mut s = String::from("@startuml\n");
    for sv in &p.services {
        let _ = writeln!(s, "component [{}]", sv.name);
    }
    for r in p.relations.iter().filter(|r| matches!(r.kind, RelationKind::DependsOn)) {
        let from = p.services.iter().find(|s| s.id == r.from).map(|s| s.name.clone());
        if let Some(from) = from {
            let to = r.to.strip_prefix("service:").unwrap_or(&r.to);
            let _ = writeln!(s, "[{from}] --> [{to}]");
        }
    }
    s.push_str("@enduml\n");
    s
}

fn sequence(p: &Project) -> String {
    let mut s = String::from("@startuml\nactor Client\nparticipant API\n");
    for e in p.endpoints.iter().take(25) {
        let _ = writeln!(s, "Client -> API: {} {}", e.method, e.path);
        let _ = writeln!(s, "API --> Client: 200 OK");
    }
    s.push_str("@enduml\n");
    s
}
