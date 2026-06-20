//! Estrazione di simboli via [tree-sitter](https://tree-sitter.github.io).
//!
//! Parsing sintattico reale (non regex) per più linguaggi: tipi, metodi e
//! chiamate. Da queste si ricostruisce un grafo delle chiamate intra-linguaggio
//! (relazione `Calls`), base dell'analisi del flusso applicativo.
//!
//! Copertura: C#/Java/TypeScript/Python = tipi + metodi + call graph; Go = tipi
//! (i metodi Go hanno un receiver esterno al tipo, estrazione best-effort futura).
//! Un file che non parsa non blocca nulla.

use crate::model::{Component, ComponentKind, Language, Project, Relation, RelationKind};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use tree_sitter::{Node, Parser};

/// Linguaggio supportato dal walker.
#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    CSharp,
    Java,
    TypeScript,
    Python,
    Go,
}

impl Lang {
    fn language(self) -> tree_sitter::Language {
        match self {
            Lang::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Lang::Java => tree_sitter_java::LANGUAGE.into(),
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::Go => tree_sitter_go::LANGUAGE.into(),
        }
    }

    fn model_language(self) -> Language {
        match self {
            Lang::CSharp => Language::CSharp,
            Lang::Java => Language::Java,
            Lang::TypeScript => Language::TypeScript,
            Lang::Python => Language::Python,
            Lang::Go => Language::Go,
        }
    }

    fn file_ext(self) -> &'static str {
        match self {
            Lang::CSharp => "cs",
            Lang::Java => "java",
            Lang::TypeScript => "ts",
            Lang::Python => "py",
            Lang::Go => "go",
        }
    }

    fn id_prefix(self) -> &'static str {
        match self {
            Lang::CSharp => "cs",
            Lang::Java => "java",
            Lang::TypeScript => "ts",
            Lang::Python => "py",
            Lang::Go => "go",
        }
    }

    /// Nodo del contenitore "namespace/package" (vuoto se non applicabile).
    fn container_kinds(self) -> &'static [&'static str] {
        match self {
            Lang::CSharp => &["namespace_declaration", "file_scoped_namespace_declaration"],
            Lang::Java => &["package_declaration"],
            Lang::Go => &["package_clause"],
            _ => &[],
        }
    }

    /// Nodo che rappresenta una chiamata a metodo.
    fn invocation_kind(self) -> &'static str {
        match self {
            Lang::CSharp => "invocation_expression",
            Lang::Java | Lang::Go => "method_invocation",
            Lang::TypeScript => "call_expression",
            Lang::Python => "call",
        }
    }

    /// Granularità del componente per un tipo di nodo dichiarazione.
    fn type_kind(self, k: &str) -> Option<ComponentKind> {
        match self {
            Lang::CSharp => match k {
                "class_declaration" | "record_declaration" | "struct_declaration" => Some(ComponentKind::Class),
                "interface_declaration" => Some(ComponentKind::Interface),
                _ => None,
            },
            Lang::Java => match k {
                "class_declaration" | "enum_declaration" => Some(ComponentKind::Class),
                "interface_declaration" => Some(ComponentKind::Interface),
                _ => None,
            },
            Lang::TypeScript => match k {
                "class_declaration" => Some(ComponentKind::Class),
                "interface_declaration" => Some(ComponentKind::Interface),
                _ => None,
            },
            Lang::Python => match k {
                "class_definition" => Some(ComponentKind::Class),
                _ => None,
            },
            // Go: il nome sta su type_spec; la specie dipende dal tipo sottostante.
            Lang::Go => None,
        }
    }

    /// Il nodo è una dichiarazione di metodo?
    fn is_method(self, k: &str) -> bool {
        match self {
            Lang::CSharp | Lang::Java => k == "method_declaration",
            Lang::TypeScript => k == "method_definition",
            Lang::Python => k == "function_definition",
            Lang::Go => false,
        }
    }
}

/// Un tipo estratto, con metodi e nomi dei metodi chiamati.
struct ExtractedType {
    name: String,
    kind: ComponentKind,
    path: String,
    methods: Vec<String>,
    callees: BTreeSet<String>,
}

