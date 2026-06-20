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
