<script>
  import { marked } from "marked";
  import { store } from "../lib/state.svelte.js";
  import { generateMarkdown, exportDoc } from "../lib/api.js";

  let md = $state("");
  let html = $state("");
  let msg = $state("");

  $effect(() => {
    build(store.project);
  });

  async function build(project) {
    if (!project) return;
    md = await generateMarkdown(project);
    html = marked.parse(md);
  }

  async function exp(format, ext) {
    msg = "";
    try {
      const ok = await exportDoc(store.project, format, store.project.name + ext);
      if (ok) msg = `Esportato ${ext}`;
    } catch (e) {
      msg = String(e);
    }
  }
</script>

<div class="toolbar">
  <h1 style="margin:0">Documentazione</h1>
  <span style="flex:1"></span>
  <button class="btn secondary" onclick={() => exp("md", ".md")}>Markdown</button>
  <button class="btn secondary" onclick={() => exp("html", ".html")}>HTML</button>
  <button class="btn secondary" onclick={() => exp("wiki", ".wiki.md")}>Wiki</button>
  <button class="btn secondary" onclick={() => exp("pdf", ".pdf")}>PDF</button>
</div>
{#if msg}<p class="hint">{msg}</p>{/if}

<div class="markdown">{@html html}</div>
