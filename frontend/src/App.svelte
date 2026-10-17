<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import ChatArea from './lib/ChatArea.svelte';
  import OrchestratorView from './lib/orchestrator/OrchestratorView.svelte';
  import {
    loadConversations,
    loadModels,
    loadModelTags,
    loadTrash,
    setupEventListeners,
  } from './lib/stores.svelte';
  import {
    orchStore,
    setupOrchestratorEvents,
    cleanupOrchestratorEvents,
  } from './lib/orchestrator-store.svelte';

  onMount(() => {
    loadConversations();
    loadModels();
    loadModelTags();
    loadTrash();
    setupEventListeners();
    setupOrchestratorEvents();
  });

  onDestroy(() => {
    cleanupOrchestratorEvents();
  });
</script>

<Sidebar />
{#if orchStore.mode === 'orchestrator'}
  <OrchestratorView />
{:else}
  <ChatArea />
{/if}
