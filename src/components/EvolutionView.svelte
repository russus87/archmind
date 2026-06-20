<script>
  // Evoluzione architetturale: snapshot del progetto, confronto tra versioni
  // e analisi d'impatto.
  import { store } from "../lib/state.svelte.js";
  import {
    saveSnapshot,
    listSnapshots,
    diffSnapshots,
    diffAgainstCurrent,
  } from "../lib/api.js";

  let snapshots = $state([]);
  let label = $state("");
  let a = $state(null); // id snapshot A
  let b = $state("current"); // id snapshot B oppure "current"
  let changes = $state(null);
  let error = $state("");
  let busy = $state(false);

  $effect(() => {
    refresh(store.project);
  });

  async function refresh(project) {
    if (!project) return;
    try {
      snapshots = await listSnapshots(project.root);
      if (snapshots.length && a === null) a = snapshots[0].id;
    } catch (e) {
      error = String(e);
    }
  }

  function fmt(ts) {
    return new Date(ts * 1000).toLocaleString();
  }

  async function save() {
    error = "";
    busy = true;
    try {
      await saveSnapshot(store.project, label || "snapshot");
      label = "";
      await refresh(store.project);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function compare() {
    error = "";
    changes = null;
    busy = true;
    try {
      changes =
        b === "current"
          ? await diffAgainstCurrent(a, store.project)
          : await diffSnapshots(store.project.root, a, Number(b));
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<h1>Evoluzione</h1>

<div class="toolbar">
  <input class="search" type="text" placeholder="Etichetta snapshot (es. v1.2)" bind:value={label} style="max-width:260px" />
  <button class="btn" onclick={save} disabled={busy}>Salva snapshot</button>
</div>

{#if error}<div class="error">{error}</div>{/if}

{#if snapshots.length === 0}
  <p class="hint">Nessuno snapshot salvato. Salvane uno per iniziare a confrontare le versioni.</p>
{:else}
  <h2>Confronta</h2>
  <div class="toolbar">
    <label class="hint">Da
      <select bind:value={a}>
        {#each snapshots as s}<option value={s.id}>{s.label} — {fmt(s.created_at)}</option>{/each}
      </select>
    </label>
    <label class="hint">a
      <select bind:value={b}>
        <option value="current">stato attuale</option>
        {#each snapshots as s}<option value={s.id}>{s.label} — {fmt(s.created_at)}</option>{/each}
      </select>
    </label>
    <button class="btn" onclick={compare} disabled={busy || a === null}>Confronta</button>
  </div>
{/if}

{#if changes}
  <div class="cards">
    <div class="card"><div class="n">{changes.added.length}</div><div class="l">Aggiunti</div></div>
    <div class="card"><div class="n">{changes.removed.length}</div><div class="l">Rimossi</div></div>
    <div class="card"><div class="n">{changes.modified.length}</div><div class="l">Modificati</div></div>
    <div class="card"><div class="n">{changes.impacted.length}</div><div class="l">Con impatto</div></div>
  </div>

  {#each [["Aggiunti", changes.added], ["Rimossi", changes.removed], ["Modificati", changes.modified]] as [title, list]}
    {#if list.length}
      <h2>{title}</h2>
      <table>
        <thead><tr><th>Tipo</th><th>Elemento</th></tr></thead>
        <tbody>
          {#each list as c}<tr><td><span class="chip">{c.kind}</span></td><td>{c.label}</td></tr>{/each}
        </tbody>
      </table>
    {/if}
  {/each}

  {#if changes.impacted.length}
    <h2>Analisi d'impatto</h2>
    <p class="hint">Chi dipende dagli elementi cambiati (a monte nel grafo):</p>
    {#each changes.impacted as imp}
      <p><strong>{imp.label}</strong> → {imp.dependents.join(", ")}</p>
    {/each}
  {/if}
{/if}
