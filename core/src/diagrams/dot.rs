//! Export diagrammi in formato Graphviz (DOT). Adatto ai grafi: dipendenze,
//! flusso applicativo e componenti/servizi.

use crate::model::{Project, RelationKind};
use crate::{Error, Result};
use std::collections::HashMap;
use std::fmt::Write;

/// Genera il sorgente DOT per il tipo di diagramma richiesto.
pub fn render(p: &Project, kind: &str) -> Result<String> {
    match kind {
        "dependency" => Ok(dependency(p)),
        "flow" => Ok(flow(p)),
        "component" => Ok(component(p)),
        other => Err(Error::UnknownDiagram(format!("dot: {other}"))),
    }
}

/// Identificatore DOT sicuro (alfanumerico/underscore).
fn id(s: &str) -> String {
    let c: String = s
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { '_' })
        .collect();
    format!("n_{c}")
}

fn dependency(p: &Project) -> String {
    let mut s = String::from("digraph dependencies {\n  rankdir=LR;\n  node [shape=box, style=rounded];\n");
    let root = id(&p.name);
    let _ = writeln!(s, "  {root} [label=\"{}\", shape=ellipse];", esc(&p.name));
    for d in p.dependencies.iter().take(80) {
        let n = id(&format!("{}_{}", d.ecosystem, d.name));
        let _ = writeln!(s, "  {root} -> {n};");
        let _ = writeln!(s, "  {n} [label=\"{}\"];", esc(&d.name));
    }
    s.push_str("}\n");
    s
}

fn component(p: &Project) -> String {
    let mut s = String::from("digraph components {\n  node [shape=component];\n");
    for sv in &p.services {
        let _ = writeln!(s, "  {} [label=\"{}\"];", id(&sv.id), esc(&sv.name));
    }
    for r in p.relations.iter().filter(|r| matches!(r.kind, RelationKind::DependsOn)) {
        let _ = writeln!(s, "  {} -> {};", id(&r.from), id(&r.to));
    }
    s.push_str("}\n");
    s
}

fn flow(p: &Project) -> String {
    let mut s = String::from("digraph flow {\n  rankdir=LR;\n  node [shape=box, style=rounded];\n");
    let mut label: HashMap<&str, String> = HashMap::new();
    for c in &p.components {
        label.insert(c.id.as_str(), c.name.clone());
    }
    for e in &p.endpoints {
        label.insert(e.id.as_str(), format!("{} {}", e.method, e.path));
    }
    for t in &p.tables {
        label.insert(t.id.as_str(), t.name.clone());
    }
    let emit = |s: &mut String, from: &str, to: &str, lf: &str, lt: &str| {
        let _ = writeln!(s, "  {} [label=\"{}\"];", id(from), esc(lf));
        let _ = writeln!(s, "  {} [label=\"{}\"];", id(to), esc(lt));
        let _ = writeln!(s, "  {} -> {};", id(from), id(to));
    };
    for r in &p.relations {
        match (label.get(r.from.as_str()), label.get(r.to.as_str())) {
            (Some(lf), Some(lt)) => match r.kind {
                RelationKind::Exposes => emit(&mut s, &r.to, &r.from, lt, lf),
                RelationKind::References if r.to.starts_with("table:") => {
                    emit(&mut s, &r.from, &r.to, lf, lt)
                }
                _ => {}
            },
            _ => {}
        }
    }
    s.push_str("}\n");
    s
}

/// Escape per le label DOT.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
