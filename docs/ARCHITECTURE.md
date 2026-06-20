# ArchMind — Architettura & Design

> Piattaforma desktop che analizza progetti software esistenti e genera
> documentazione tecnica sempre aggiornata: reverse engineering, diagrammi
> automatici, analisi del database e un assistente che "conosce" il progetto.

Documento di progettazione: architettura, stack, modello dati, API interne,
strategia di indicizzazione, integrazioni AI e roadmap.

---

## 1. Principi guida

1. **Local-first.** Il codice sorgente dei clienti non lascia la macchina per
   default. L'analisi gira in locale; le funzioni AI cloud sono opt-in e
   sostituibili da modelli locali (Ollama).
2. **Core riutilizzabile.** Tutta la logica vive in un crate Rust puro
   (`archmind-core`) senza dipendenze da Tauri: la stessa logica alimenta
   l'app desktop, una futura CLI e un futuro server headless/CI.
3. **Cross-platform reale.** Windows, Linux (AppImage/.deb/.rpm/.pkg.tar.zst) e
   macOS Apple Silicon, dallo stesso codice. Niente dipendenze di sistema
   esotiche nel core (solo pure-Rust) per pacchettizzare senza attriti.
4. **Modello unico.** Ogni analyzer scrive in un unico *grafo di conoscenza*
   (`Project`). Documentazione, diagrammi, ricerca e chat leggono solo da lì:
   aggiungere una sorgente non cambia i consumatori.
5. **Tolleranza al rumore.** Un file malformato non interrompe mai l'analisi:
   ogni analyzer degrada con grazia.

---

## 2. Architettura complessiva

```
┌──────────────────────────────────────────────────────────────┐
│                        ArchMind Desktop                        │
│                    (Tauri 2 — finestra nativa)                 │
│                                                                │
│  ┌───────────────────────────┐   IPC    ┌──────────────────┐  │
│  │   Frontend (Svelte 5)     │ <──────> │  Backend (Rust)  │  │
│  │  - Overview / Tabelle     │  invoke  │  src-tauri/lib   │  │
│  │  - Diagrammi (Mermaid)    │  commands│  (adattatori)    │  │
│  │  - Documentazione (md)    │          └────────┬─────────┘  │
│  │  - Ricerca / Assistente   │                   │            │
│  └───────────────────────────┘                   ▼            │
│                                        ┌──────────────────────┐│
│                                        │   archmind-core      ││
│                                        │  (Rust puro)         ││
│                                        │                      ││
│   analyzers/        project::analyze   │  ┌────────────────┐  ││
│   ├─ git            ───────────────►   │  │   Project      │  ││
│   ├─ csharp                            │  │ (grafo di      │  ││
│   ├─ java                              │  │  conoscenza)   │  ││
│   ├─ database (DDL)                    │  └───────┬────────┘  ││
│   ├─ openapi                           │          │           ││
│   ├─ docker_compose                    │   ┌──────┴───────┐   ││
│   ├─ kubernetes        docs/ diagrams/ │   │ render/query │   ││
│   ├─ config            search/ (RAG)   │   └──────────────┘   ││
│   └─ deps                              └──────────────────────┘│
└──────────────────────────────────────────────────────────────┘
        │ (opzionale, opt-in)
        ▼
┌──────────────────────┐   ┌─────────────────────┐
│  Provider LLM cloud  │   │  LLM locale (Ollama)│
│  (Claude API)        │   │  + embeddings ONNX  │
└──────────────────────┘   └─────────────────────┘
```

**Tre livelli:**

- **Frontend Svelte** — solo presentazione. Chiama il backend via `invoke`,
  non contiene logica di analisi.
- **Backend Tauri** (`src-tauri/src/lib.rs`) — sottili adattatori: invocano il
  core, serializzano in JSON, traducono gli errori per la UI.
- **Core** (`archmind-core`) — analisi, modello, generazione doc/diagrammi,
  ricerca e (in roadmap) RAG. È l'unico punto in cui vive la conoscenza del
  dominio.

---

## 3. Stack tecnologico

