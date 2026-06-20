//! Test di fumo end-to-end: costruisce un finto progetto su disco, lo analizza
//! e verifica che ogni analyzer popoli il modello e che doc/diagrammi escano.

use std::fs;

/// Crea un mini progetto con un po' di tutto e ne controlla l'analisi.
#[test]
fn analizza_un_progetto_minimo() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // C#
    fs::write(
        root.join("Order.cs"),
        "namespace Shop.Domain;\npublic class Order { public void Pay() {} }",
    )
    .unwrap();
    // Java
    fs::write(
        root.join("App.java"),
        "package com.shop;\npublic class App { public String run() { return \"\"; } }",
    )
    .unwrap();
    // OpenAPI
    fs::write(
        root.join("api.yaml"),
        "openapi: 3.0.0\npaths:\n  /orders:\n    get:\n      operationId: listOrders\n",
    )
    .unwrap();
    // Docker Compose
    fs::write(
        root.join("docker-compose.yml"),
        "services:\n  web:\n    image: nginx\n    ports: [\"80:80\"]\n    depends_on: [db]\n  db:\n    image: postgres\n",
    )
    .unwrap();
    // DDL SQL con foreign key
    fs::write(
        root.join("schema.sql"),
        "CREATE TABLE customers (id NUMBER PRIMARY KEY, name VARCHAR2(100));\n\
         CREATE TABLE orders (id NUMBER PRIMARY KEY, customer_id NUMBER REFERENCES customers(id));",
    )
    .unwrap();
    // Dipendenze npm
    fs::write(
        root.join("package.json"),
        "{\"dependencies\": {\"svelte\": \"^5\"}}",
    )
    .unwrap();

    let p = archmind_core::project::analyze(root.to_str().unwrap()).unwrap();

    assert!(p.components.iter().any(|c| c.name == "Order"), "manca classe C#");
    assert!(p.components.iter().any(|c| c.name == "App"), "manca classe Java");
    assert!(p.endpoints.iter().any(|e| e.path == "/orders"), "manca endpoint");
    assert!(p.services.iter().any(|s| s.name == "web"), "manca servizio compose");
    assert!(p.tables.iter().any(|t| t.name == "orders"), "manca tabella");
    assert!(
        p.tables.iter().any(|t| !t.foreign_keys.is_empty()),
        "manca foreign key"
    );
    assert!(p.dependencies.iter().any(|d| d.name == "svelte"), "manca dipendenza");

    // La documentazione e i diagrammi non devono andare in errore.
    let md = archmind_core::docs::markdown::render(&p);
    assert!(md.contains("# "), "markdown vuoto");
    for kind in ["dependency", "component", "er", "class", "sequence"] {
        let d = archmind_core::diagrams::mermaid::render(&p, kind).unwrap();
        assert!(!d.is_empty(), "diagramma {kind} vuoto");
    }

    // La ricerca trova la tabella orders.
    let hits = archmind_core::search::search(&p, "orders");
    assert!(!hits.is_empty(), "ricerca senza risultati");
}

/// Verifica che tree-sitter estragga metodi e ricostruisca le chiamate
/// tra tipi (relazione `Calls`).
#[test]
fn estrae_grafo_chiamate_csharp() {
    use archmind_core::model::RelationKind;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    fs::write(
        root.join("Repo.cs"),
        "namespace Shop;\npublic class Repository { public void Save() {} }",
    )
    .unwrap();
    fs::write(
        root.join("Service.cs"),
        "namespace Shop;\npublic class Service {\n  private Repository repo;\n  public void Handle() { repo.Save(); }\n}",
    )
    .unwrap();

    let p = archmind_core::project::analyze(root.to_str().unwrap()).unwrap();

    let service = p.components.iter().find(|c| c.name == "Service").expect("manca Service");
    assert!(service.members.contains(&"Handle".to_string()), "metodo non estratto");

    // Service.Handle() chiama Repository.Save() -> arco Calls.
    assert!(
        p.relations
            .iter()
            .any(|r| r.kind == RelationKind::Calls && r.from == "cs:Service" && r.to == "cs:Repository"),
        "manca l'arco Calls Service -> Repository"
    );
}

