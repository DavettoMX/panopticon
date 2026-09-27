<script lang="ts">
  import { Send, Plus, Settings, Sparkles, Square, Brain, ChevronDown, Circle, Check } from '@lucide/svelte';
  import { tick } from 'svelte';
  import {
    store,
    sendMessage,
    stopGeneration,
    newConversation,
    selectModel,
    type ModelInfo,
  } from './stores.svelte';
  import Markdown from './Markdown.svelte';
  import ModelConfigPanel from './ModelConfigPanel.svelte';

  let inputText = $state('');
  let messagesContainer: HTMLDivElement | undefined = $state();
  let showModelDropdown = $state(false);
  let configModel = $state<ModelInfo | null>(null);

  function autoResize(e: Event) {
    const textarea = e.target as HTMLTextAreaElement;
    textarea.style.height = 'auto';
    textarea.style.height = `${textarea.scrollHeight}px`;
  }

  function handleModelSelect(model: ModelInfo) {
    showModelDropdown = false;
    configModel = model;
  }

  const statusColor = $derived(
    store.serverStatus === 'ready' ? '#4ade80' :
    store.serverStatus === 'loading' ? '#facc15' :
    '#6b7280'
  );

  const suggestions = [
    { title: 'Help me study', subtitle: 'vocabulary for a college entrance exam' },
    { title: 'Give me ideas', subtitle: 'for what to do with my kids\u2019 art' },
    { title: 'Overcome procrastination', subtitle: 'give me tips' },
  ];

  async function handleSend() {
    if (!inputText.trim() || store.isGenerating) return;
    if (store.serverStatus !== 'ready') return;

    if (!store.activeConversationId) {
      const conv = await newConversation();
      if (!conv) return;
    }

    const text = inputText;
    inputText = '';
    // Reset textarea height after clearing
    document.querySelectorAll<HTMLTextAreaElement>('.message-input textarea').forEach(el => {
      el.style.height = 'auto';
    });
    await sendMessage(text);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  async function handleSuggestion(title: string, subtitle: string) {
    const text = `${title} ${subtitle}`;
    if (!store.activeConversationId) {
      const conv = await newConversation();
      if (!conv) return;
    }
    await sendMessage(text);
  }

  // Auto-scroll on new messages
  $effect(() => {
    const _ = store.messages.length;
    const __ = store.messages.at(-1)?.content;
    tick().then(() => {
      if (messagesContainer) {
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
      }
    });
  });

  const canSend = $derived(
    inputText.trim().length > 0 && store.serverStatus === 'ready' && !store.isGenerating
  );

  const inputPlaceholder = $derived(
    store.serverStatus === 'loading' ? 'Loading model...' :
    store.serverStatus === 'stopped' ? 'Select a model to start chatting' :
    'How can I help you today?'
  );

  const modelDisplayName = $derived(
    store.activeModel ? store.activeModel.filename.replace('.gguf', '') : 'No model selected'
  );

  function formatDuration(seconds: number): string {
    if (seconds < 60) return `${Math.round(seconds)}s`;
    const m = Math.floor(seconds / 60);
    const s = Math.round(seconds % 60);
    return s > 0 ? `${m}m ${s}s` : `${m}m`;
  }

  function formatTokenCount(count: number): string {
    if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
    return String(count);
  }
</script>

<main class="chat-area" data-cy="chat-area">
  <header class="chat-header">
    <div class="header-model-name">
      {#if store.activeConversationId}
        {modelDisplayName}
      {/if}
    </div>
    <div class="header-actions">
      <button class="header-btn">
        <Settings size={18} />
      </button>
    </div>
  </header>

  {#if !store.activeConversationId || store.messages.length === 0}
    <!-- Welcome / empty state -->
    <div class="chat-content">
      <div class="welcome" data-cy="welcome-screen">
        <div class="model-icon">P</div>
        <h1 class="welcome-model-name">{modelDisplayName}</h1>
      </div>

      <div class="input-wrapper">
        <div class="message-input">
          <button class="input-action">
            <Plus size={18} />
          </button>
          <textarea
            rows={1}
            placeholder={inputPlaceholder}
            bind:value={inputText}
            onkeydown={handleKeydown}
            oninput={autoResize}
            disabled={store.isGenerating}
            data-cy="message-input"
          ></textarea>
          <button
            class="thinking-toggle"
            class:active={store.thinkingEnabled}
            onclick={() => store.thinkingEnabled = !store.thinkingEnabled}
            title={store.thinkingEnabled ? 'Thinking enabled' : 'Thinking disabled'}
            data-cy="thinking-toggle"
          >
            <Brain size={16} />
          </button>
          <button
            class="send-btn"
            onclick={handleSend}
            disabled={!canSend}
            data-cy="send-btn"
          >
            <Send size={18} />
          </button>
        </div>
        <div class="input-footer">
          <div class="model-selector">
            <button class="model-pill" onclick={() => showModelDropdown = !showModelDropdown} data-cy="model-pill">
              <Circle size={6} fill={statusColor} color={statusColor} />
              <span class="model-pill-name">
                {store.activeModel ? store.activeModel.filename.replace('.gguf', '') : 'Select model'}
              </span>
              <ChevronDown size={12} />
            </button>
            {#if showModelDropdown}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="model-backdrop" onclick={() => showModelDropdown = false}></div>
              <div class="model-dropdown" data-cy="model-dropdown">
                {#each store.models as model}
                  <button
                    class="model-option"
                    class:active={store.activeModel?.path === model.path}
                    onclick={() => handleModelSelect(model)}
                    data-cy="model-option"
                  >
                    <div class="model-option-info">
                      <span class="model-option-name">{model.filename.replace('.gguf', '')}</span>
                      <span class="model-option-size">{(model.size_bytes / 1e9).toFixed(1)} GB</span>
                    </div>
                    {#if store.activeModel?.path === model.path}
                      <Check size={14} />
                    {/if}
                  </button>
                {/each}
                {#if store.models.length === 0}
                  <div class="model-option disabled" data-cy="model-empty">No models found</div>
                {/if}
              </div>
            {/if}
          </div>
          {#if store.serverError}
            <div class="server-error" data-cy="server-error">{store.serverError}</div>
          {/if}
        </div>
      </div>

      <div class="suggestions">
        <div class="suggestions-header">
          <Sparkles size={14} />
          <span>Suggested</span>
        </div>
        {#each suggestions as suggestion}
          <button
            class="suggestion-item"
            onclick={() => handleSuggestion(suggestion.title, suggestion.subtitle)}
            disabled={store.isGenerating}
            data-cy="suggestion-item"
          >
            <span class="suggestion-title">{suggestion.title}</span>
            <span class="suggestion-subtitle">{suggestion.subtitle}</span>
          </button>
        {/each}
      </div>
    </div>
  {:else}
    <!-- Active conversation -->
    <div class="messages-area" bind:this={messagesContainer} data-cy="messages-area">
      {#each store.messages as msg, i}
        <div class="message message-{msg.role}">
          {#if msg.role === 'assistant'}
            <div class="message-avatar">P</div>
          {/if}
          <div class="message-bubble {msg.role}" data-cy="message-{msg.role}">
            <div class="message-content">
              {#if msg.role === 'assistant'}
                {#if msg.id === '__streaming__' && store.isGenerating && (store.isThinking || !msg.content)}
                  <span class="thinking" data-cy="thinking-indicator"><span class="thinking-dots"></span> Thinking...</span>
                  {#if msg.content}
                    <Markdown content={msg.content} />
                  {/if}
                {:else}
                  <Markdown content={msg.content} />{#if msg.id === '__streaming__' && store.isGenerating}<span class="cursor">▊</span>{/if}
                {/if}
              {:else}
                {msg.content}
              {/if}
            </div>
            {#if msg.role === 'assistant' && msg.id !== '__streaming__' && !store.isGenerating && store.lastStats && i === store.messages.length - 1}
              <div class="generation-stats" data-cy="generation-stats">
                {formatDuration(store.lastStats.total_seconds)}
                · ↑ {formatTokenCount(store.lastStats.token_count)} tokens
                · {store.lastStats.tokens_per_second.toFixed(1)} t/s
                {#if store.lastStats.thinking_seconds > 0}
                  · thought for {formatDuration(store.lastStats.thinking_seconds)}
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {/each}
      {#if store.generationError}
        <div class="message message-assistant">
          <div class="message-avatar">P</div>
          <div class="message-bubble assistant error-bubble" data-cy="generation-error">
            <div class="message-content error-content">
              {store.generationError}
            </div>
          </div>
        </div>
      {/if}
      {#if store.isGenerating && !store.messages.some(m => m.id === '__streaming__')}
        <div class="message message-assistant">
          <div class="message-avatar">P</div>
          <div class="message-bubble assistant">
            <div class="message-content">
              <span class="thinking" data-cy="thinking-indicator"><span class="thinking-dots"></span> {store.isThinking ? 'Thinking...' : 'Waiting...'}</span>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <div class="chat-input-bar">
      <div class="message-input">
        <button class="input-action">
          <Plus size={18} />
        </button>
        <textarea
          rows={1}
          placeholder={store.serverStatus === 'ready' ? 'Type a message...' : inputPlaceholder}
          bind:value={inputText}
          onkeydown={handleKeydown}
          oninput={autoResize}
          disabled={store.isGenerating}
          data-cy="message-input"
        ></textarea>
        <button
          class="thinking-toggle"
          class:active={store.thinkingEnabled}
          onclick={() => store.thinkingEnabled = !store.thinkingEnabled}
          title={store.thinkingEnabled ? 'Thinking enabled' : 'Thinking disabled'}
          disabled={store.isGenerating}
          data-cy="thinking-toggle"
        >
          <Brain size={16} />
        </button>
        {#if store.isGenerating}
          <button class="stop-btn" onclick={stopGeneration} data-cy="stop-btn">
            <Square size={16} />
          </button>
        {:else}
          <button
            class="send-btn"
            onclick={handleSend}
            disabled={!canSend}
            data-cy="send-btn"
          >
            <Send size={18} />
          </button>
        {/if}
      </div>
      <div class="input-footer">
        <div class="model-selector">
          <button class="model-pill" onclick={() => showModelDropdown = !showModelDropdown} data-cy="model-pill">
            <Circle size={6} fill={statusColor} color={statusColor} />
            <span class="model-pill-name">
              {store.activeModel ? store.activeModel.filename.replace('.gguf', '') : 'Select model'}
            </span>
            <ChevronDown size={12} />
          </button>
          {#if showModelDropdown}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="model-backdrop" onclick={() => showModelDropdown = false}></div>
            <div class="model-dropdown" data-cy="model-dropdown">
              {#each store.models as model}
                <button
                  class="model-option"
                  class:active={store.activeModel?.path === model.path}
                  onclick={() => handleModelSelect(model)}
                  data-cy="model-option"
                >
                  <div class="model-option-info">
                    <span class="model-option-name">{model.filename.replace('.gguf', '')}</span>
                    <span class="model-option-size">{(model.size_bytes / 1e9).toFixed(1)} GB</span>
                  </div>
                  {#if store.activeModel?.path === model.path}
                    <Check size={14} />
                  {/if}
                </button>
              {/each}
              {#if store.models.length === 0}
                <div class="model-option disabled" data-cy="model-empty">No models found</div>
              {/if}
            </div>
          {/if}
        </div>
        {#if store.serverError}
          <div class="server-error" data-cy="server-error">{store.serverError}</div>
        {/if}
      </div>
    </div>
  {/if}
</main>

{#if configModel}
  <ModelConfigPanel
    model={configModel}
    onclose={() => configModel = null}
  />
{/if}

<style>
  .chat-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    min-height: 52px;
  }

  .header-model-name {
    font-size: 14px;
    color: var(--text-muted);
  }

  .header-actions {
    display: flex;
    gap: 4px;
  }

  .header-btn {
    padding: 6px;
    border-radius: 6px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }

  .header-btn:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .chat-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 0 20px;
    gap: 24px;
    max-width: 640px;
    margin: 0 auto;
    width: 100%;
  }

  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .model-icon {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--primary);
    color: var(--text-bright);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
    font-weight: 700;
  }

  .welcome-model-name {
    font-size: 22px;
    font-weight: 500;
    color: var(--text-bright);
  }

  .input-wrapper {
    width: 100%;
  }

  .message-input {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    padding: 10px 14px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 12px;
    transition: border-color 0.15s;
  }

  .message-input:focus-within {
    border-color: var(--primary);
  }

  .message-input textarea {
    flex: 1;
    resize: none;
    font-size: 14px;
    line-height: 1.5;
    padding: 4px 0;
    min-height: 24px;
    max-height: 50vh;
    overflow-y: auto;
    background: transparent;
    transition: height 0.1s ease;
  }

  .message-input textarea::placeholder {
    color: var(--text-muted);
  }

  .message-input textarea:disabled {
    opacity: 0.5;
  }

  .input-action {
    padding: 4px;
    border-radius: 6px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }

  .input-action:hover {
    color: var(--text-bright);
  }

  .thinking-toggle {
    padding: 4px;
    border-radius: 6px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    cursor: pointer;
    transition: color 0.15s, background 0.15s;
  }

  .thinking-toggle.active {
    color: var(--primary);
    background: rgba(139, 92, 246, 0.15);
  }

  .thinking-toggle:hover:not(:disabled) {
    color: var(--text-bright);
    background: var(--bg-hover);
  }

  .thinking-toggle:disabled {
    cursor: not-allowed;
    opacity: 0.4;
  }

  .send-btn {
    padding: 6px;
    border-radius: 8px;
    background: var(--primary);
    color: var(--text-bright);
    display: flex;
    align-items: center;
    transition: background 0.15s;
  }

  .send-btn:hover:not(:disabled) {
    background: var(--primary-light);
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .stop-btn {
    padding: 6px;
    border-radius: 8px;
    background: #dc2626;
    color: var(--text-bright);
    display: flex;
    align-items: center;
    transition: background 0.15s;
  }

  .stop-btn:hover {
    background: #ef4444;
  }

  .suggestions {
    width: 100%;
  }

  .suggestions-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 8px;
  }

  .suggestion-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    padding: 12px 16px;
    border-radius: 10px;
    margin-bottom: 4px;
    text-align: left;
    transition: background 0.1s;
  }

  .suggestion-item:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .suggestion-item:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .suggestion-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-bright);
  }

  .suggestion-subtitle {
    font-size: 13px;
    color: var(--text-muted);
  }

  .messages-area {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .message {
    display: flex;
    gap: 12px;
    max-width: 80%;
    min-width: 0;
  }

  .message-user {
    align-self: flex-end;
    flex-direction: row-reverse;
  }

  .message-assistant {
    align-self: flex-start;
  }

  .message-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--primary);
    color: var(--text-bright);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    font-weight: 600;
    flex-shrink: 0;
  }

  .message-bubble {
    padding: 10px 14px;
    border-radius: 12px;
    font-size: 14px;
    line-height: 1.6;
    min-width: 0;
    overflow: hidden;
  }

  .message-bubble.user {
    background: var(--primary);
    color: var(--text-bright);
    border-bottom-right-radius: 4px;
  }

  .message-bubble.assistant {
    background: var(--bg-input);
    color: var(--text-bright);
    border-bottom-left-radius: 4px;
  }

  .message-content {
    word-break: break-word;
    overflow-wrap: anywhere;
  }

  .message-bubble.user .message-content {
    white-space: pre-wrap;
  }

  .cursor {
    animation: blink 0.7s infinite;
    color: var(--text-muted);
  }

  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }

  .thinking {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 13px;
    font-style: italic;
  }

  .thinking-dots {
    display: inline-flex;
    gap: 3px;
  }

  .thinking-dots::before,
  .thinking-dots::after,
  .thinking-dots {
    content: '';
  }

  .thinking-dots::before {
    content: '•••';
    animation: pulse 1.4s ease-in-out infinite;
    letter-spacing: 2px;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.3; }
    50% { opacity: 1; }
  }

  .error-bubble {
    border: 1px solid #dc2626;
    background: rgba(220, 38, 38, 0.1);
  }

  .error-content {
    color: #f87171;
    font-size: 13px;
  }

  .generation-stats {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 6px;
    opacity: 0.7;
  }

  .chat-input-bar {
    padding: 12px 20px 20px;
    max-width: 800px;
    margin: 0 auto;
    width: 100%;
  }

  .input-footer {
    display: flex;
    align-items: center;
    padding: 6px 4px 0;
  }

  .model-selector {
    position: relative;
  }

  .model-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-muted);
    cursor: pointer;
    transition: color 0.15s, background 0.15s;
  }

  .model-pill:hover {
    color: var(--text-bright);
    background: var(--bg-hover);
  }

  .model-pill-name {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-backdrop {
    position: fixed;
    inset: 0;
    z-index: 99;
  }

  .model-dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    min-width: 280px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 10px;
    margin-bottom: 6px;
    z-index: 100;
    overflow: hidden;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  .model-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 10px 14px;
    font-size: 13px;
    color: var(--text);
    text-align: left;
    transition: background 0.1s;
  }

  .model-option:hover {
    background: var(--bg-hover);
    color: var(--text-bright);
  }

  .model-option.active {
    color: var(--text-bright);
  }

  .model-option.disabled {
    color: var(--text-muted);
    cursor: default;
    padding: 14px;
    text-align: center;
    justify-content: center;
  }

  .model-option-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .model-option-name {
    font-weight: 500;
    color: var(--text-bright);
  }

  .model-option-size {
    font-size: 11px;
    color: var(--text-muted);
  }

  .server-error {
    font-size: 11px;
    color: #f87171;
    padding: 4px 0 0;
  }
</style>
