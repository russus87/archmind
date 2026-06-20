//! Estrazione di simboli via [tree-sitter](https://tree-sitter.github.io).
//!
//! Sostituisce le euristiche regex con un parsing sintattico reale: tipi
//! (classi/interfacce), metodi e *chiamate* tra metodi. Da queste ultime si
//! ricostruisce un grafo delle chiamate intra-linguaggio (relazioni `Calls`),
//! base per l'analisi del flusso applicativo.
//!
//! Il walker e' generico: le poche differenze tra C# e Java sono catturate da
//! [`Lang`]. Un file che non parsa non blocca nulla (degrada in silenzio).

use crate::model::{Component, ComponentKind, Language, Project, Relation, RelationKind};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

/// Linguaggio supportato dal walker.
#[derive(Clone, Copy)]
pub enum Lang {
    CSharp,
    Java,
}

impl Lang {
    fn language(self) -> tree_sitter::Language {
        match self {
            Lang::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Lang::Java => tree_sitter_java::LANGUAGE.into(),
        }
    }

    fn model_language(self) -> Language {
        match self {
            Lang::CSharp => Language::CSharp,
            Lang::Java => Language::Java,
        }
    }

    fn file_ext(self) -> &'static str {
        match self {
            Lang::CSharp => "cs",
            Lang::Java => "java",
        }
    }

    /// Prefisso degli id dei componenti (deve restare stabile nel tempo).
    fn id_prefix(self) -> &'static str {
        match self {
            Lang::CSharp => "cs",
            Lang::Java => "java",
        }
    }

    /// Nodo del contenitore "namespace/package".
    fn container_kind(self) -> &'static [&'static str] {
        match self {
            Lang::CSharp => &["namespace_declaration", "file_scoped_namespace_declaration"],
            Lang::Java => &["package_declaration"],
        }
    }

    /// Nodo che rappresenta una chiamata a metodo.
    fn invocation_kind(self) -> &'static str {
        match self {
            Lang::CSharp => "invocation_expression",
            Lang::Java => "method_invocation",
        }
    }
}

/// Un tipo estratto da un file, con i suoi metodi e i nomi dei metodi chiamati.
struct ExtractedType {
    name: String,
    kind: ComponentKind,
    path: String,
    methods: Vec<String>,
    /// Nomi (semplici) dei metodi invocati dentro questo tipo.
    callees: BTreeSet<String>,
}

/// Punto di ingresso: estrae componenti e relazioni `Calls` per un linguaggio.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf], lang: Lang) {
    let mut parser = Parser::new();
    if parser.set_language(&lang.language()).is_err() {
        return; // grammatica non caricabile: si rinuncia, senza errori
    }

    let mut types: Vec<ExtractedType> = Vec::new();

    for path in files {
        if super::ext(path) != lang.file_ext() {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        let Some(tree) = parser.parse(&src, None) else {
            continue;
        };
        let where_ = super::rel(root, path);

        // Eventuale namespace/package del file -> componente contenitore.
        if let Some(container) = find_container(tree.root_node(), &src, lang) {
            let (id, kind) = match lang {
                Lang::CSharp => (format!("ns:{container}"), ComponentKind::Namespace),
                Lang::Java => (format!("pkg:{container}"), ComponentKind::Package),
            };
            push_component(project, Component {
                id,
                name: container,
                kind,
                language: lang.model_language(),
                path: where_.clone(),
                members: vec![],
            });
        }

        let mut walker = TypeWalker {
            src: &src,
            lang,
            path: &where_,
            out: &mut types,
        };
        walker.walk(tree.root_node(), None);
    }

    // Indice metodo -> tipo proprietario (per risolvere le chiamate).
    let mut owner: HashMap<&str, &str> = HashMap::new();
    for t in &types {
        for m in &t.methods {
            owner.entry(m.as_str()).or_insert(t.name.as_str());
        }
    }

    // Relazioni Calls: tipo -> tipo proprietario del metodo chiamato.
    let mut rels: BTreeSet<(String, String)> = BTreeSet::new();
    for t in &types {
        for callee in &t.callees {
            if let Some(&owning) = owner.get(callee.as_str()) {
                if owning != t.name {
                    rels.insert((t.name.clone(), owning.to_string()));
                }
            }
        }
    }

    // Pubblica i componenti (deduplicati per id).
    for t in &types {
        push_component(project, Component {
            id: format!("{}:{}", lang.id_prefix(), t.name),
            name: t.name.clone(),
            kind: t.kind,
            language: lang.model_language(),
            path: t.path.clone(),
            members: t.methods.clone(),
        });
    }
    for (from, to) in rels {
        project.relations.push(Relation {
            from: format!("{}:{from}", lang.id_prefix()),
            to: format!("{}:{to}", lang.id_prefix()),
            kind: RelationKind::Calls,
        });
    }
}

