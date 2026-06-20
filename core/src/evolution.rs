//! Evoluzione architetturale: confronto tra due versioni del progetto e
//! analisi d'impatto.
//!
//! Dato un modello "prima" e uno "dopo" (tipicamente due [`crate::store`]
//! snapshot), calcola cosa è stato aggiunto/rimosso/modificato e, per ogni
//! elemento cambiato, chi ne dipende (impatto), risalendo il grafo delle
//! relazioni a ritroso.

use crate::model::Project;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// Un singolo elemento cambiato.
#[derive(Debug, Clone, Serialize)]
pub struct EntityChange {
    /// Categoria: "component", "endpoint", "service", "table", "dependency".
    pub kind: String,
    pub id: String,
    pub label: String,
}

/// L'impatto di un cambiamento: chi dipende (anche transitivamente) dall'elemento.
#[derive(Debug, Clone, Serialize)]
pub struct Impact {
    pub changed: String,
    pub label: String,
    /// Etichette dei dipendenti (a monte) impattati.
    pub dependents: Vec<String>,
}

/// Il risultato del confronto tra due versioni.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChangeSet {
    pub added: Vec<EntityChange>,
    pub removed: Vec<EntityChange>,
    pub modified: Vec<EntityChange>,
    pub impacted: Vec<Impact>,
}

/// Una entità del progetto con la sua "firma" (per rilevare le modifiche).
struct Entity {
    kind: &'static str,
    label: String,
    signature: String,
}

/// Confronta due progetti e produce l'insieme dei cambiamenti + l'impatto.
pub fn diff(old: &Project, new: &Project) -> ChangeSet {
    let old_e = entities(old);
    let new_e = entities(new);

    let mut cs = ChangeSet::default();

    // Aggiunti e modificati.
    for (id, ne) in &new_e {
        match old_e.get(id) {
            None => cs.added.push(EntityChange {
                kind: ne.kind.into(),
                id: id.clone(),
                label: ne.label.clone(),
            }),
            Some(oe) if oe.signature != ne.signature => cs.modified.push(EntityChange {
                kind: ne.kind.into(),
                id: id.clone(),
                label: ne.label.clone(),
            }),
            _ => {}
        }
    }
    // Rimossi.
    for (id, oe) in &old_e {
        if !new_e.contains_key(id) {
            cs.removed.push(EntityChange {
                kind: oe.kind.into(),
                id: id.clone(),
                label: oe.label.clone(),
            });
        }
    }

    // Impatto: dipendenti a monte di ogni elemento cambiato.
    // Adiacenza inversa unendo le relazioni di entrambe le versioni
    // (così copre anche gli elementi rimossi).
    let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();
    for r in old.relations.iter().chain(new.relations.iter()) {
        reverse.entry(r.to.as_str()).or_default().push(r.from.as_str());
    }
    let label_of = |id: &str| {
        new_e
            .get(id)
            .or_else(|| old_e.get(id))
            .map(|e| e.label.clone())
            .unwrap_or_else(|| id.to_string())
    };

    let changed_ids: Vec<String> = cs
        .modified
        .iter()
        .chain(cs.removed.iter())
        .map(|c| c.id.clone())
        .collect();

    for cid in changed_ids {
        let deps = upstream(&reverse, &cid);
        if !deps.is_empty() {
            cs.impacted.push(Impact {
                changed: cid.clone(),
                label: label_of(&cid),
                dependents: deps.iter().map(|d| label_of(d)).collect(),
            });
        }
    }

    cs
}

/// Risalita a ritroso (BFS) per trovare tutti i dipendenti di `start`.
fn upstream(reverse: &HashMap<&str, Vec<&str>>, start: &str) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(start.to_string());
    while let Some(node) = queue.pop_front() {
        if let Some(parents) = reverse.get(node.as_str()) {
            for p in parents {
                if *p != start && seen.insert(p.to_string()) {
                    queue.push_back(p.to_string());
                }
            }
        }
        if seen.len() > 200 {
            break; // salvaguardia su grafi enormi
        }
    }
    seen.into_iter().collect()
}

/// Indicizza tutte le entità di un progetto per id, con firma per il confronto.
fn entities(p: &Project) -> BTreeMap<String, Entity> {
    let mut m = BTreeMap::new();

    for c in &p.components {
        let mut members = c.members.clone();
        members.sort();
        m.insert(
            c.id.clone(),
            Entity {
                kind: "component",
                label: c.name.clone(),
                signature: format!("{:?}|{:?}|{}", c.kind, c.language, members.join(",")),
            },
        );
    }
    for e in &p.endpoints {
        m.insert(
            e.id.clone(),
            Entity {
                kind: "endpoint",
                label: format!("{} {}", e.method, e.path),
                signature: format!(
                    "{}|{}|{}",
                    e.method,
                    e.path,
                    e.operation_id.clone().unwrap_or_default()
                ),
            },
        );
    }
    for s in &p.services {
        let mut dep = s.depends_on.clone();
        dep.sort();
        m.insert(
            s.id.clone(),
            Entity {
                kind: "service",
                label: s.name.clone(),
                signature: format!(
                    "{}|{}|{}",
                    s.image.clone().unwrap_or_default(),
                    s.ports.join(","),
                    dep.join(",")
                ),
            },
        );
    }
    for t in &p.tables {
        let cols: Vec<String> = t
            .columns
            .iter()
            .map(|c| format!("{}:{}:{}", c.name, c.data_type, c.primary_key))
            .collect();
        let fks: Vec<String> = t
            .foreign_keys
            .iter()
            .map(|f| format!("{}->{}", f.column, f.references_table))
            .collect();
        m.insert(
            t.id.clone(),
            Entity {
                kind: "table",
                label: t.name.clone(),
                signature: format!("{}|{}", cols.join(","), fks.join(",")),
            },
        );
    }
    for d in &p.dependencies {
        let id = format!("dep:{}:{}", d.ecosystem, d.name);
        m.insert(
            id,
            Entity {
                kind: "dependency",
                label: format!("{} ({})", d.name, d.ecosystem),
                signature: d.version.clone().unwrap_or_default(),
            },
        );
    }

    m
}
