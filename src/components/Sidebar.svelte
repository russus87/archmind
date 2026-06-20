<script>
  import { store } from "../lib/state.svelte.js";

  // Le voci sono disabilitate finche' non c'e' un progetto analizzato.
  const items = [
    { id: "overview", label: "Panoramica" },
    { id: "components", label: "Componenti" },
    { id: "api", label: "API" },
    { id: "services", label: "Servizi" },
    { id: "database", label: "Database" },
    { id: "diagrams", label: "Diagrammi" },
    { id: "docs", label: "Documentazione" },
    { id: "search", label: "Ricerca" },
    { id: "chat", label: "Assistente" },
  ];
</script>

<aside class="sidebar">
  <div class="brand">
    ArchMind
    <small>Reverse engineering &amp; docs</small>
  </div>
  {#each items as it}
    <button
      class="nav-item"
      class:active={store.tab === it.id}
      disabled={!store.project && it.id !== "overview"}
      onclick={() => (store.tab = it.id)}
    >
      {it.label}
    </button>
  {/each}
</aside>
