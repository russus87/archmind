//! Server REST headless di ArchMind (uso team/Enterprise).
//!
//! Espone il core su HTTP con:
//! - **registro multi-progetto** (portfolio) persistito su SQLite;
//! - **auth API-key + RBAC** (viewer < editor < admin);
//! - **audit log** di ogni richiesta autenticata;
//! - endpoint: analyze, doc, diagram, ask (RAG), snapshots, diff, gestione chiavi.
//!
//! L'LLM on-prem è supportato passando un provider Ollama nelle richieste `ask`.
//! Nota: l'autenticazione è a chiavi API con ruoli; SSO (OIDC/SAML) è un
//! successivo passo di integrazione.

use archmind_core::assistant::Provider;
use archmind_core::{diagrams, docs, evolution, project, store};
use axum::{
    extract::{Path, Query, Request, State},
    http::StatusCode,
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Ruolo dell'utente (RBAC). Ordine crescente di privilegio.
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Viewer,
    Editor,
    Admin,
}
impl Role {
    fn level(self) -> u8 {
        match self {
            Role::Viewer => 1,
            Role::Editor => 2,
            Role::Admin => 3,
        }
    }
    fn parse(s: &str) -> Option<Role> {
        match s {
            "viewer" => Some(Role::Viewer),
            "editor" => Some(Role::Editor),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }
}

/// Contesto di autenticazione propagato agli handler.
#[derive(Clone)]
struct AuthCtx {
    role: Role,
    label: String,
}

/// Stato condiviso dell'applicazione.
#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
}

#[tokio::main]
async fn main() {
    let db_path = std::env::var("ARCHMIND_DB").unwrap_or_else(|_| "archmind-server.db".into());
    let conn = Connection::open(&db_path).expect("apertura database");
    init_schema(&conn);
    bootstrap_admin(&conn);

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/projects", get(list_projects).post(create_project))
        .route("/projects/{id}/analyze", post(analyze_project))
        .route("/projects/{id}/doc", get(get_doc))
        .route("/projects/{id}/diagram", get(get_diagram))
        .route("/projects/{id}/ask", post(ask))
        .route("/projects/{id}/snapshots", get(get_snapshots))
        .route("/projects/{id}/diff", get(get_diff))
        .route("/admin/keys", post(create_key))
        .layer(from_fn_with_state(state.clone(), auth_and_audit))
        .with_state(state);

    let addr = std::env::var("ARCHMIND_ADDR").unwrap_or_else(|_| "0.0.0.0:7878".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    println!("ArchMind server in ascolto su http://{addr}");
    axum::serve(listener, app).await.expect("serve");
}

/// Crea le tabelle se non esistono.
fn init_schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, root TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS api_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT, key_hash TEXT NOT NULL UNIQUE,
            role TEXT NOT NULL, label TEXT NOT NULL, created_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT, ts INTEGER NOT NULL, actor TEXT NOT NULL,
            method TEXT NOT NULL, path TEXT NOT NULL, status INTEGER NOT NULL);",
    )
    .expect("schema");
}

/// Alla prima esecuzione crea una chiave admin e la stampa una sola volta.
fn bootstrap_admin(conn: &Connection) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM api_keys", [], |r| r.get(0))
        .unwrap_or(0);
    if count == 0 {
        let key = gen_key();
        insert_key(conn, &key, Role::Admin, "bootstrap-admin");
        println!("\n  Chiave ADMIN iniziale (salvala, non verrà più mostrata):\n    {key}\n");
    }
}

