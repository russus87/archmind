//! CLI di ArchMind — analisi e documentazione headless per la CI.
//!
//! Comandi:
//!   analyze <path> [--out docs] [--diagrams] [--json]   genera la documentazione
//!   check   <path> [--out docs]                         gate CI: fallisce se la doc è in drift
//!   ask     <path> --question "..." [--ollama]          assistente RAG da terminale
//!
//! Riusa interamente `archmind-core`: stessa logica del desktop, senza Tauri.

use archmind_core::assistant::Provider;
use archmind_core::{diagrams, docs, project};
use clap::{Parser, Subcommand};
use std::error::Error;
use std::fs;
use std::process::exit;

/// File di documentazione generato (confrontato dal comando `check`).
const DOC_FILE: &str = "PROJECT.md";
const DIAGRAM_KINDS: &[&str] = &["dependency", "component", "er", "class", "sequence", "flow"];

#[derive(Parser)]
#[command(
    name = "archmind-cli",
    version,
    about = "Analisi automatica di progetti software e generazione documentazione"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Analizza un progetto e scrive la documentazione su disco.
    Analyze {
        /// Cartella radice del progetto.
        path: String,
        /// Cartella di output.
        #[arg(long, default_value = "docs")]
        out: String,
        /// Genera anche i diagrammi Mermaid in <out>/diagrams.
        #[arg(long)]
        diagrams: bool,
        /// Esporta anche il modello completo in <out>/project.json.
        #[arg(long)]
        json: bool,
    },
    /// Verifica che la documentazione committata sia aggiornata (gate CI).
    /// Esce con codice 1 se la doc rigenerata differisce da quella su disco.
    Check {
        /// Cartella radice del progetto.
        path: String,
        /// Cartella della documentazione da confrontare.
        #[arg(long, default_value = "docs")]
        out: String,
    },
    /// Esporta la documentazione in un formato specifico.
    Export {
        /// Cartella radice del progetto.
        path: String,
        /// Formato: md | html | wiki | pdf.
        #[arg(long, default_value = "html")]
        format: String,
        /// File di output.
        #[arg(long)]
        out: String,
    },
    /// Pone una domanda all'assistente RAG sul progetto.
    Ask {
        /// Cartella radice del progetto.
        path: String,
        /// Domanda da porre.
        #[arg(long)]
        question: String,
        /// Usa Ollama locale invece di Claude.
        #[arg(long)]
        ollama: bool,
        /// Modello (default: claude-opus-4-8, oppure llama3.1 con --ollama).
        #[arg(long)]
        model: Option<String>,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("errore: {e}");
        exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().cmd {
        Cmd::Analyze {
            path,
            out,
            diagrams,
            json,
        } => analyze(&path, &out, diagrams, json),
        Cmd::Check { path, out } => check(&path, &out),
        Cmd::Export { path, format, out } => export(&path, &format, &out),
        Cmd::Ask {
            path,
            question,
            ollama,
            model,
        } => ask(&path, &question, ollama, model),
    }
}

/// Genera la documentazione (e opzionalmente diagrammi/JSON).
fn analyze(path: &str, out: &str, diagrams: bool, json: bool) -> Result<(), Box<dyn Error>> {
    let project = project::analyze(path)?;
    fs::create_dir_all(out)?;

    let md = docs::markdown::render(&project);
    fs::write(format!("{out}/{DOC_FILE}"), &md)?;
    println!("scritto {out}/{DOC_FILE}");

    if diagrams {
        let dir = format!("{out}/diagrams");
        fs::create_dir_all(&dir)?;
        for kind in DIAGRAM_KINDS {
            let d = diagrams::mermaid::render(&project, kind)?;
            fs::write(format!("{dir}/{kind}.mmd"), d)?;
        }
        println!("scritti {} diagrammi in {dir}", DIAGRAM_KINDS.len());
    }

    if json {
        fs::write(
            format!("{out}/project.json"),
            serde_json::to_string_pretty(&project)?,
        )?;
        println!("scritto {out}/project.json");
    }
    Ok(())
}

/// Gate CI: rigenera la doc e la confronta con quella committata.
fn check(path: &str, out: &str) -> Result<(), Box<dyn Error>> {
    let project = project::analyze(path)?;
    let fresh = docs::markdown::render(&project);
    let committed = fs::read_to_string(format!("{out}/{DOC_FILE}")).unwrap_or_default();

    if fresh.trim() == committed.trim() {
        println!("✓ documentazione aggiornata ({out}/{DOC_FILE})");
        Ok(())
    } else {
        eprintln!(
            "✗ documentazione in drift: {out}/{DOC_FILE} non è aggiornata.\n  \
             Rigenerala con:  archmind-cli analyze {path} --out {out}"
        );
        exit(1);
    }
}

/// Esporta la documentazione nel formato richiesto.
fn export(path: &str, format: &str, out: &str) -> Result<(), Box<dyn Error>> {
    let project = project::analyze(path)?;
    match format {
        "md" => fs::write(out, docs::markdown::render(&project))?,
        "html" => fs::write(out, docs::html::render(&project))?,
        "wiki" => fs::write(out, docs::wiki::render(&project))?,
        "pdf" => fs::write(out, docs::pdf::render(&project)?)?,
        other => return Err(format!("formato non supportato: {other}").into()),
    }
    println!("scritto {out}");
    Ok(())
}

/// Assistente RAG da terminale.
fn ask(path: &str, question: &str, ollama: bool, model: Option<String>) -> Result<(), Box<dyn Error>> {
    let project = project::analyze(path)?;

    let provider = if ollama {
        Provider::Ollama {
            host: None,
            model: model.unwrap_or_else(|| "llama3.1".to_string()),
        }
    } else {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "imposta ANTHROPIC_API_KEY oppure usa --ollama")?;
        Provider::Claude { api_key, model }
    };

    let answer = archmind_core::assistant::ask(&project, question, &provider)?;
    println!("{}\n", answer.text);
    if !answer.citations.is_empty() {
        println!("Fonti:");
        for c in answer.citations {
            println!("  - [{}] {}", c.kind, if c.location.is_empty() { c.title } else { c.location });
        }
    }
    Ok(())
}