| Strato | Tecnologia | Perché |
|---|---|---|
| Shell desktop | **Tauri 2** | Binari piccoli, webview nativa, multipiattaforma, già usato negli altri progetti |
| Linguaggio core | **Rust** | Sicurezza, performance, parsing veloce, un solo binario portabile |
| Frontend | **Svelte 5 + Vite 6** | Reattività con i *rune*, bundle leggero, coerente col resto dei progetti |
| Diagrammi | **Mermaid 11** | Testo → diagramma, incorporabile in Markdown/Wiki; export PlantUML/DOT in roadmap |
| Markdown UI | **marked** | Render veloce della doc generata |
| YAML/JSON | **serde_yaml / serde_json** | OpenAPI, Compose, K8s, config |
| Scansione FS | **walkdir** | Attraversamento ricorsivo con skip delle cartelle di build |
| Estrazione simboli | **regex** (MVP) → **tree-sitter** (V1) | Euristica veloce ora, parsing sintattico robusto poi |
| Full-text (V1) | **tantivy** | Motore di ricerca Rust, in-process, niente server |
| Vettoriale (V1) | **sqlite + sqlite-vec** o **hnsw** | RAG locale senza dipendenze esterne |
| Persistenza | **SQLite** (file di progetto `.archmind`) | Cache analisi, storico versioni, indici |
| LLM | **Claude API** (cloud) / **Ollama** (locale) | Chat e sintesi; pluggable e opt-in |

**Packaging / CI** (workflow `release.yml`, identico al pattern degli altri repo):

- macOS `aarch64-apple-darwin` (M1+) → `.dmg` / `.app`
- Ubuntu 22.04 → `.AppImage`, `.deb`, `.rpm`
- Windows → `.exe` (NSIS) e `.msi`
- Arch Linux → `.pkg.tar.zst` (job dedicato con `makepkg` in container)

---

## 4. Modello dati (il grafo di conoscenza)

Tutto converge in `Project` (`core/src/model.rs`). Entità tipizzate + relazioni
generiche.

```
Project
├─ root, name, stats{files, loc, by_extension}
├─ components[]   (Namespace | Package | Class | Interface | Module)
│                  id, name, kind, language, path, members[]
├─ endpoints[]    (method, path, operationId, summary, source)  ← OpenAPI
├─ services[]     (name, image, ports[], depends_on[], source)  ← Compose/K8s
├─ tables[]       (name, schema, columns[], foreign_keys[])     ← DDL SQL
│   ├─ Column     (name, data_type, nullable, primary_key)
│   └─ ForeignKey (column, references_table, references_column)
├─ dependencies[] (name, version, ecosystem, declared_in)       ← NuGet/Maven/npm
└─ relations[]    (from, to, kind: DependsOn|Exposes|References|Contains)
```

**Perché entità tipizzate + archi generici:** le tabelle/endpoint hanno campi
specifici che la doc deve mostrare; gli archi generici (`relations`) tengono il
grafo estendibile (chiamate, lettura/scrittura DB, esposizione API) senza
toccare le strutture esistenti. ER e dependency graph si derivano dagli archi.

**Persistenza (V1):** il modello viene serializzato in un file di progetto
SQLite (`<root>/.archmind/index.db`) con: snapshot del `Project`, indice
full-text tantivy, embeddings, e tabella `snapshots` per lo storico versioni
(vedi §8 evoluzione architetturale).

---

## 5. API interne (comandi Tauri)

Il contratto tra UI e core. Ogni comando è un adattatore sottile.

| Comando | Input | Output | Stato |
|---|---|---|---|
| `analyze_project` | `root: String` | `Project` | ✅ MVP |
| `generate_markdown` | `project: Project` | `String` (md) | ✅ MVP |
| `generate_diagram` | `project, kind` | `String` (Mermaid) | ✅ MVP |
| `search_project` | `project, query` | `Hit[]` | ✅ MVP (full-text in V1) |
| `save_text` | `path, content` | `()` | ✅ MVP |
| `export_html` / `export_pdf` | `project, opts` | file | 🔜 V1 |
| `connect_database` | `dsn, kind` | `Table[]` (introspezione live) | 🔜 V1 |
| `ask` | `project, question, provider` | `Answer` (testo + citazioni) | ✅ MVP |
| `index_project` (interno) | `project, query` | passaggi rilevanti (tantivy) | ✅ MVP |
| `diff_snapshots` | `a, b` | `ChangeSet` (impatto) | 🔜 V2 |

