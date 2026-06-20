//! Analisi C#: estrae namespace, classi/interfacce, metodi e grafo delle
//! chiamate via tree-sitter (vedi [`super::treesitter`]).

use super::treesitter::{self, Lang};
use crate::model::Project;
use std::path::PathBuf;

/// Estrae i componenti C# da tutti i file `.cs`.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf]) {
    treesitter::collect(project, root, files, Lang::CSharp);
}
