//! Export HTML: converte la documentazione Markdown in una pagina HTML
//! autosufficiente e con uno stile leggibile.

use crate::model::Project;
use pulldown_cmark::{html, Options, Parser};

/// Foglio di stile minimale incorporato nella pagina.
const CSS: &str = r#"
:root { color-scheme: light dark; }
body { font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  max-width: 920px; margin: 40px auto; padding: 0 20px; line-height: 1.6; }
h1 { border-bottom: 2px solid #6d8cff; padding-bottom: .3em; }
h2 { color: #4a5680; margin-top: 2em; }
code { font-family: ui-monospace, monospace; background: #00000010; padding: 1px 4px; border-radius: 4px; }
table { border-collapse: collapse; width: 100%; margin: 1em 0; }
th, td { border: 1px solid #88888855; padding: 6px 10px; text-align: left; }
th { background: #6d8cff22; }
"#;

/// Produce una pagina HTML completa per il progetto.
pub fn render(p: &Project) -> String {
    let md = super::markdown::render(p);

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&md, opts);

    let mut body = String::new();
    html::push_html(&mut body, parser);

    format!(
        "<!DOCTYPE html>\n<html lang=\"it\">\n<head>\n<meta charset=\"UTF-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\
         <title>{title} — ArchMind</title>\n<style>{css}</style>\n</head>\n<body>\n{body}\n</body>\n</html>\n",
        title = escape(&p.name),
        css = CSS,
        body = body
    )
}

/// Escape minimale per il titolo nel tag <title>.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
