//! Analisi del database: estrae tabelle, colonne e foreign key dai file DDL
//! (`.sql`). Supporta la sintassi comune a Oracle e PostgreSQL per `CREATE TABLE`.
//!
//! L'analisi via connessione live (introspezione di `information_schema` /
//! dizionario dati Oracle) e' prevista in roadmap; qui si parte dal DDL su file,
//! che copre la maggior parte dei progetti versionati.

use super::ext;
use crate::model::{Column, ForeignKey, Project, Table};
use std::path::PathBuf;

/// Estrae il modello dati da tutti i file `.sql`.
pub fn collect(project: &mut Project, _root: &str, files: &[PathBuf]) {
    // CREATE TABLE [schema.]nome ( ...corpo... )
    let table_re = regex::Regex::new(
        r#"(?is)create\s+table\s+(?:if\s+not\s+exists\s+)?["`\[]?([\w.]+)["`\]]?\s*\((.*?)\)\s*;"#,
    )
    .unwrap();

    for path in files {
        if ext(path) != "sql" {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for c in table_re.captures_iter(&text) {
            let raw = c[1].to_string();
            let (schema, name) = split_schema(&raw);
            let body = &c[2];
            let (columns, foreign_keys) = parse_body(body);
            project.tables.push(Table {
                id: format!("table:{}", name.to_lowercase()),
                name,
                schema,
                columns,
                foreign_keys,
            });
        }
    }
}

/// Divide `schema.tabella` nei due pezzi (schema opzionale).
fn split_schema(raw: &str) -> (Option<String>, String) {
    match raw.rsplit_once('.') {
        Some((s, t)) => (Some(s.to_string()), t.to_string()),
        None => (None, raw.to_string()),
    }
}

/// Analizza il corpo di un `CREATE TABLE` (le righe tra parentesi).
fn parse_body(body: &str) -> (Vec<Column>, Vec<ForeignKey>) {
    let mut columns = Vec::new();
    let mut fks = Vec::new();

    let fk_re = regex::Regex::new(
        r#"(?i)foreign\s+key\s*\(\s*["`\[]?(\w+)["`\]]?\s*\)\s*references\s+["`\[]?([\w.]+)["`\]]?\s*\(\s*["`\[]?(\w+)["`\]]?\s*\)"#,
    )
    .unwrap();

    for line in split_top_level(body) {
        let l = line.trim();
        let lower = l.to_lowercase();
        if lower.is_empty() {
            continue;
        }

        // Vincolo di foreign key esplicito.
        if let Some(c) = fk_re.captures(l) {
            let (_, table) = split_schema(&c[2]);
            fks.push(ForeignKey {
                column: c[1].to_string(),
                references_table: table,
                references_column: c[3].to_string(),
            });
            continue;
        }

        // Righe di vincolo a livello tabella: le saltiamo come colonne.
        if lower.starts_with("primary key")
            || lower.starts_with("constraint")
            || lower.starts_with("unique")
            || lower.starts_with("foreign key")
            || lower.starts_with("check")
        {
            continue;
        }

        // Riga di colonna: "nome TIPO ...".
        let mut parts = l.split_whitespace();
        let Some(col_name) = parts.next() else { continue };
        let data_type = parts.next().unwrap_or("?").trim_end_matches(',').to_string();
        columns.push(Column {
            name: col_name.trim_matches(|c| c == '"' || c == '`' || c == '[' || c == ']').to_string(),
            data_type,
            nullable: !lower.contains("not null"),
            primary_key: lower.contains("primary key"),
        });

        // Foreign key in linea: "col TIPO REFERENCES altra(col)".
        if let Some(idx) = lower.find("references") {
            let after = &l[idx + "references".len()..];
            let inline = regex::Regex::new(r#"(?i)\s*["`\[]?([\w.]+)["`\]]?\s*\(\s*["`\[]?(\w+)["`\]]?\s*\)"#).unwrap();
            if let Some(c) = inline.captures(after) {
                let (_, table) = split_schema(&c[1]);
                fks.push(ForeignKey {
                    column: col_name.to_string(),
                    references_table: table,
                    references_column: c[2].to_string(),
                });
            }
        }
    }

    (columns, fks)
}

/// Divide il corpo sulle virgole di primo livello (ignora quelle dentro le
/// parentesi, es. `NUMBER(10,2)`).
fn split_top_level(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in body.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}
