//! Indice full-text [tantivy](https://github.com/quickwit-oss/tantivy) per il
//! retrieval del RAG.
//!
//! Costruisce un indice in memoria a partire dal modello di progetto e dal
//! contenuto dei file sorgente, poi restituisce i passaggi piu' rilevanti per
//! una domanda (ranking BM25). E' la base di recupero dell'assistente
//! ([`crate::assistant`]); la persistenza su disco (MmapDirectory) e l'aggiunta
//! di un indice vettoriale denso sono previste come evoluzione.

use crate::model::Project;
use crate::{Error, Result};
use std::collections::HashSet;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, Value, STORED, STRING, TEXT};
use tantivy::{doc, Index, TantivyDocument};

/// Un passaggio recuperato dall'indice, pronto per il contesto del prompt.
#[derive(Debug, Clone)]
pub struct Passage {
    /// Categoria: "code", "endpoint", "table", "service", "dependency".
    pub kind: String,
    /// Titolo leggibile (nome file o entita').
    pub title: String,
    /// Posizione (percorso file o sorgente).
    pub location: String,
    /// Testo del passaggio (gia' troncato a una lunghezza ragionevole).
    pub snippet: String,
}

/// Numero massimo di caratteri indicizzati per file (evita prompt giganti).
const MAX_CHARS: usize = 6000;

/// Costruisce l'indice e recupera i `k` passaggi piu' rilevanti per `query`.
pub fn retrieve(project: &Project, query: &str, k: usize) -> Result<Vec<Passage>> {
    let mut sb = Schema::builder();
    let f_kind = sb.add_text_field("kind", STRING | STORED);
    let f_title = sb.add_text_field("title", TEXT | STORED);
    let f_body = sb.add_text_field("body", TEXT | STORED);
    let f_loc = sb.add_text_field("location", STRING | STORED);
    let schema = sb.build();

    let index = Index::create_in_ram(schema);
    let mut writer = index
        .writer(15_000_000)
        .map_err(|e| Error::Index(e.to_string()))?;

    // 1) Contenuto dei file sorgente referenziati dai componenti (RAG sul codice).
    let mut seen: HashSet<&str> = HashSet::new();
    for c in &project.components {
        if c.path.is_empty() || !seen.insert(c.path.as_str()) {
            continue;
        }
        let abs = Path::new(&project.root).join(&c.path);
        if let Ok(text) = std::fs::read_to_string(&abs) {
            let body: String = text.chars().take(MAX_CHARS).collect();
            writer
                .add_document(doc!(f_kind=>"code", f_title=>c.path.clone(), f_body=>body, f_loc=>c.path.clone()))
                .map_err(|e| Error::Index(e.to_string()))?;
        }
    }

    // 2) Documenti sintetici per le entita' (recuperabili anche senza file).
    for e in &project.endpoints {
        let body = format!(
            "Endpoint {} {} {} {}",
            e.method,
            e.path,
            e.operation_id.clone().unwrap_or_default(),
            e.summary.clone().unwrap_or_default()
        );
        let title = format!("{} {}", e.method, e.path);
        writer
            .add_document(doc!(f_kind=>"endpoint", f_title=>title, f_body=>body, f_loc=>e.source.clone()))
            .map_err(|e| Error::Index(e.to_string()))?;
    }
    for t in &project.tables {
        let cols: Vec<String> = t.columns.iter().map(|c| c.name.clone()).collect();
        let body = format!("Tabella {} colonne: {}", t.name, cols.join(", "));
        writer
            .add_document(doc!(f_kind=>"table", f_title=>t.name.clone(), f_body=>body, f_loc=>t.schema.clone().unwrap_or_default()))
            .map_err(|e| Error::Index(e.to_string()))?;
    }
    for s in &project.services {
        let body = format!(
            "Servizio {} immagine {} dipende da {}",
            s.name,
            s.image.clone().unwrap_or_default(),
            s.depends_on.join(", ")
        );
        writer
            .add_document(doc!(f_kind=>"service", f_title=>s.name.clone(), f_body=>body, f_loc=>s.source.clone()))
            .map_err(|e| Error::Index(e.to_string()))?;
    }

    writer.commit().map_err(|e| Error::Index(e.to_string()))?;

    let reader = index.reader().map_err(|e| Error::Index(e.to_string()))?;
    let searcher = reader.searcher();
    let parser = QueryParser::for_index(&index, vec![f_title, f_body]);

    // Sanifica la query: solo parole, in OR. Evita errori di sintassi del parser
    // (es. caratteri ":", "(" digitati dall'utente).
    let cleaned: String = query
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    if cleaned.split_whitespace().next().is_none() {
        return Ok(vec![]);
    }
    let q = parser
        .parse_query(cleaned.trim())
        .map_err(|e| Error::Index(e.to_string()))?;

    let top = searcher
        .search(&q, &TopDocs::with_limit(k).order_by_score())
        .map_err(|e| Error::Index(e.to_string()))?;

    let get = |doc: &TantivyDocument, field| {
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    let mut out = Vec::new();
    for (_score, addr) in top {
        let doc: TantivyDocument = searcher
            .doc(addr)
            .map_err(|e| Error::Index(e.to_string()))?;
        let body = get(&doc, f_body);
        out.push(Passage {
            kind: get(&doc, f_kind),
            title: get(&doc, f_title),
            location: get(&doc, f_loc),
            snippet: body.chars().take(1200).collect(),
        });
    }
    Ok(out)
}
