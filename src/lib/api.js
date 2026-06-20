// Sottile wrapper sui comandi Tauri del backend ArchMind.
// Tiene la UI ignara dei dettagli di `invoke` e della finestra di dialogo.

import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

/// Chiede all'utente una cartella e ne restituisce il percorso (o null).
export async function pickFolder() {
  return await open({ directory: true, multiple: false, title: "Scegli il progetto da analizzare" });
}

/// Analizza un progetto a partire dalla cartella radice.
export async function analyzeProject(root) {
  return await invoke("analyze_project", { root });
}

/// Genera la documentazione Markdown del progetto.
export async function generateMarkdown(project) {
  return await invoke("generate_markdown", { project });
}

/// Genera un diagramma Mermaid del tipo richiesto.
export async function generateDiagram(project, kind) {
  return await invoke("generate_diagram", { project, kind });
}

/// Genera un diagramma in un formato specifico (mermaid|plantuml|dot).
export async function generateDiagramFmt(project, kind, format) {
  return await invoke("generate_diagram_fmt", { project, kind, format });
}

/// Ricerca full-text sugli elementi del progetto.
export async function searchProject(project, query) {
  return await invoke("search_project", { project, query });
}

/// Assistente RAG: pone una domanda sul progetto.
/// `provider` è { kind: "claude", api_key, model } oppure { kind: "ollama", host, model }.
export async function ask(project, question, provider) {
  return await invoke("ask", { project, question, provider });
}

/// Salva del testo su file, chiedendo il percorso all'utente.
export async function saveTextDialog(content, defaultName) {
  const path = await save({ defaultPath: defaultName });
  if (!path) return false;
  await invoke("save_text", { path, content });
  return true;
}

/// Esporta la documentazione (md|html|wiki|pdf) chiedendo dove salvarla.
export async function exportDoc(project, format, defaultName) {
  const path = await save({ defaultPath: defaultName });
  if (!path) return false;
  await invoke("export_doc", { project, format, path });
  return true;
}

/// Salva uno snapshot del progetto con un'etichetta.
export async function saveSnapshot(project, label) {
  return await invoke("save_snapshot", { project, label });
}

/// Elenca gli snapshot salvati per la cartella radice.
export async function listSnapshots(root) {
  return await invoke("list_snapshots", { root });
}

/// Confronta due snapshot per id.
export async function diffSnapshots(root, a, b) {
  return await invoke("diff_snapshots", { root, a, b });
}

/// Confronta uno snapshot con lo stato attuale del progetto.
export async function diffAgainstCurrent(snapshotId, current) {
  return await invoke("diff_against_current", { snapshotId, current });
}

/// Introspezione live di un database PostgreSQL.
export async function connectDatabase(project, dsn, schema) {
  return await invoke("connect_database", { project, dsn, schema: schema || null });
}