`kind` dei diagrammi: `dependency | component | er | class | sequence`.

> Una futura **CLI** (`archmind analyze <path> --out docs/`) e un **server
> headless** per la CI useranno lo stesso `archmind-core`, senza Tauri.

---

## 6. Strategia di indicizzazione

Tre indici complementari, tutti in-process e locali:

1. **Indice strutturale** — il `Project` stesso (grafo). Risponde a domande
   precise ("quali servizi dipendono da X", "quali tabelle referenzia Y").
   Persistito in SQLite.
2. **Indice full-text (tantivy)** — un documento per entità (componente,
   endpoint, servizio, tabella, chunk di codice) con campi `kind`, `name`,
   `path`, `body`. Ricerca per termini, prefissi, fuzzy.
3. **Indice semantico (vettoriale)** — chunk di codice e documentazione →
   embeddings (modello ONNX locale, es. `bge-small`, oppure provider cloud) →
   indice HNSW / `sqlite-vec`. È la base del RAG dell'assistente.

**Pipeline di indicizzazione (V1):**

```
file → chunking (per simbolo: classe/metodo, o a finestra) →
   ├─ tantivy.add(doc)            (full-text)
   └─ embed(chunk) → vec_index    (semantico)
```

**Incrementalità:** ogni chunk porta un hash del contenuto. Alla rianalisi si
re-indicizzano solo i chunk cambiati (confronto hash), così i progetti grandi
si aggiornano in secondi. L'hash alimenta anche il diff fra versioni (§8).

---

## 7. Integrazioni AI

**Assistente (RAG) — "Chat con il progetto":**

```
domanda utente
  → retrieval ibrido: tantivy (lessicale) + vec_index (semantico)
  → re-ranking + assemblaggio contesto (snippet + percorso + metadati grafo)
  → LLM (Claude / Ollama) con citazioni ai file
  → risposta in streaming con riferimenti cliccabili
```

- **Provider pluggable.** Default consigliato: **Claude** (Opus 4.8 per sintesi
  profonde, Sonnet 4.6 per il Q&A interattivo, Haiku 4.5 per task economici);
  alternativa **locale via Ollama** per ambienti air-gapped.
- **Privacy.** Il RAG cloud invia solo gli snippet recuperati, non l'intero
  repo; modalità "solo locale" disattiva ogni chiamata di rete.

**Altri usi dell'AI:**

- **Sintesi architetturale** — descrizioni in linguaggio naturale di moduli e
  servizi a partire dal grafo, incluse nella documentazione generata.
- **Naming/normalizzazione** — raggruppamento di componenti, inferenza dei
  *bounded context*.
- **Analisi d'impatto assistita** — spiegazione in linguaggio naturale di cosa
  cambia tra due versioni (§8).
- **Analisi query** — spiegazione/ottimizzazione di query SQL trovate nel
  codice, suggerimento indici (V2).

---

## 8. Evoluzione architetturale (confronto versioni)

Ogni analisi può salvare uno **snapshot** del `Project` (con hash per entità).

```
snapshot(A) ─┐
             ├─► diff_snapshots(A, B) ─► ChangeSet
snapshot(B) ─┘                            ├─ added/removed/modified:
                                          │   componenti, endpoint, servizi, tabelle
                                          └─ impatto: traversamento del grafo
                                              (chi dipende dall'elemento cambiato?)
```

