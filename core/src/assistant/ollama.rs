//! Client HTTP per Ollama (LLM locale), per l'uso offline/air-gapped.
//!
//! Usa l'endpoint `POST /api/chat` con `stream: false`.

use crate::{Error, Result};
use serde_json::Value;
use std::time::Duration;

const DEFAULT_HOST: &str = "http://localhost:11434";

/// Invia una richiesta a Ollama e restituisce il testo della risposta.
pub fn complete(host: Option<&str>, model: &str, system: &str, user: &str) -> Result<String> {
    if model.trim().is_empty() {
        return Err(Error::Llm("modello Ollama non specificato".into()));
    }
    let host = host.unwrap_or(DEFAULT_HOST).trim_end_matches('/');
    let url = format!("{host}/api/chat");

    let body = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
    });

    // I modelli locali possono essere lenti: timeout generoso.
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::new_with_config(config);

    let mut resp = agent
        .post(&url)
        .header("content-type", "application/json")
        .send_json(&body)
        .map_err(|e| Error::Llm(format!("richiesta a Ollama fallita (è in esecuzione?): {e}")))?;

    let status = resp.status();
    let v: Value = resp
        .body_mut()
        .read_json()
        .map_err(|e| Error::Llm(format!("risposta di Ollama non valida: {e}")))?;

    if !status.is_success() {
        let msg = v.get("error").and_then(|m| m.as_str()).unwrap_or("errore sconosciuto");
        return Err(Error::Llm(format!("Ollama HTTP {}: {msg}", status.as_u16())));
    }

    let text = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string();

    if text.trim().is_empty() {
        return Err(Error::Llm("Ollama ha restituito una risposta vuota".into()));
    }
    Ok(text)
}
