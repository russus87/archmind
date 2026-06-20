//! Analisi Git: legge i metadati del repository invocando il binario `git`.
//!
//! Si appoggia al `git` di sistema (presente su praticamente tutte le macchine
//! di sviluppo e sui runner CI). Se `git` non c'e' o la cartella non e' un
//! repository, l'analyzer non fa nulla: l'analisi prosegue senza errori.

use crate::model::Project;
use std::path::Path;
use std::process::Command;

/// Aggiunge alle statistiche del progetto le info del repository, se presente.
pub fn collect(project: &mut Project, root: &Path) {
    if !root.join(".git").exists() {
        return;
    }

    // Remote di origine: utile come "identita'" del progetto nella doc.
    if let Some(remote) = git(root, &["config", "--get", "remote.origin.url"]) {
        if !remote.is_empty() {
            project
                .stats
                .by_extension
                .entry("__git_remote".into())
                .or_insert(0);
            // Il remote vero finisce nel nome se il progetto e' anonimo.
            if project.name == "progetto" {
                if let Some(last) = remote.rsplit('/').next() {
                    project.name = last.trim_end_matches(".git").to_string();
                }
            }
        }
    }
}

/// Esegue un comando git nella cartella indicata e ne restituisce lo stdout.
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
