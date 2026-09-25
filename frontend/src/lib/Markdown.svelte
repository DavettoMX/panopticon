<script lang="ts">
  import { Marked } from 'marked';
  import hljs from 'highlight.js';

  interface Props {
    content: string;
  }

  let { content }: Props = $props();

  const marked = new Marked({
    renderer: {
      code({ text, lang }: { text: string; lang?: string }) {
        const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext';
        const label = lang || '';
        const highlighted = hljs.highlight(text, { language }).value;
        const codeId = 'code-' + Math.random().toString(36).slice(2, 9);
        return `<div class="code-block"><div class="code-header"><span class="code-lang">${label}</span><button class="copy-btn" data-code-id="${codeId}">Copy</button></div><pre><code id="${codeId}" class="hljs language-${language}">${highlighted}</code></pre></div>`;
      },
      codespan({ text }: { text: string }) {
        return `<code class="inline-code">${text}</code>`;
      },
    },
  });

  function escapeAttr(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  function closeUnclosedCodeBlocks(text: string): string {
    // Count opening and closing triple backtick fences
    const fences = text.match(/^```/gm);
    if (fences && fences.length % 2 !== 0) {
      return text + '\n```';
    }
    return text;
  }

  const html = $derived(marked.parse(closeUnclosedCodeBlocks(content)) as string);

  function handleClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.classList.contains('copy-btn')) {
      const codeId = target.getAttribute('data-code-id');
      const codeEl = codeId ? document.getElementById(codeId) : null;
      if (codeEl) {
        navigator.clipboard.writeText(codeEl.textContent || '');
        target.textContent = 'Copied!';
        setTimeout(() => { target.textContent = 'Copy'; }, 2000);
      }
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="markdown-body" onclick={handleClick}>
  {@html html}
</div>

<style>
  .markdown-body {
    line-height: 1.6;
    word-break: break-word;
    overflow-wrap: anywhere;
    overflow: hidden;
  }

  .markdown-body :global(p) {
    margin: 0 0 8px 0;
  }

  .markdown-body :global(p:last-child) {
    margin-bottom: 0;
  }

  .markdown-body :global(ul),
  .markdown-body :global(ol) {
    margin: 4px 0 8px 0;
    padding-left: 20px;
  }

  .markdown-body :global(li) {
    margin-bottom: 2px;
  }

  .markdown-body :global(h1),
  .markdown-body :global(h2),
  .markdown-body :global(h3),
  .markdown-body :global(h4) {
    margin: 12px 0 6px 0;
    font-weight: 600;
    color: var(--text-bright);
  }

  .markdown-body :global(h1) { font-size: 1.3em; }
  .markdown-body :global(h2) { font-size: 1.15em; }
  .markdown-body :global(h3) { font-size: 1.05em; }

  .markdown-body :global(strong) {
    font-weight: 600;
    color: var(--text-bright);
  }

  .markdown-body :global(.inline-code) {
    background: rgba(255, 255, 255, 0.08);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 0.9em;
    color: #e2b3ff;
  }

  /* Code block container */
  .markdown-body :global(.code-block) {
    margin: 8px 0;
    border-radius: 8px;
    overflow: hidden;
    background: #1a1a2e;
    border: 1px solid var(--border);
  }

  .markdown-body :global(.code-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background: rgba(255, 255, 255, 0.04);
    border-bottom: 1px solid var(--border);
  }

  .markdown-body :global(.code-lang) {
    font-size: 12px;
    color: var(--text-muted);
    font-family: 'JetBrains Mono', monospace;
  }

  .markdown-body :global(.copy-btn) {
    font-size: 12px;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 8px;
    border-radius: 4px;
    transition: all 0.15s;
  }

  .markdown-body :global(.copy-btn:hover) {
    color: var(--text-bright);
    background: rgba(255, 255, 255, 0.08);
  }

  .markdown-body :global(pre) {
    margin: 0;
    padding: 12px 16px;
    overflow-x: auto;
  }

  .markdown-body :global(pre code) {
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 13px;
    line-height: 1.5;
    background: none;
    padding: 0;
  }

  /* highlight.js theme - dark colors matching the app */
  .markdown-body :global(.hljs) {
    color: #c9d1d9;
    background: transparent;
  }
  .markdown-body :global(.hljs-keyword) { color: #ff7b72; }
  .markdown-body :global(.hljs-string) { color: #a5d6ff; }
  .markdown-body :global(.hljs-number) { color: #79c0ff; }
  .markdown-body :global(.hljs-built_in) { color: #ffa657; }
  .markdown-body :global(.hljs-function) { color: #d2a8ff; }
  .markdown-body :global(.hljs-title) { color: #d2a8ff; }
  .markdown-body :global(.hljs-title.function_) { color: #d2a8ff; }
  .markdown-body :global(.hljs-title.class_) { color: #ffa657; }
  .markdown-body :global(.hljs-params) { color: #c9d1d9; }
  .markdown-body :global(.hljs-comment) { color: #8b949e; font-style: italic; }
  .markdown-body :global(.hljs-attr) { color: #79c0ff; }
  .markdown-body :global(.hljs-attribute) { color: #79c0ff; }
  .markdown-body :global(.hljs-property) { color: #79c0ff; }
  .markdown-body :global(.hljs-type) { color: #ffa657; }
  .markdown-body :global(.hljs-meta) { color: #79c0ff; }
  .markdown-body :global(.hljs-literal) { color: #79c0ff; }
  .markdown-body :global(.hljs-symbol) { color: #79c0ff; }
  .markdown-body :global(.hljs-variable) { color: #ffa657; }
  .markdown-body :global(.hljs-template-variable) { color: #ffa657; }
  .markdown-body :global(.hljs-selector-class) { color: #7ee787; }
  .markdown-body :global(.hljs-selector-id) { color: #7ee787; }
  .markdown-body :global(.hljs-selector-tag) { color: #7ee787; }
  .markdown-body :global(.hljs-tag) { color: #7ee787; }
  .markdown-body :global(.hljs-name) { color: #7ee787; }

  .markdown-body :global(blockquote) {
    margin: 8px 0;
    padding: 4px 12px;
    border-left: 3px solid var(--primary);
    color: var(--text-muted);
  }

  .markdown-body :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 12px 0;
  }

  .markdown-body :global(table) {
    border-collapse: collapse;
    margin: 8px 0;
    width: 100%;
  }

  .markdown-body :global(th),
  .markdown-body :global(td) {
    border: 1px solid var(--border);
    padding: 6px 10px;
    font-size: 13px;
    text-align: left;
  }

  .markdown-body :global(th) {
    background: rgba(255, 255, 255, 0.04);
    font-weight: 600;
    color: var(--text-bright);
  }
</style>
