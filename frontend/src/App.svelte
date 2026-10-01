<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import ChatArea from './lib/ChatArea.svelte';
  import {
    loadConversations,
    loadModels,
    loadModelTags,
    loadTrash,
    setupEventListeners,
  } from './lib/stores.svelte';

  onMount(() => {
    loadConversations();
    loadModels();
    loadModelTags();
    loadTrash();
    setupEventListeners();
  });
</script>

<Sidebar />
<ChatArea />
