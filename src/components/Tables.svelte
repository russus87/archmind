<script>
  import { store } from "../lib/state.svelte.js";

  // Vista tabellare riusata per Componenti / API / Servizi / Database.
  let { kind } = $props();
  const p = $derived(store.project);
</script>

{#if kind === "components"}
  <h1>Componenti ({p.components.length})</h1>
  <table>
    <thead><tr><th>Nome</th><th>Tipo</th><th>Linguaggio</th><th>Percorso</th></tr></thead>
    <tbody>
      {#each p.components as c}
        <tr><td>{c.name}</td><td>{c.kind}</td><td>{c.language}</td><td><code>{c.path}</code></td></tr>
      {/each}
    </tbody>
  </table>
{:else if kind === "api"}
  <h1>API ({p.endpoints.length} endpoint)</h1>
  <table>
    <thead><tr><th>Metodo</th><th>Path</th><th>Operazione</th><th>File</th></tr></thead>
    <tbody>
      {#each p.endpoints as e}
        <tr><td><code>{e.method}</code></td><td><code>{e.path}</code></td><td>{e.operation_id ?? e.summary ?? ""}</td><td class="hint">{e.source}</td></tr>
      {/each}
    </tbody>
  </table>
{:else if kind === "services"}
  <h1>Servizi ({p.services.length})</h1>
  <table>
    <thead><tr><th>Nome</th><th>Immagine</th><th>Porte</th><th>Dipende da</th><th>Origine</th></tr></thead>
    <tbody>
      {#each p.services as s}
        <tr><td>{s.name}</td><td><code>{s.image ?? "-"}</code></td><td>{s.ports.join(", ") || "-"}</td><td>{s.depends_on.join(", ") || "-"}</td><td class="hint">{s.source}</td></tr>
      {/each}
    </tbody>
  </table>
{:else if kind === "database"}
  <h1>Database ({p.tables.length} tabelle)</h1>
  {#each p.tables as t}
    <h2>{t.schema ? t.schema + "." : ""}{t.name}</h2>
    <table>
      <thead><tr><th>Colonna</th><th>Tipo</th><th>Null</th><th>PK</th></tr></thead>
      <tbody>
        {#each t.columns as col}
          <tr><td>{col.name}</td><td><code>{col.data_type}</code></td><td>{col.nullable ? "si" : "no"}</td><td>{col.primary_key ? "PK" : ""}</td></tr>
        {/each}
      </tbody>
    </table>
    {#each t.foreign_keys as fk}
      <p class="hint">FK: <code>{fk.column}</code> → <code>{fk.references_table}({fk.references_column})</code></p>
    {/each}
  {/each}
{/if}
