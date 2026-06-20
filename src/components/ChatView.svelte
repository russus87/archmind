<script>
  // Assistente RAG ("chat con il progetto"): recupero via indice + LLM.
  import { marked } from "marked";
  import { store } from "../lib/state.svelte.js";
  import { ask } from "../lib/api.js";

  let messages = $state([
    {
      role: "assistant",
      text:
        "Ciao! Sono l'assistente di ArchMind. Fammi domande sul funzionamento " +
        "del progetto: recupero gli estratti rilevanti dal codice e rispondo " +
        "citando le fonti. Configura il provider qui sotto (Claude o Ollama locale).",
      citations: [],
    },
  ]);
  let draft = $state("");
  let busy = $state(false);
  let error = $state("");
  let showSettings = $state(false);

  // Costruisce l'oggetto provider per il backend dallo stato condiviso.
  function provider() {
    const p = store.provider;
    return p.kind === "ollama"
      ? { kind: "ollama", host: p.host, model: p.ollamaModel }
      : { kind: "claude", api_key: p.api_key, model: p.model };
  }

  async function send() {
    const q = draft.trim();
    if (!q || busy || !store.project) return;
    error = "";
    messages.push({ role: "user", text: q, citations: [] });
    draft = "";
    busy = true;
    try {
      const ans = await ask(store.project, q, provider());
      messages.push({ role: "assistant", text: ans.text, citations: ans.citations });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="toolbar">
  <h1 style="margin:0">Assistente</h1>
  <span style="flex:1"></span>
  <button class="btn secondary" onclick={() => (showSettings = !showSettings)}>
    Provider: {store.provider.kind === "ollama" ? "Ollama" : "Claude"}
  </button>
</div>

{#if showSettings}
  <div class="markdown" style="margin-bottom:16px">
    <div class="diagram-tabs">
      <button class="chip" class:active={store.provider.kind === "claude"} onclick={() => (store.provider.kind = "claude")}>Claude</button>
      <button class="chip" class:active={store.provider.kind === "ollama"} onclick={() => (store.provider.kind = "ollama")}>Ollama (locale)</button>
    </div>
    {#if store.provider.kind === "claude"}
      <p class="hint">Chiave API e modello (default <code>claude-opus-4-8</code>). La chiave resta locale.</p>
      <input class="search" type="password" placeholder="ANTHROPIC_API_KEY" bind:value={store.provider.api_key} style="margin-bottom:8px" />
      <input class="search" type="text" placeholder="modello" bind:value={store.provider.model} />
    {:else}
      <p class="hint">Host Ollama e nome modello (tutto in locale, nessun dato esce).</p>
      <input class="search" type="text" placeholder="http://localhost:11434" bind:value={store.provider.host} style="margin-bottom:8px" />
      <input class="search" type="text" placeholder="modello (es. llama3.1)" bind:value={store.provider.ollamaModel} />
    {/if}
  </div>
{/if}

{#if error}<div class="error">{error}</div>{/if}

<div class="markdown" style="min-height:280px; display:flex; flex-direction:column; gap:16px">
  {#each messages as m}
    <div>
      <strong>{m.role === "user" ? "Tu" : "ArchMind"}:</strong>
      {#if m.role === "assistant"}
        {@html marked.parse(m.text)}
        {#if m.citations && m.citations.length}
          <p class="hint" style="margin-top:6px">Fonti:</p>
          <ol class="hint" style="margin:0">
            {#each m.citations as c}
              <li><span class="chip">{c.kind}</span> {c.location || c.title}</li>
            {/each}
          </ol>
        {/if}
      {:else}
        {m.text}
      {/if}
    </div>
  {/each}
  {#if busy}<div class="hint">Sto pensando…</div>{/if}
</div>

<div class="toolbar" style="margin-top:16px">
  <input
    class="search"
    type="text"
    placeholder="Fai una domanda sul progetto…"
    bind:value={draft}
    onkeydown={(e) => e.key === "Enter" && send()}
    disabled={busy}
  />
  <button class="btn" onclick={send} disabled={busy}>Invia</button>
</div>
<p class="hint">Progetto attivo: {store.project ? store.project.name : "nessuno"}</p>
