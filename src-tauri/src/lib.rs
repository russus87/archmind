//! Backend Tauri di ArchMind: espone alla UI i comandi del crate `archmind-core`.
//!
//! Ogni comando e' un sottile adattatore: invoca l'analisi del core, serializza
//! il modello in JSON e trasforma gli errori in stringhe leggibili per il frontend.
//! Tutta la logica vera (scansione, parsing, documentazione, diagrammi) vive nel core.

use archmind_core::model::Project;
use archmind_core::{diagrams, docs};

/// Converte un errore del core in stringa per la UI.
fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// Analizza un progetto a partire dalla cartella radice e restituisce
/// il grafo di conoscenza completo (componenti, endpoint, servizi, tabelle...).
#[tauri::command]
fn analyze_project(root: String) -> Result<Project, String> {
    archmind_core::project::analyze(&root).map_err(err)
}

/// Genera la documentazione Markdown per un progetto gia' analizzato.
#[tauri::command]
fn generate_markdown(project: Project) -> String {
    docs::markdown::render(&project)
}

/// Genera un diagramma Mermaid del tipo richiesto:
/// "dependency" | "component" | "er" | "class" | "sequence".
#[tauri::command]
fn generate_diagram(project: Project, kind: String) -> Result<String, String> {
    diagrams::mermaid::render(&project, &kind).map_err(err)
}

/// Ricerca full-text semplice sugli elementi del progetto.
#[tauri::command]
fn search_project(project: Project, query: String) -> Vec<archmind_core::search::Hit> {
    archmind_core::search::search(&project, &query)
}

/// Assistente RAG: risponde a una domanda sul progetto usando il provider
/// scelto (Claude o Ollama). Recupera il contesto via indice tantivy.
#[tauri::command]
fn ask(
    project: Project,
    question: String,
    provider: archmind_core::assistant::Provider,
) -> Result<archmind_core::assistant::Answer, String> {
    archmind_core::assistant::ask(&project, &question, &provider).map_err(err)
}

/// Salva un testo (Markdown, HTML, diagramma...) sul percorso scelto dall'utente.
#[tauri::command]
fn save_text(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(err)
}

/// Esporta la documentazione nel formato richiesto ("md" | "html" | "wiki" | "pdf")
/// scrivendola sul percorso indicato.
#[tauri::command]
fn export_doc(project: Project, format: String, path: String) -> Result<(), String> {
    use archmind_core::docs;
    match format.as_str() {
        "md" => std::fs::write(&path, docs::markdown::render(&project)).map_err(err),
        "html" => std::fs::write(&path, docs::html::render(&project)).map_err(err),
        "wiki" => std::fs::write(&path, docs::wiki::render(&project)).map_err(err),
        "pdf" => {
            let bytes = docs::pdf::render(&project).map_err(err)?;
            std::fs::write(&path, bytes).map_err(err)
        }
        other => Err(format!("formato non supportato: {other}")),
    }
}

/// Salva uno snapshot del progetto (in <root>/.archmind/store.db) e ne restituisce l'id.
#[tauri::command]
fn save_snapshot(project: Project, label: String) -> Result<i64, String> {
    archmind_core::store::save_snapshot(&project.root, &project, &label).map_err(err)
}

/// Elenca gli snapshot salvati per un progetto.
#[tauri::command]
fn list_snapshots(root: String) -> Result<Vec<archmind_core::store::SnapshotMeta>, String> {
    archmind_core::store::list_snapshots(&root).map_err(err)
}

/// Confronta due snapshot e restituisce i cambiamenti + l'impatto.
#[tauri::command]
fn diff_snapshots(
    root: String,
    a: i64,
    b: i64,
) -> Result<archmind_core::evolution::ChangeSet, String> {
    let old = archmind_core::store::load_snapshot(&root, a).map_err(err)?;
    let new = archmind_core::store::load_snapshot(&root, b).map_err(err)?;
    Ok(archmind_core::evolution::diff(&old, &new))
}

/// Confronta uno snapshot salvato con lo stato attuale del progetto.
#[tauri::command]
fn diff_against_current(
    snapshot_id: i64,
    current: Project,
) -> Result<archmind_core::evolution::ChangeSet, String> {
    let old = archmind_core::store::load_snapshot(&current.root, snapshot_id).map_err(err)?;
    Ok(archmind_core::evolution::diff(&old, &current))
}

/// Introspezione live di un database PostgreSQL: aggiorna le tabelle del progetto.
#[tauri::command]
fn connect_database(
    mut project: Project,
    dsn: String,
    schema: Option<String>,
) -> Result<Project, String> {
    use archmind_core::model::{Relation, RelationKind};
    let tables = archmind_core::db::introspect_postgres(&dsn, schema.as_deref()).map_err(err)?;
    project.tables = tables;
    // Rigenera le relazioni table->table dalle nuove foreign key.
    project
        .relations
        .retain(|r| !(r.kind == RelationKind::References && r.from.starts_with("table:")));
    for t in &project.tables {
        for fk in &t.foreign_keys {
            project.relations.push(Relation {
                from: t.id.clone(),
                to: format!("table:{}", fk.references_table.to_lowercase()),
                kind: RelationKind::References,
            });
        }
    }
    Ok(project)
}

/// Punto di ingresso dell'app Tauri.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            analyze_project,
            generate_markdown,
            generate_diagram,
            search_project,
            ask,
            save_text,
            export_doc,
            save_snapshot,
            list_snapshots,
            diff_snapshots,
            diff_against_current,
            connect_database,
        ])
        .run(tauri::generate_context!())
        .expect("errore irreversibile all'avvio di ArchMind");
}
