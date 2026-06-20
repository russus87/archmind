<script>
  import mermaid from "mermaid";
  import { store } from "../lib/state.svelte.js";
  import { generateDiagram, saveTextDialog } from "../lib/api.js";

  mermaid.initialize({ startOnLoad: false, theme: "dark", securityLevel: "loose" });

  const kinds = [
    { id: "dependency", label: "Dipendenze" },
    { id: "component", label: "Componenti" },
    { id: "er", label: "ER (Database)" },
    { id: "class", label: "Class Diagram" },
    { id: "sequence", label: "Sequence" },
    { id: "flow", label: "Flusso (cross-layer)" },
  ];

  let kind = $state("dependency");
  let source = $state("");
  let svg = $state("");
  let error = $state("");

  // Rigenera e disegna il diagramma a ogni cambio di tipo o progetto.
  $effect(() => {
    render(kind, store.project);
  });

  async function render(k, project) {
    if (!project) return;
    error = "";
    try {
      source = await generateDiagram(project, k);
      const { svg: out } = await mermaid.render("graph_" + k, source);
      svg = out;
    } catch (e) {
      error = String(e);
      svg = "";
    }
  }
</script>

<h1>Diagrammi</h1>
<div class="diagram-tabs">
  {#each kinds as k}
    <button class="chip" class:active={kind === k.id} onclick={() => (kind = k.id)}>{k.label}</button>
  {/each}
  <button class="chip" onclick={() => saveTextDialog(source, kind + ".mmd")}>Esporta .mmd</button>
</div>

{#if error}<div class="error">{error}</div>{/if}
<div class="markdown">
  {@html svg}
</div>