/// Genera una chiave API casuale.
fn gen_key() -> String {
    use rand::RngCore;
    let mut buf = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hash SHA-256 di una chiave (in DB non si salva mai il valore in chiaro).
fn hash_key(key: &str) -> String {
    let mut h = Sha256::new();
    h.update(key.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn insert_key(conn: &Connection, key: &str, role: Role, label: &str) {
    let role_s = match role {
        Role::Viewer => "viewer",
        Role::Editor => "editor",
        Role::Admin => "admin",
    };
    conn.execute(
        "INSERT INTO api_keys (key_hash, role, label, created_at) VALUES (?1,?2,?3,?4)",
        rusqlite::params![hash_key(key), role_s, label, now()],
    )
    .ok();
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Middleware: autentica (tranne /health), propaga il ruolo e registra l'audit.
async fn auth_and_audit(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().to_string();

    if path == "/health" {
        return next.run(req).await;
    }

    let key = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            req.headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .map(|s| s.to_string())
        });

    let ctx = key.as_ref().and_then(|k| lookup_key(&state, k));
    let Some(ctx) = ctx else {
        return (StatusCode::UNAUTHORIZED, "chiave API mancante o non valida").into_response();
    };

    let actor = ctx.label.clone();
    req.extensions_mut().insert(ctx);
    let resp = next.run(req).await;

    // Audit a posteriori (con lo status della risposta).
    if let Ok(db) = state.db.lock() {
        db.execute(
            "INSERT INTO audit (ts, actor, method, path, status) VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![now(), actor, method, path, resp.status().as_u16() as i64],
        )
        .ok();
    }
    resp
}

/// Risolve una chiave nel suo contesto (ruolo + label), se valida.
fn lookup_key(state: &AppState, key: &str) -> Option<AuthCtx> {
    let db = state.db.lock().ok()?;
    let hash = hash_key(key);
    db.query_row(
        "SELECT role, label FROM api_keys WHERE key_hash = ?1",
        [hash],
        |r| {
            let role: String = r.get(0)?;
            let label: String = r.get(1)?;
            Ok((role, label))
        },
    )
    .ok()
    .and_then(|(role, label)| Role::parse(&role).map(|role| AuthCtx { role, label }))
}

/// Verifica che il ruolo sia sufficiente.
fn require(ctx: &AuthCtx, min: Role) -> Result<(), Response> {
    if ctx.role.level() >= min.level() {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "privilegi insufficienti").into_response())
    }
}

/// Errore generico → risposta 500 con messaggio.
fn e500<E: std::fmt::Display>(e: E) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}

