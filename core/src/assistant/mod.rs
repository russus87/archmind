//! L'assistente conversazionale (RAG): "chat con il progetto".
//!
//! Flusso: recupera i passaggi piu' rilevanti dall'indice ([`crate::index`]),
//! costruisce un prompt con quei passaggi come contesto e cita i file, poi
//! chiede la risposta a un provider LLM ([`Provider`]). Il provider e'
//! sostituibile: Claude (cloud) oppure Ollama (locale, per ambienti offline).
//!
//! Local-first: con Ollama nessun dato esce dalla macchina; con Claude vengono
//! inviati solo gli snippet recuperati, non l'intero repository.

mod claude;
mod ollama;

use crate::index;
use crate::model::Project;
use crate::Result;
use serde::{Deserialize, Serialize};

/// Configurazione del provider LLM scelto dall'utente.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Provider {
    /// API di Claude (Anthropic). `api_key` obbligatoria; `model` opzionale.
    Claude {
        api_key: String,
        #[serde(default)]
        model: Option<String>,
    },
    /// Ollama locale. `host` opzionale (default http://localhost:11434).
    Ollama {
        #[serde(default)]
        host: Option<String>,
        model: String,
    },
}

/// Una citazione a una fonte usata nella risposta.
#[derive(Debug, Clone, Serialize)]
pub struct Citation {
    pub title: String,
    pub location: String,
    pub kind: String,
}

/// La risposta dell'assistente con le sue citazioni.
#[derive(Debug, Clone, Serialize)]
pub struct Answer {
    pub text: String,
    pub citations: Vec<Citation>,
}

/// Quanti passaggi recuperare per il contesto.
const TOP_K: usize = 6;

/// Risponde a una domanda sul progetto usando il provider indicato.
pub fn ask(project: &Project, question: &str, provider: &Provider) -> Result<Answer> {
    let passages = index::retrieve(project, question, TOP_K)?;

    // Contesto numerato: l'LLM puo' citare [1], [2]... e noi le risolviamo.
    let mut context = String::new();
    for (i, p) in passages.iter().enumerate() {
        context.push_str(&format!(
            "[{}] ({}) {}\n{}\n\n",
            i + 1,
            p.kind,
            p.location,
            p.snippet
        ));
    }
    if context.is_empty() {
        context.push_str("(nessun contesto rilevante trovato nell'indice)\n");
    }

    let system = "Sei l'assistente tecnico di ArchMind. Rispondi a domande sul \
        funzionamento di un progetto software usando ESCLUSIVAMENTE il contesto \
        fornito (estratti di codice e metadati). Cita le fonti con la loro \
        numerazione tra parentesi quadre, es. [1]. Se il contesto non basta, \
        dillo chiaramente. Rispondi in italiano, in modo conciso e tecnico.";

    let user = format!(
        "Contesto del progetto \"{}\":\n\n{}\n---\nDomanda: {}",
        project.name, context, question
    );

    let text = match provider {
        Provider::Claude { api_key, model } => claude::complete(api_key, model.as_deref(), system, &user)?,
        Provider::Ollama { host, model } => ollama::complete(host.as_deref(), model, system, &user)?,
    };

    let citations = passages
        .into_iter()
        .map(|p| Citation {
            title: p.title,
            location: p.location,
            kind: p.kind,
        })
        .collect();

    Ok(Answer { text, citations })
}
