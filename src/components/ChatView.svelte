<script>
  // L'assistente conversazionale (RAG sul progetto) arriva in V1.
  // Per l'MVP mostriamo lo stato e cosa servira' per abilitarlo.
  import { store } from "../lib/state.svelte.js";

  let messages = $state([
    {
      role: "assistant",
      text:
        "Ciao! Sono l'assistente di ArchMind. La chat con il progetto (RAG) sara' " +
        "attiva nella V1: indicizzero' codice e documentazione e rispondero' alle " +
        "domande sul funzionamento dell'applicazione. Per ora puoi usare la Ricerca.",
    },
  ]);
  let draft = $state("");

  function send() {
    if (!draft.trim()) return;
    messages.push({ role: "user", text: draft });
    messages.push({
      role: "assistant",
      text:
        "La generazione di risposte LLM non e' ancora collegata in questa build " +
        "(MVP). Configura un provider (Claude / Ollama) nella V1 per abilitarla.",
    });
    draft = "";
  }
</script>

<h1>Assistente</h1>
<div class="markdown" style="min-height:300px; display:flex; flex-direction:column; gap:12px">
  {#each messages as m}
    <div>
      <strong>{m.role === "user" ? "Tu" : "ArchMind"}:</strong>
      {m.text}
    </div>
  {/each}
</div>
<div class="toolbar" style="margin-top:16px">
  <input class="search" type="text" placeholder="Fai una domanda sul progetto..." bind:value={draft} />
  <button class="btn" onclick={send}>Invia</button>
</div>
<p class="hint">Progetto attivo: {store.project ? store.project.name : "nessuno"}</p>
