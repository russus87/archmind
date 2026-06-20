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
        ])
        .run(tauri::generate_context!())
        .expect("errore irreversibile all'avvio di ArchMind");
}