- **Individuazione modifiche:** diff per entità basato su id + hash.
- **Impatto:** dato un elemento modificato, si risale `relations` per elencare
  i dipendenti transitivi (es. "modificata la tabella ORDERS → impattati 3
  servizi e 7 endpoint").
- **Confronto due cartelle / due tag Git:** si analizzano entrambe le revisioni
  e si confrontano gli snapshot.

---

## 9. Sicurezza & privacy

- Nessuna telemetria di default; nessun invio di codice senza azione esplicita.
- Le credenziali DB (per l'introspezione live) restano nel keychain del SO.
- Capabilities Tauri minime (`src-tauri/capabilities/default.json`): dialog,
  opener, notification, process — niente accesso FS arbitrario non mediato.

---

## 10. Roadmap

### MVP (questa base) ✅
- Scansione progetto multipiattaforma, deterministica, con skip degli artefatti.
- Analyzer: Git (metadati), **C# e Java via tree-sitter** (tipi, metodi e grafo
  delle chiamate), OpenAPI/Swagger, Docker Compose, Kubernetes, file di config,
  DDL SQL (tabelle/colonne/FK), dipendenze (NuGet/Maven/npm).
- Modello unico `Project`; documentazione Markdown; diagrammi Mermaid
  (dependency, component, ER, class con call graph, sequence).
- **Ricerca full-text + Assistente RAG** (retrieval tantivy + LLM Claude/Ollama,
  con citazioni alle fonti).
- **CLI headless** (`archmind-cli`) e **CI docs-as-code** (`check` fallisce sul drift).
- UI desktop completa; build CI per Linux/Win/macOS/Arch.

### V1 — Profondità e conoscenza
- tree-sitter per altri linguaggi (TS/Python/Go) e linking cross-layer
  (controller → service → tabella) per il flusso applicativo.
- **Connessione DB live** (PostgreSQL via `tokio-postgres`, Oracle) →
  introspezione schema, relazioni, analisi query.
- **Indice vettoriale denso** (embeddings ONNX) accanto al BM25, per RAG ibrido.
- **Export** HTML e PDF; tema della documentazione.
- File di progetto SQLite con cache incrementale.

### V2 — Evoluzione & integrazioni
- **Confronto versioni** e **analisi d'impatto** sul grafo (§8).
- Export verso **Wiki** (Confluence, Azure DevOps, GitHub/GitLab Wiki).
- Analisi semantica C# via **sidecar Roslyn** opzionale; Java via JavaParser.
- Diagrammi **PlantUML** e **Graphviz (DOT)**; viste filtrabili.
- CLI headless per pipeline CI ("docs-as-code": doc rigenerata a ogni push).

### Enterprise
- **Server / modalità team** headless con lo stesso core: analisi centralizzata,
  doc pubblicata su portale interno, API REST.
- **SSO/RBAC**, multi-repo e multi-progetto, dashboard di portfolio.
- **Integrazione CI/CD** (gate su drift di documentazione), webhook.
- LLM **on-prem** (vLLM/Ollama in cluster) per ambienti regolamentati.
- Audit log, policy di data residency, supporto e SLA.

---

## 11. Struttura del repository

```
archmind/
├─ core/                  # archmind-core: analisi pura Rust (riutilizzabile)
│  └─ src/
│     ├─ model.rs         # il grafo di conoscenza (Project)
│     ├─ project.rs       # orchestrazione dell'analisi
│     ├─ analyzers/       # git, treesitter (csharp/java), database, openapi,
│     │                   #   docker_compose, kubernetes, config, deps, stats
│     ├─ docs/            # generazione documentazione (markdown)
│     ├─ diagrams/        # generazione diagrammi (mermaid)
│     ├─ index.rs         # indice full-text tantivy (retrieval RAG)
│     ├─ assistant/       # RAG: provider LLM (claude, ollama) + orchestrazione
│     └─ search.rs        # ricerca full-text semplice (live, in UI)
├─ cli/                   # archmind-cli: analyze / check (docs-as-code) / ask
├─ src-tauri/             # backend Tauri (adattatori) + config + icone
├─ src/                   # frontend Svelte 5
├─ packaging/             # PKGBUILD + .desktop per Arch Linux
├─ .github/workflows/     # release.yml (build multipiattaforma)
└─ docs/ARCHITECTURE.md   # questo documento
```
