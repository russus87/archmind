//! Dipendenze dichiarate: NuGet (.csproj), Maven (pom.xml), npm (package.json).
//!
//! L'estrazione e' euristica (regex/serde) ed e' pensata per essere veloce e
//! senza falsi negativi grossolani; il parsing semantico completo e' in roadmap.

use super::{file_name_lc, rel};
use crate::model::{Dependency, Project};
use std::path::PathBuf;

/// Estrae le dipendenze da tutti i manifest riconosciuti.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf]) {
    for path in files {
        let name = file_name_lc(path);
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let where_ = rel(root, path);

        if name.ends_with(".csproj") {
            nuget(project, &text, &where_);
        } else if name == "pom.xml" {
            maven(project, &text, &where_);
        } else if name == "package.json" {
            npm(project, &text, &where_);
        }
    }
}

/// `<PackageReference Include="X" Version="Y" />`
fn nuget(project: &mut Project, text: &str, where_: &str) {
    let re = regex::Regex::new(
        r#"(?i)<PackageReference\s+Include="([^"]+)"(?:\s+Version="([^"]*)")?"#,
    )
    .unwrap();
    for c in re.captures_iter(text) {
        project.dependencies.push(Dependency {
            name: c[1].to_string(),
            version: c.get(2).map(|m| m.as_str().to_string()).filter(|v| !v.is_empty()),
            ecosystem: "NuGet".into(),
            declared_in: where_.to_string(),
        });
    }
}

/// `<dependency><groupId>g</groupId><artifactId>a</artifactId><version>v</version>`
fn maven(project: &mut Project, text: &str, where_: &str) {
    let re = regex::Regex::new(
        r#"(?is)<dependency>.*?<groupId>(.*?)</groupId>.*?<artifactId>(.*?)</artifactId>(?:.*?<version>(.*?)</version>)?.*?</dependency>"#,
    )
    .unwrap();
    for c in re.captures_iter(text) {
        project.dependencies.push(Dependency {
            name: format!("{}:{}", c[1].trim(), c[2].trim()),
            version: c.get(3).map(|m| m.as_str().trim().to_string()),
            ecosystem: "Maven".into(),
            declared_in: where_.to_string(),
        });
    }
}

/// `"dependencies": { "x": "^1.2.3" }` e `devDependencies`.
fn npm(project: &mut Project, text: &str, where_: &str) {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else {
        return;
    };
    for key in ["dependencies", "devDependencies"] {
        if let Some(map) = json.get(key).and_then(|v| v.as_object()) {
            for (name, ver) in map {
                project.dependencies.push(Dependency {
                    name: name.clone(),
                    version: ver.as_str().map(|s| s.to_string()),
                    ecosystem: "npm".into(),
                    declared_in: where_.to_string(),
                });
            }
        }
    }
}
