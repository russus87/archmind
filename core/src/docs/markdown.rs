//! Render della documentazione di progetto in Markdown.

use crate::model::Project;
use std::fmt::Write;

/// Produce un documento Markdown completo per il progetto analizzato.
pub fn render(p: &Project) -> String {
    let mut s = String::new();

    let _ = writeln!(s, "# {}\n", p.name);
    let _ = writeln!(s, "_Documentazione generata automaticamente da ArchMind._\n");
    let _ = writeln!(s, "- **Radice**: `{}`", p.root);
    let _ = writeln!(s, "- **File analizzati**: {}", p.stats.files);
    let _ = writeln!(s, "- **Righe di codice**: {}\n", p.stats.lines_of_code);

    // Composizione del progetto per estensione.
    if !p.stats.by_extension.is_empty() {
        let _ = writeln!(s, "## Composizione\n");
        let _ = writeln!(s, "| Estensione | File |");
        let _ = writeln!(s, "|---|---|");
        for (ext, n) in &p.stats.by_extension {
            if ext.starts_with("__") {
                continue; // metadati interni
            }
            let _ = writeln!(s, "| `{ext}` | {n} |");
        }
        let _ = writeln!(s);
    }

    section_endpoints(&mut s, p);
    section_services(&mut s, p);
    section_components(&mut s, p);
    section_database(&mut s, p);
    section_dependencies(&mut s, p);

    s
}

fn section_endpoints(s: &mut String, p: &Project) {
    if p.endpoints.is_empty() {
        return;
    }
    let _ = writeln!(s, "## API ({} endpoint)\n", p.endpoints.len());
    let _ = writeln!(s, "| Metodo | Path | Operazione |");
    let _ = writeln!(s, "|---|---|---|");
    for e in &p.endpoints {
        let op = e.operation_id.clone().or_else(|| e.summary.clone()).unwrap_or_default();
        let _ = writeln!(s, "| `{}` | `{}` | {} |", e.method, e.path, op);
    }
    let _ = writeln!(s);
}

fn section_services(s: &mut String, p: &Project) {
    if p.services.is_empty() {
        return;
    }
    let _ = writeln!(s, "## Servizi ({})\n", p.services.len());
    let _ = writeln!(s, "| Servizio | Immagine | Porte | Dipende da |");
    let _ = writeln!(s, "|---|---|---|---|");
    for sv in &p.services {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} |",
            sv.name,
            sv.image.clone().unwrap_or_else(|| "-".into()),
            join(&sv.ports),
            join(&sv.depends_on),
        );
    }
    let _ = writeln!(s);
}

fn section_components(s: &mut String, p: &Project) {
    if p.components.is_empty() {
        return;
    }
    let _ = writeln!(s, "## Componenti ({})\n", p.components.len());
    let _ = writeln!(s, "| Nome | Tipo | Linguaggio | Percorso |");
    let _ = writeln!(s, "|---|---|---|---|");
    for c in &p.components {
        let _ = writeln!(
            s,
            "| {} | {:?} | {:?} | `{}` |",
            c.name, c.kind, c.language, c.path
        );
    }
    let _ = writeln!(s);
}

fn section_database(s: &mut String, p: &Project) {
    if p.tables.is_empty() {
        return;
    }
    let _ = writeln!(s, "## Database ({} tabelle)\n", p.tables.len());
    for t in &p.tables {
        let title = match &t.schema {
            Some(sc) => format!("{sc}.{}", t.name),
            None => t.name.clone(),
        };
        let _ = writeln!(s, "### `{title}`\n");
        if !t.columns.is_empty() {
            let _ = writeln!(s, "| Colonna | Tipo | Null | PK |");
            let _ = writeln!(s, "|---|---|---|---|");
            for col in &t.columns {
                let _ = writeln!(
                    s,
                    "| {} | {} | {} | {} |",
                    col.name,
                    col.data_type,
                    if col.nullable { "si" } else { "no" },
                    if col.primary_key { "PK" } else { "" },
                );
            }
            let _ = writeln!(s);
        }
        for fk in &t.foreign_keys {
            let _ = writeln!(
                s,
                "- FK: `{}` -> `{}({})`",
                fk.column, fk.references_table, fk.references_column
            );
        }
        let _ = writeln!(s);
    }
}

fn section_dependencies(s: &mut String, p: &Project) {
    if p.dependencies.is_empty() {
        return;
    }
    let _ = writeln!(s, "## Dipendenze ({})\n", p.dependencies.len());
    let _ = writeln!(s, "| Pacchetto | Versione | Ecosistema |");
    let _ = writeln!(s, "|---|---|---|");
    for d in &p.dependencies {
        let _ = writeln!(
            s,
            "| {} | {} | {} |",
            d.name,
            d.version.clone().unwrap_or_else(|| "-".into()),
            d.ecosystem
        );
    }
    let _ = writeln!(s);
}

/// Unisce una lista in una cella di tabella ("-" se vuota).
fn join(items: &[String]) -> String {
    if items.is_empty() {
        "-".into()
    } else {
        items.join(", ")
    }
}