/// Restituisce la cartella radice di un progetto registrato.
fn project_root(state: &AppState, id: i64) -> Result<String, Response> {
    let db = state.db.lock().map_err(e500)?;
    db.query_row("SELECT root FROM projects WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|_| (StatusCode::NOT_FOUND, "progetto inesistente").into_response())
}

// --- Handler -----------------------------------------------------------------

async fn list_projects(State(state): State<AppState>, Extension(ctx): Extension<AuthCtx>) -> Response {
    if let Err(r) = require(&ctx, Role::Viewer) {
        return r;
    }
    let db = match state.db.lock() {
        Ok(d) => d,
        Err(e) => return e500(e),
    };
    let mut stmt = match db.prepare("SELECT id, name, root FROM projects ORDER BY id") {
        Ok(s) => s,
        Err(e) => return e500(e),
    };
    let rows = stmt
        .query_map([], |r| {
            Ok(json!({"id": r.get::<_, i64>(0)?, "name": r.get::<_, String>(1)?, "root": r.get::<_, String>(2)?}))
        })
        .and_then(|it| it.collect::<rusqlite::Result<Vec<_>>>());
    match rows {
        Ok(list) => Json(list).into_response(),
        Err(e) => e500(e),
    }
}

#[derive(Deserialize)]
struct NewProject {
    name: String,
    root: String,
}
async fn create_project(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthCtx>,
    Json(body): Json<NewProject>,
) -> Response {
    if let Err(r) = require(&ctx, Role::Admin) {
        return r;
    }
    let db = match state.db.lock() {
        Ok(d) => d,
        Err(e) => return e500(e),
    };
    match db.execute(
        "INSERT INTO projects (name, root) VALUES (?1, ?2)",
        rusqlite::params![body.name, body.root],
    ) {
        Ok(_) => Json(json!({"id": db.last_insert_rowid()})).into_response(),
        Err(e) => e500(e),
    }
}

async fn analyze_project(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(r) = require(&ctx, Role::Editor) {
        return r;
    }
    let root = match project_root(&state, id) {
        Ok(r) => r,
        Err(r) => return r,
    };
    let result = tokio::task::spawn_blocking(move || {
        let p = project::analyze(&root)?;
        store::save_snapshot(&p.root, &p, "server").ok();
        Ok::<_, archmind_core::Error>(p)
    })
    .await;
    match result {
        Ok(Ok(p)) => Json(p).into_response(),
        Ok(Err(e)) => e500(e),
        Err(e) => e500(e),
    }
}

#[derive(Deserialize)]
struct DocQuery {
    #[serde(default = "default_md")]
    format: String,
}
fn default_md() -> String {
    "md".into()
}
async fn get_doc(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<i64>,
    Query(q): Query<DocQuery>,
) -> Response {
    if let Err(r) = require(&ctx, Role::Viewer) {
        return r;
    }
    let root = match project_root(&state, id) {
        Ok(r) => r,
        Err(r) => return r,
    };
    let res = tokio::task::spawn_blocking(move || {
        let p = project::analyze(&root)?;
        let out = match q.format.as_str() {
            "html" => docs::html::render(&p),
            "wiki" => docs::wiki::render(&p),
            _ => docs::markdown::render(&p),
        };
        Ok::<_, archmind_core::Error>(out)
    })
    .await;
    match res {
        Ok(Ok(out)) => out.into_response(),
        Ok(Err(e)) => e500(e),
        Err(e) => e500(e),
    }
}

#[derive(Deserialize)]
struct DiagramQuery {
    kind: String,
    #[serde(default = "default_mermaid")]
    format: String,
}
fn default_mermaid() -> String {
    "mermaid".into()
}
async fn get_diagram(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<i64>,
    Query(q): Query<DiagramQuery>,
) -> Response {
    if let Err(r) = require(&ctx, Role::Viewer) {
        return r;
    }
    let root = match project_root(&state, id) {
        Ok(r) => r,
        Err(r) => return r,
    };
    let res = tokio::task::spawn_blocking(move || {
        let p = project::analyze(&root)?;
        diagrams::render(&p, &q.kind, &q.format)
    })
    .await;
    match res {
        Ok(Ok(out)) => out.into_response(),
        Ok(Err(e)) => e500(e),
        Err(e) => e500(e),
    }
}

#[derive(Deserialize)]
struct AskBody {
    question: String,
    provider: Provider,
}
async fn ask(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<i64>,
    Json(body): Json<AskBody>,
) -> Response {
    if let Err(r) = require(&ctx, Role::Viewer) {
        return r;
    }
    let root = match project_root(&state, id) {
        Ok(r) => r,
        Err(r) => return r,
    };
    let res = tokio::task::spawn_blocking(move || {
        let p = project::analyze(&root)?;
        archmind_core::assistant::ask(&p, &body.question, &body.provider)
    })
    .await;
    match res {
        Ok(Ok(ans)) => Json(ans).into_response(),
        Ok(Err(e)) => e500(e),
        Err(e) => e500(e),
    }
}

async fn get_snapshots(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(r) = require(&ctx, Role::Viewer) {
        return r;
    }
    let root = match project_root(&state, id) {
        Ok(r) => r,
        Err(r) => return r,
    };
    match store::list_snapshots(&root) {
        Ok(list) => Json(list).into_response(),
        Err(e) => e500(e),
    }
}

#[derive(Deserialize)]
struct DiffQuery {
    a: i64,
    b: i64,
}
async fn get_diff(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthCtx>,
    Path(id): Path<i64>,
    Query(q): Query<DiffQuery>,
) -> Response {
    if let Err(r) = require(&ctx, Role::Viewer) {
        return r;
    }
    let root = match project_root(&state, id) {
        Ok(r) => r,
        Err(r) => return r,
    };
    let old = match store::load_snapshot(&root, q.a) {
        Ok(p) => p,
        Err(e) => return e500(e),
    };
    let new = match store::load_snapshot(&root, q.b) {
        Ok(p) => p,
        Err(e) => return e500(e),
    };
    Json(evolution::diff(&old, &new)).into_response()
}

#[derive(Deserialize)]
struct NewKey {
    role: String,
    label: String,
}
async fn create_key(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthCtx>,
    Json(body): Json<NewKey>,
) -> Response {
    if let Err(r) = require(&ctx, Role::Admin) {
        return r;
    }
    let Some(role) = Role::parse(&body.role) else {
        return (StatusCode::BAD_REQUEST, "ruolo non valido (viewer|editor|admin)").into_response();
    };
    let key = gen_key();
    let db = match state.db.lock() {
        Ok(d) => d,
        Err(e) => return e500(e),
    };
    insert_key(&db, &key, role, &body.label);
    // La chiave in chiaro è restituita una sola volta al chiamante.
    Json(json!({"key": key, "role": body.role, "label": body.label})).into_response()
}