/// Verifica il linking cross-layer: endpoint → componente → tabella.
#[test]
fn collega_i_livelli_applicativi() {
    use archmind_core::model::RelationKind;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Controller con metodo listOrders che usa la tabella "orders".
    fs::write(
        root.join("OrdersController.cs"),
        "namespace Shop.Api;\npublic class OrdersController {\n  public void ListOrders() { var q = \"select * from orders\"; }\n}",
    )
    .unwrap();
    // OpenAPI con operationId che combacia col metodo.
    fs::write(
        root.join("api.yaml"),
        "openapi: 3.0.0\npaths:\n  /orders:\n    get:\n      operationId: ListOrders\n",
    )
    .unwrap();
    // DDL con la tabella orders.
    fs::write(
        root.join("schema.sql"),
        "CREATE TABLE orders (id NUMBER PRIMARY KEY);",
    )
    .unwrap();

    let p = archmind_core::project::analyze(root.to_str().unwrap()).unwrap();

    // endpoint esposto dal controller (Exposes: componente -> endpoint)
    assert!(
        p.relations.iter().any(|r| r.kind == RelationKind::Exposes
            && r.from == "cs:OrdersController"
            && r.to.contains("/orders")),
        "manca il link endpoint <- controller"
    );
    // controller che referenzia la tabella orders (References: componente -> tabella)
    assert!(
        p.relations.iter().any(|r| r.kind == RelationKind::References
            && r.from == "cs:OrdersController"
            && r.to == "table:orders"),
        "manca il link controller -> tabella"
    );

    // Il diagramma di flusso deve contenere entrambi i salti.
    let flow = archmind_core::diagrams::mermaid::render(&p, "flow").unwrap();
    assert!(flow.contains("OrdersController"), "flow senza controller");
}

/// Persistenza: salva, elenca e ricarica uno snapshot.
#[test]
fn salva_e_ricarica_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_str().unwrap();
    fs::write(dir.path().join("A.cs"), "namespace N;\npublic class A {}").unwrap();

    let p = archmind_core::project::analyze(root).unwrap();
    let id = archmind_core::store::save_snapshot(root, &p, "v1").unwrap();

    let metas = archmind_core::store::list_snapshots(root).unwrap();
    assert_eq!(metas.len(), 1);
    assert_eq!(metas[0].label, "v1");

    let loaded = archmind_core::store::load_snapshot(root, id).unwrap();
    assert_eq!(loaded.components.len(), p.components.len());
}

/// Confronto versioni: rileva un componente aggiunto e calcola l'impatto.
#[test]
fn confronto_versioni() {
    let d1 = tempfile::tempdir().unwrap();
    fs::write(
        d1.path().join("A.cs"),
        "namespace N;\npublic class A { public void X() {} }",
    )
    .unwrap();
    let v1 = archmind_core::project::analyze(d1.path().to_str().unwrap()).unwrap();

    let d2 = tempfile::tempdir().unwrap();
    fs::write(
        d2.path().join("A.cs"),
        "namespace N;\npublic class A { public void X() {} }",
    )
    .unwrap();
    fs::write(d2.path().join("B.cs"), "namespace N;\npublic class B {}").unwrap();
    let v2 = archmind_core::project::analyze(d2.path().to_str().unwrap()).unwrap();

    let cs = archmind_core::evolution::diff(&v1, &v2);
    assert!(cs.added.iter().any(|c| c.label == "B"), "B non risulta aggiunto");
    assert!(cs.removed.is_empty(), "non dovrebbero esserci rimozioni");
}

/// Export: HTML, Wiki (con mermaid) e PDF si generano correttamente.
#[test]
fn esporta_html_wiki_pdf() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("A.cs"), "namespace N;\npublic class A {}").unwrap();
    let p = archmind_core::project::analyze(dir.path().to_str().unwrap()).unwrap();

    let html = archmind_core::docs::html::render(&p);
    assert!(html.starts_with("<!DOCTYPE html>"), "HTML non valido");

    let wiki = archmind_core::docs::wiki::render(&p);
    assert!(wiki.contains("```mermaid"), "Wiki senza diagrammi mermaid");

    let pdf = archmind_core::docs::pdf::render(&p).unwrap();
    assert!(pdf.starts_with(b"%PDF"), "byte PDF non validi");
}

/// Verifica che l'indice tantivy recuperi passaggi rilevanti (base del RAG).
#[test]
fn indice_recupera_passaggi() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join("Payment.cs"),
        "namespace Billing;\npublic class PaymentProcessor { public void Charge() {} }",
    )
    .unwrap();

    let p = archmind_core::project::analyze(root.to_str().unwrap()).unwrap();
    let hits = archmind_core::index::retrieve(&p, "payment processor charge", 5).unwrap();
    assert!(!hits.is_empty(), "il retrieval non ha trovato nulla");
    assert!(
        hits.iter().any(|h| h.location.contains("Payment.cs")),
        "il file Payment.cs non è tra i risultati"
    );
}