/// Estrae componenti e relazioni `Calls` per un linguaggio.
pub fn collect(project: &mut Project, root: &str, files: &[PathBuf], lang: Lang) {
    let mut parser = Parser::new();
    if parser.set_language(&lang.language()).is_err() {
        return;
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

        if let Some(container) = find_container(tree.root_node(), &src, lang) {
            let (id, kind) = match lang {
                Lang::Java | Lang::Go => (format!("pkg:{container}"), ComponentKind::Package),
                _ => (format!("ns:{container}"), ComponentKind::Namespace),
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

/// Cerca il primo namespace/package e ne restituisce il nome.
fn find_container(node: Node, src: &str, lang: Lang) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if lang.container_kinds().contains(&child.kind()) {
            // Go: package_clause -> figlio package_identifier.
            if lang == Lang::Go {
                let mut c2 = child.walk();
                for gc in child.children(&mut c2) {
                    if gc.kind() == "package_identifier" {
                        return Some(text(gc, src).to_string());
                    }
                }
            } else if let Some(name) = child.child_by_field_name("name") {
                return Some(text(name, src).to_string());
            }
        }
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
    fn walk(&mut self, node: Node, current: Option<usize>) {
        let kind = node.kind();

        // Go: il tipo è su type_spec (name + type sottostante).
        if self.lang == Lang::Go && kind == "type_spec" {
            if let Some(name) = node.child_by_field_name("name") {
                let ck = node
                    .child_by_field_name("type")
                    .map(|t| match t.kind() {
                        "interface_type" => ComponentKind::Interface,
                        _ => ComponentKind::Class,
                    })
                    .unwrap_or(ComponentKind::Class);
                self.push_type(text(name, self.src).to_string(), ck);
            }
        } else if let Some(ck) = self.lang.type_kind(kind) {
            if let Some(name) = node.child_by_field_name("name") {
                let idx = self.push_type(text(name, self.src).to_string(), ck);
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.walk(child, Some(idx));
                }
                return;
            }
        }

        if self.lang.is_method(kind) {
            if let (Some(cur), Some(n)) = (current, node.child_by_field_name("name")) {
                self.out[cur].methods.push(text(n, self.src).to_string());
            }
        }

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

    /// Aggiunge un tipo e restituisce il suo indice.
    fn push_type(&mut self, name: String, kind: ComponentKind) -> usize {
        self.out.push(ExtractedType {
            name,
            kind,
            path: self.path.to_string(),
            methods: vec![],
            callees: BTreeSet::new(),
        });
        self.out.len() - 1
    }

    /// Nome semplice del metodo chiamato (per il call graph).
    fn callee_name(&self, node: Node) -> Option<String> {
        let f = node.child_by_field_name("function");
        match self.lang {
            Lang::Java => node.child_by_field_name("name").map(|n| text(n, self.src).to_string()),
            Lang::CSharp => {
                let f = f?;
                if f.kind() == "member_access_expression" {
                    f.child_by_field_name("name").map(|n| text(n, self.src).to_string())
                } else {
                    Some(text(f, self.src).to_string())
                }
            }
            Lang::TypeScript => {
                let f = f?;
                if f.kind() == "member_expression" {
                    f.child_by_field_name("property").map(|n| text(n, self.src).to_string())
                } else {
                    Some(text(f, self.src).to_string())
                }
            }
            Lang::Python => {
                let f = f?;
                if f.kind() == "attribute" {
                    f.child_by_field_name("attribute").map(|n| text(n, self.src).to_string())
                } else {
                    Some(text(f, self.src).to_string())
                }
            }
            Lang::Go => None,
        }
    }
}

/// Testo sorgente coperto da un nodo.
fn text<'a>(node: Node, src: &'a str) -> &'a str {
    &src[node.byte_range()]
}

/// Inserisce un componente solo se l'id non è già presente.
fn push_component(project: &mut Project, comp: Component) {
    if !project.components.iter().any(|c| c.id == comp.id) {
        project.components.push(comp);
    }
}
