//! Persistenza del progetto su file SQLite (`<root>/.archmind/store.db`).
//!
//! Serve a due cose: riaprire un progetto senza rianalizzarlo e tenere uno
//! **storico di snapshot** del modello, su cui si basa il confronto tra
//! versioni e l'analisi d'impatto ([`crate::evolution`]).

use crate::model::Project;
use crate::{Error, Result};
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Metadati di uno snapshot salvato (senza il modello completo).
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotMeta {
    pub id: i64,
    /// Timestamp Unix (secondi) della creazione.
    pub created_at: i64,
    /// Etichetta scelta dall'utente (es. "v1.2", "prima del refactor").
    pub label: String,
    /// Nome del progetto al momento dello snapshot.
    pub name: String,
}

/// Percorso del database per una cartella di progetto.
pub fn db_path(root: &str) -> PathBuf {
    Path::new(root).join(".archmind").join("store.db")
}

/// Apre (creando cartella e schema se serve) la connessione al database.
fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path).map_err(|e| Error::Index(e.to_string()))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS snapshots (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at INTEGER NOT NULL,
            label      TEXT NOT NULL DEFAULT '',
            name       TEXT NOT NULL DEFAULT '',
            json       TEXT NOT NULL
        );",
    )
    .map_err(|e| Error::Index(e.to_string()))?;
    Ok(conn)
}

/// Salva uno snapshot del progetto e restituisce il suo id.
pub fn save_snapshot(root: &str, project: &Project, label: &str) -> Result<i64> {
    let conn = open(&db_path(root))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let json = serde_json::to_string(project)?;
    conn.execute(
        "INSERT INTO snapshots (created_at, label, name, json) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![now, label, project.name, json],
    )
    .map_err(|e| Error::Index(e.to_string()))?;
    Ok(conn.last_insert_rowid())
}

/// Elenca gli snapshot salvati, dal piu' recente.
pub fn list_snapshots(root: &str) -> Result<Vec<SnapshotMeta>> {
    let conn = open(&db_path(root))?;
    let mut stmt = conn
        .prepare("SELECT id, created_at, label, name FROM snapshots ORDER BY id DESC")
        .map_err(|e| Error::Index(e.to_string()))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(SnapshotMeta {
                id: r.get(0)?,
                created_at: r.get(1)?,
                label: r.get(2)?,
                name: r.get(3)?,
            })
        })
        .map_err(|e| Error::Index(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| Error::Index(e.to_string()))?);
    }
    Ok(out)
}

/// Carica il modello completo di uno snapshot per id.
pub fn load_snapshot(root: &str, id: i64) -> Result<Project> {
    let conn = open(&db_path(root))?;
    let json: String = conn
        .query_row("SELECT json FROM snapshots WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|e| Error::Index(e.to_string()))?;
    Ok(serde_json::from_str(&json)?)
}
