<div align="center">
  <img src="assets/icon-source.png" width="120" alt="ArchMind" />
  <h1>ArchMind</h1>
  <p><strong>Analizza progetti software esistenti e genera documentazione tecnica sempre aggiornata.</strong></p>
  <p>Reverse engineering · diagrammi automatici · analisi database · assistente che conosce il progetto.</p>
  <p>Rust + Tauri + Svelte · Windows · Linux · macOS (Apple Silicon)</p>
</div>

---

ArchMind apre una cartella di progetto, ne scansiona il codice, le API, i
container, i manifest e il database, e ricostruisce l'**architettura** in un
unico modello da cui genera **documentazione**, **diagrammi** e risposte a
domande sul funzionamento dell'applicazione.

Tutto gira in locale: il codice sorgente non lascia la macchina (le funzioni
AI cloud sono opt-in).

## Cosa analizza

- **Repository Git** — metadati del repo
- **C#** (`.cs`, `.csproj`) e **Java** (`.java`, `pom.xml`, Gradle) — namespace,
  package, classi, interfacce, metodi e **grafo delle chiamate** (parsing reale
  via [tree-sitter](https://tree-sitter.github.io))
- **Database** — DDL SQL (Oracle/PostgreSQL): tabelle, colonne, chiavi esterne
- **OpenAPI / Swagger** — endpoint (metodo, path, operationId)
- **Docker Compose** — servizi, immagini, porte, `depends_on`
- **Kubernetes** — Deployment, Service, ecc.
- **File di configurazione** — `.env`, `appsettings.json`, `application.properties/yml`
- **Dipendenze** — NuGet, Maven, npm

## Cosa genera

- **Documentazione** Markdown (HTML/PDF/Wiki in roadmap)
- **Diagrammi** Mermaid: dependency graph, component, **ER**, class diagram (con
  archi delle chiamate), sequence
- **Ricerca** full-text su tutto il progetto
- **Assistente RAG**: chat con il progetto (retrieval [tantivy](https://github.com/quickwit-oss/tantivy)
  + LLM **Claude** o **Ollama** locale), con citazioni alle fonti

## CLI & docs-as-code

Oltre al desktop c'è una **CLI headless** (`archmind-cli`) che riusa lo stesso
core, pensata per la CI:

```bash
archmind-cli analyze . --out docs --diagrams   # genera documentazione + diagrammi
archmind-cli check   . --out docs              # gate CI: fallisce se la doc è in drift
archmind-cli ask     . --question "come funziona X?"   # assistente RAG da terminale
```

Il workflow [`.github/workflows/docs.yml`](.github/workflows/docs.yml) esegue
`check` a ogni push: la documentazione resta sempre allineata al codice.

## Funzionalità (fatto / roadmap)

| Area | Fatto | Roadmap |
|---|---|---|
| Reverse engineering | C#/Java via **tree-sitter** + call graph, deps, servizi, DDL | DB live, sidecar Roslyn |
| Documentazione | Markdown + **CLI/CI docs-as-code** | HTML, PDF, Wiki |
| Diagrammi | Mermaid (5 tipi) | PlantUML, Graphviz |
| Knowledge | **ricerca full-text + RAG con citazioni** (Claude/Ollama) | indice vettoriale denso |
| Evoluzione | — | confronto versioni, analisi d'impatto |

Dettagli completi (architettura, modello dati, API, indicizzazione, AI,
roadmap MVP→V1→V2→Enterprise): **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)**.

## Sviluppo

Prerequisiti: [Rust](https://rustup.rs), [Node 20+](https://nodejs.org) e le
dipendenze Tauri per il tuo SO ([guida](https://tauri.app/start/prerequisites/)).

```bash
npm install
npm run tauri dev      # avvia l'app in sviluppo
npm run tauri build    # crea i pacchetti per il tuo SO
```

Il "cervello" è in `core/` (crate Rust puro, senza Tauri): la stessa logica
potrà alimentare in futuro una CLI e un server headless.

## Build & release

Il workflow [`.github/workflows/release.yml`](.github/workflows/release.yml)
compila e pubblica a ogni tag `v*`:

- **Linux** — `.AppImage`, `.deb`, `.rpm`
- **Arch Linux** — `.pkg.tar.zst` (job dedicato via `makepkg`)
- **Windows** — `.exe` (NSIS) e `.msi`
- **macOS** — Apple Silicon (M1+)

```bash
git tag v0.1.0 && git push origin v0.1.0   # avvia la release (draft)
```

## Licenza

MIT
