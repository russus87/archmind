//! Analisi Java: estrae package, classi/interfacce, metodi e grafo delle
//! chiamate via tree-sitter (vedi [`super::treesitter`]).

use super::treesitter::{self, Lang};
use crate::model::Project;
use std::path::PathBuf;

/// Estrae i componenti Java da tutti i file `.java`.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf]) {
    treesitter::collect(project, root, files, Lang::Java);
}
