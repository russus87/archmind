<script>
  import { store } from "../lib/state.svelte.js";

  const p = $derived(store.project);
  const langs = $derived(
    p
      ? Object.entries(p.stats.by_extension).filter(([k]) => !k.startsWith("__"))
      : [],
  );
</script>

{#if p}
  <h1>{p.name}</h1>
  <div class="cards">
    <div class="card"><div class="n">{p.stats.files}</div><div class="l">File</div></div>
    <div class="card"><div class="n">{p.stats.lines_of_code}</div><div class="l">Righe di codice</div></div>
    <div class="card"><div class="n">{p.components.length}</div><div class="l">Componenti</div></div>
    <div class="card"><div class="n">{p.endpoints.length}</div><div class="l">Endpoint API</div></div>
    <div class="card"><div class="n">{p.services.length}</div><div class="l">Servizi</div></div>
    <div class="card"><div class="n">{p.tables.length}</div><div class="l">Tabelle DB</div></div>
    <div class="card"><div class="n">{p.dependencies.length}</div><div class="l">Dipendenze</div></div>
  </div>

  <h2>Composizione</h2>
  <table>
    <thead><tr><th>Estensione</th><th>File</th></tr></thead>
    <tbody>
      {#each langs as [ext, n]}
        <tr><td><code>{ext}</code></td><td>{n}</td></tr>
      {/each}
    </tbody>
  </table>
{/if}
