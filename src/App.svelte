<script>
  import { store } from "./lib/state.svelte.js";
  import { pickFolder, analyzeProject } from "./lib/api.js";
  import Sidebar from "./components/Sidebar.svelte";
  import Overview from "./components/Overview.svelte";
  import Tables from "./components/Tables.svelte";
  import Diagrams from "./components/Diagrams.svelte";
  import DocsView from "./components/DocsView.svelte";
  import SearchView from "./components/SearchView.svelte";
  import ChatView from "./components/ChatView.svelte";
  import EvolutionView from "./components/EvolutionView.svelte";

  async function openProject() {
    const root = await pickFolder();
    if (!root) return;
    store.loading = true;
    store.error = "";
    try {
      store.project = await analyzeProject(root);
      store.tab = "overview";
    } catch (e) {
      store.error = String(e);
    } finally {
      store.loading = false;
    }
  }
</script>

<div class="app">
  <Sidebar />

  <main class="main">
    <div class="toolbar">
      <button class="btn" onclick={openProject} disabled={store.loading}>
        {store.loading ? "Analisi in corso..." : "Apri progetto"}
      </button>
      {#if store.project}
        <span class="path">{store.project.root}</span>
      {/if}
    </div>

    {#if store.error}
      <div class="error">{store.error}</div>
    {/if}

    {#if !store.project}
      <div class="empty">
        <p style="font-size:18px">Nessun progetto analizzato.</p>
        <p class="hint">
          Apri una cartella: ArchMind scansiona codice, API, container, manifest e
          database e ne ricostruisce l'architettura.
        </p>
      </div>
    {:else if store.tab === "overview"}
      <Overview />
    {:else if store.tab === "components"}
      <Tables kind="components" />
    {:else if store.tab === "api"}
      <Tables kind="api" />
    {:else if store.tab === "services"}
      <Tables kind="services" />
    {:else if store.tab === "database"}
      <Tables kind="database" />
    {:else if store.tab === "evolution"}
      <EvolutionView />
    {:else if store.tab === "diagrams"}
      <Diagrams />
    {:else if store.tab === "docs"}
      <DocsView />
    {:else if store.tab === "search"}
      <SearchView />
    {:else if store.tab === "chat"}
      <ChatView />
    {/if}
  </main>
</div>
