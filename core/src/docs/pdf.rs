//! Export PDF (puro Rust, via [genpdf](https://github.com/kaj/genpdf-rs)).
//!
//! Il font è incorporato nel binario, così l'export funziona ovunque senza
//! dipendere dai font di sistema. Il layout è volutamente semplice: titoli,
//! paragrafi e righe di tabella rese leggibili — sufficiente per una doc tecnica.

use crate::model::Project;
use crate::{Error, Result};
use genpdf::{elements, style, Document, Element, SimplePageDecorator};

/// Font incorporato (DejaVu Sans) usato per tutte le varianti.
const FONT: &[u8] = include_bytes!("../../../assets/fonts/doc.ttf");

/// Genera i byte di un PDF a partire dalla documentazione del progetto.
pub fn render(p: &Project) -> Result<Vec<u8>> {
    let font = || {
        genpdf::fonts::FontData::new(FONT.to_vec(), None)
            .map_err(|e| Error::Export(format!("font PDF non valido: {e}")))
    };
    // genpdf richiede le quattro varianti: riusiamo lo stesso font.
    let family = genpdf::fonts::FontFamily {
        regular: font()?,
        bold: font()?,
        italic: font()?,
        bold_italic: font()?,
    };

    let mut doc = Document::new(family);
    doc.set_title(&p.name);
    let mut deco = SimplePageDecorator::new();
    deco.set_margins(15);
    doc.set_page_decorator(deco);

    let md = super::markdown::render(p);
    for raw in md.lines() {
        let line = raw.trim_end();
        if let Some(t) = line.strip_prefix("# ") {
            doc.push(heading(t, 20));
        } else if let Some(t) = line.strip_prefix("## ") {
            doc.push(elements::Break::new(1));
            doc.push(heading(t, 14));
        } else if let Some(t) = line.strip_prefix("### ") {
            doc.push(heading(t, 12));
        } else if line.is_empty() {
            doc.push(elements::Break::new(1));
        } else if line.starts_with('|') {
            // Riga di tabella: rendiamo le celle separate da spazi.
            let cells = line.trim_matches('|').replace('|', "   ");
            if !cells.trim().chars().all(|c| c == '-' || c == ' ') {
                doc.push(elements::Paragraph::new(cells));
            }
        } else {
            doc.push(elements::Paragraph::new(line.to_string()));
        }
    }

    let mut buf: Vec<u8> = Vec::new();
    doc.render(&mut buf)
        .map_err(|e| Error::Export(format!("rendering PDF fallito: {e}")))?;
    Ok(buf)
}

/// Un paragrafo-titolo in grassetto con la dimensione indicata.
fn heading(text: &str, size: u8) -> elements::StyledElement<elements::Paragraph> {
    elements::Paragraph::new(text.to_string())
        .styled(style::Style::new().bold().with_font_size(size))
}
