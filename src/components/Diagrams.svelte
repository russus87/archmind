<script>
  import mermaid from "mermaid";
  import { store } from "../lib/state.svelte.js";
  import { generateDiagramFmt, saveTextDialog } from "../lib/api.js";

  mermaid.initialize({ startOnLoad: false, theme: "dark", securityLevel: "loose" });

  const kinds = [
    { id: "dependency", label: "Dipendenze" },
    { id: "component", label: "Componenti" },
    { id: "er", label: "ER (Database)" },
    { id: "class", label: "Class Diagram" },
    { id: "sequence", label: "Sequence" },
    { id: "flow", label: "Flusso (cross-layer)" },
  ];
  const formats = [
    { id: "mermaid", label: "Mermaid", ext: "mmd" },
    { id: "plantuml", label: "PlantUML", ext: "puml" },
    { id: "dot", label: "Graphviz", ext: "dot" },
  ];

  let kind = $state("dependency");
  let format = $state("mermaid");
  let source = $state("");
  let svg = $state("");
  let error = $state("");

  $effect(() => {
    render(kind, format, store.project);
  });

  async function render(k, f, project) {
    if (!project) return;
    error = "";
    svg = "";
    try {
      source = await generateDiagramFmt(project, k, f);
      if (f === "mermaid") {
        const { svg: out } = await mermaid.render("graph_" + k, source);
        svg = out;
      }
    } catch (e) {
      error = String(e);
      source = "";
    }
  }

  const ext = $derived(formats.find((f) => f.id === format)?.ext ?? "txt");
</script>

<h1>Diagrammi</h1>
<div class="diagram-tabs">
  {#each kinds as k}
    <button class="chip" class:active={kind === k.id} onclick={() => (kind = k.id)}>{k.label}</button>
  {/each}
</div>
<div class="diagram-tabs">
  {#each formats as f}
    <button class="chip" class:active={format === f.id} onclick={() => (format = f.id)}>{f.label}</button>
  {/each}
  <button class="chip" onclick={() => saveTextDialog(source, kind + "." + ext)}>Esporta .{ext}</button>
</div>

{#if error}<div class="error">{error}</div>{/if}
<div class="markdown">
  {#if format === "mermaid"}
    {@html svg}
  {:else}
    <p class="hint">Sorgente {format} (rendi con uno strumento {format}, o esporta):</p>
    <pre style="white-space:pre-wrap; overflow:auto">{source}</pre>
  {/if}
</div>
