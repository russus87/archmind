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

/// Ricerca full-text sugli elementi del progetto.
export async function searchProject(project, query) {
  return await invoke("search_project", { project, query });
}

/// Salva del testo su file, chiedendo il percorso all'utente.
export async function saveTextDialog(content, defaultName) {
  const path = await save({ defaultPath: defaultName });
  if (!path) return false;
  await invoke("save_text", { path, content });
  return true;
}
