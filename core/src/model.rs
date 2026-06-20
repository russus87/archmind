//! Il modello dati condiviso: il "grafo di conoscenza" di un progetto.
//!
//! Tutti gli analyzer scrivono qui. Documentazione, diagrammi e ricerca leggono
//! solo da qui. Le entita' tipizzate (componenti, endpoint, servizi, tabelle...)
//! convivono con un insieme di [`Relation`] generiche, cosi' il grafo resta
//! estendibile senza cambiare le strutture esistenti.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Il risultato completo dell'analisi di un progetto.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Cartella radice analizzata.
    pub root: String,
    /// Nome del progetto (di solito il nome della cartella).
    pub name: String,
    /// Componenti del codice: namespace C#, package Java, moduli, classi.
    pub components: Vec<Component>,
    /// Endpoint HTTP scoperti (OpenAPI/Swagger).
    pub endpoints: Vec<Endpoint>,
    /// Unita' eseguibili/servizi (Docker Compose, manifest Kubernetes).
    pub services: Vec<ServiceUnit>,
    /// Tabelle del database (da DDL SQL).
    pub tables: Vec<Table>,
    /// Dipendenze dichiarate (NuGet, Maven, Gradle, npm...).
    pub dependencies: Vec<Dependency>,
    /// Archi generici del grafo (chiamate, lettura/scrittura, depends-on...).
    pub relations: Vec<Relation>,
    /// Statistiche di sintesi.
    pub stats: Stats,
}

/// Linguaggio rilevato per un componente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    CSharp,
    Java,
    Sql,
    Yaml,
    Json,
    Other,
}

/// Granularita' di un componente del codice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    Namespace,
    Package,
    Class,
    Interface,
    Module,
}

/// Un componente del codice (classe, namespace, package...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub kind: ComponentKind,
    pub language: Language,
    /// Percorso file relativo alla radice del progetto.
    pub path: String,
    /// Membri rilevanti (metodi, campi) per i Class Diagram.
    #[serde(default)]
    pub members: Vec<String>,
}

/// Un endpoint HTTP (da OpenAPI/Swagger).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub id: String,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    /// File da cui proviene la definizione.
    pub source: String,
}

/// Un'unita' eseguibile/servizio (Compose o Kubernetes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUnit {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Origine: "docker-compose" oppure "kubernetes".
    pub source: String,
}

/// Una colonna di tabella.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
}

/// Una chiave esterna verso un'altra tabella.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub column: String,
    pub references_table: String,
    pub references_column: String,
}

/// Una tabella di database (da DDL SQL).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub columns: Vec<Column>,
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKey>,
}

/// Una dipendenza dichiarata in un manifest del progetto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    /// Ecosistema: "NuGet", "Maven", "Gradle", "npm"...
    pub ecosystem: String,
    /// File manifest in cui e' dichiarata.
    pub declared_in: String,
}

/// Tipo di relazione nel grafo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    DependsOn,
    Exposes,
    References,
    Contains,
    /// Un tipo chiama un metodo che appartiene a un altro tipo (call graph).
    Calls,
}

/// Un arco generico del grafo di conoscenza.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
}

/// Statistiche di sintesi del progetto.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub files: usize,
    pub lines_of_code: usize,
    /// Numero di file per estensione (es. "cs" -> 120).
    pub by_extension: BTreeMap<String, usize>,
}

impl Project {
    /// Crea un progetto vuoto agganciato a una cartella radice.
    pub fn new(root: impl Into<String>, name: impl Into<String>) -> Self {
        Project {
            root: root.into(),
            name: name.into(),
            ..Default::default()
        }
    }
}
