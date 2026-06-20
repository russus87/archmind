<script>
  import { marked } from "marked";
  import { store } from "../lib/state.svelte.js";
  import { generateMarkdown, saveTextDialog } from "../lib/api.js";

  let md = $state("");
  let html = $state("");

  $effect(() => {
    build(store.project);
  });

  async function build(project) {
    if (!project) return;
    md = await generateMarkdown(project);
    html = marked.parse(md);
  }
</script>

<div class="toolbar">
  <h1 style="margin:0">Documentazione</h1>
  <span style="flex:1"></span>
  <button class="btn secondary" onclick={() => saveTextDialog(md, store.project.name + ".md")}>
    Esporta Markdown
  </button>
</div>

<div class="markdown">{@html html}</div>