/// Cerca il primo namespace/package dichiarato e ne restituisce il nome.
fn find_container(node: Node, src: &str, lang: Lang) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if lang.container_kind().contains(&child.kind()) {
            if let Some(name) = child.child_by_field_name("name") {
                return Some(text(name, src).to_string());
            }
        }
        // I namespace possono annidare dichiarazioni: scendi di un livello.
        if let Some(found) = find_container(child, src, lang) {
            return Some(found);
        }
    }
    None
}

/// Visita ricorsiva che raccoglie tipi, metodi e chiamate.
struct TypeWalker<'a> {
    src: &'a str,
    lang: Lang,
    path: &'a str,
    out: &'a mut Vec<ExtractedType>,
}

impl<'a> TypeWalker<'a> {
    /// `current` e' l'indice del tipo che racchiude il nodo, se esiste.
    fn walk(&mut self, node: Node, current: Option<usize>) {
        let kind = node.kind();

        // Dichiarazione di tipo: crea un nuovo ExtractedType e diventa il
        // contenitore corrente per i figli.
        if let Some(ck) = type_kind(kind) {
            let name = node
                .child_by_field_name("name")
                .map(|n| text(n, self.src).to_string());
            if let Some(name) = name {
                self.out.push(ExtractedType {
                    name,
                    kind: ck,
                    path: self.path.to_string(),
                    methods: vec![],
                    callees: BTreeSet::new(),
                });
                let idx = self.out.len() - 1;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.walk(child, Some(idx));
                }
                return;
            }
        }

        // Metodo: registra il nome nel tipo corrente.
        if kind == "method_declaration" {
            if let (Some(cur), Some(n)) = (current, node.child_by_field_name("name")) {
                self.out[cur].methods.push(text(n, self.src).to_string());
            }
        }

        // Chiamata a metodo: registra il nome semplice del callee.
        if kind == self.lang.invocation_kind() {
            if let (Some(cur), Some(callee)) = (current, self.callee_name(node)) {
                self.out[cur].callees.insert(callee);
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk(child, current);
        }
    }

    /// Estrae il nome semplice del metodo chiamato.
    fn callee_name(&self, node: Node) -> Option<String> {
        match self.lang {
            // Java: il nodo ha direttamente il field "name".
            Lang::Java => node
                .child_by_field_name("name")
                .map(|n| text(n, self.src).to_string()),
            // C#: "function" puo' essere un identifier o un member_access
            // (a.B(...)): in tal caso prendiamo il field "name".
            Lang::CSharp => {
                let f = node.child_by_field_name("function")?;
                if f.kind() == "member_access_expression" {
                    f.child_by_field_name("name")
                        .map(|n| text(n, self.src).to_string())
                } else {
                    Some(text(f, self.src).to_string())
                }
            }
        }
    }
}

/// Mappa il tipo di nodo tree-sitter sulla nostra granularita'.
fn type_kind(node_kind: &str) -> Option<ComponentKind> {
    match node_kind {
        "class_declaration" | "record_declaration" | "struct_declaration" => {
            Some(ComponentKind::Class)
        }
        "interface_declaration" => Some(ComponentKind::Interface),
        _ => None,
    }
}

/// Testo sorgente coperto da un nodo.
fn text<'a>(node: Node, src: &'a str) -> &'a str {
    &src[node.byte_range()]
}

/// Inserisce un componente solo se l'id non e' gia' presente.
fn push_component(project: &mut Project, comp: Component) {
    if !project.components.iter().any(|c| c.id == comp.id) {
        project.components.push(comp);
    }
}
