<script>
  import { store } from "../lib/state.svelte.js";
  import { searchProject } from "../lib/api.js";

  let query = $state("");
  let hits = $state([]);

  async function run() {
    if (!store.project) return;
    hits = await searchProject(store.project, query);
  }
</script>

<h1>Ricerca</h1>
<input
  class="search"
  type="text"
  placeholder="Cerca tra componenti, endpoint, servizi, tabelle, dipendenze..."
  bind:value={query}
  oninput={run}
/>

{#if query && hits.length === 0}
  <p class="empty">Nessun risultato per "{query}".</p>
{:else if hits.length}
  <table style="margin-top:18px">
    <thead><tr><th>Tipo</th><th>Elemento</th><th>Posizione</th></tr></thead>
    <tbody>
      {#each hits as h}
        <tr><td><span class="chip">{h.kind}</span></td><td>{h.label}</td><td class="hint">{h.location}</td></tr>
      {/each}
    </tbody>
  </table>
{/if}
