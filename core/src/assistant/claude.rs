//! Client HTTP per l'API di Claude (Anthropic).
//!
//! Rust non ha un SDK ufficiale, quindi si usa HTTP diretto verso
//! `POST /v1/messages` con gli header `x-api-key` e `anthropic-version`.
//! Modello predefinito: `claude-opus-4-8`.

use crate::{Error, Result};
use serde_json::Value;
use std::time::Duration;

/// Modello predefinito se l'utente non ne specifica uno.
const DEFAULT_MODEL: &str = "claude-opus-4-8";
const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Invia una richiesta a Claude e restituisce il testo della risposta.
pub fn complete(api_key: &str, model: Option<&str>, system: &str, user: &str) -> Result<String> {
    if api_key.trim().is_empty() {
        return Err(Error::Llm("chiave API di Claude mancante".into()));
    }
    let model = model.unwrap_or(DEFAULT_MODEL);

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system,
        "messages": [{ "role": "user", "content": user }],
    });

    // Non trattiamo gli status HTTP come errori, cosi' possiamo leggere il
    // corpo della risposta d'errore e mostrare un messaggio utile.
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::new_with_config(config);

    let mut resp = agent
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .send_json(&body)
        .map_err(|e| Error::Llm(format!("richiesta a Claude fallita: {e}")))?;

    let status = resp.status();
    let v: Value = resp
        .body_mut()
        .read_json()
        .map_err(|e| Error::Llm(format!("risposta di Claude non valida: {e}")))?;

    if !status.is_success() {
        let msg = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("errore sconosciuto");
        return Err(Error::Llm(format!("Claude HTTP {}: {msg}", status.as_u16())));
    }

    // La risposta contiene un array `content`; uniamo tutti i blocchi di testo.
    let text = v
        .get("content")
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    if text.trim().is_empty() {
        return Err(Error::Llm("Claude ha restituito una risposta vuota".into()));
    }
    Ok(text)
}
