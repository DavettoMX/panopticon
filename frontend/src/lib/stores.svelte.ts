import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// --- Types ---

export interface Conversation {
  id: string;
  title: string;
  model: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: string;
  content: string;
  model: string | null;
  created_at: string;
}

export interface ModelInfo {
  filename: string;
  path: string;
  size_bytes: number;
  parameter_count: number;
}

export type ModelTag = 'coding' | 'chat' | 'vision' | 'reasoning' | 'creative' | 'translation' | 'summarization';

export const ALL_MODEL_TAGS: { value: ModelTag; label: string }[] = [
  { value: 'coding', label: 'Coding' },
  { value: 'chat', label: 'Chat' },
  { value: 'vision', label: 'Vision' },
  { value: 'reasoning', label: 'Reasoning' },
  { value: 'creative', label: 'Creative' },
  { value: 'translation', label: 'Translation' },
  { value: 'summarization', label: 'Summarization' },
];

export function formatModelDisplayName(filename: string): string {
  let name = filename.replace(/\.gguf$/i, '');

  // Remove quantization suffix (Q4_K_M, Q5_K_S, Q8_0, IQ4_XS, F16, BF16, etc.)
  name = name.replace(/[-._](I?Q\d+(?:_\w+)*|B?F\d+)$/i, '');

  // Extract parameter size (e.g., 4B, 27B, E4B, 0.5B)
  const sizeMatch = name.match(/[-._]([Ee]?\d+(?:\.\d+)?)[Bb]/);
  let size = '';
  if (sizeMatch) {
    size = sizeMatch[1].toUpperCase() + 'B';
    name = name.replace(/[-._][Ee]?\d+(?:\.\d+)?[Bb]/, '');
  }

  // Remove vendor prefixes (e.g., google_)
  name = name.replace(/^google[-_]/i, '');

  // Replace separators with spaces
  name = name.replace(/[-_]+/g, ' ').trim();

  // Deduplicate prefix: "Qwen Qwen3.5" → "Qwen3.5"
  const parts = name.split(' ');
  if (parts.length >= 2) {
    const first = parts[0].toLowerCase();
    if (parts[1].toLowerCase().startsWith(first)) {
      parts.splice(0, 1);
    }
  }
  name = parts.join(' ');

  // Add space between letters and digits: "Qwen3.5" → "Qwen 3.5"
  name = name.replace(/([a-zA-Z])(\d)/g, '$1 $2');

  // Capitalize each word
  name = name.split(' ').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');

  return size ? `${name}:${size}` : name;
}

export function formatQuantShort(filename: string): string {
  const match = filename.match(/(I?Q\d+|B?F\d+)(?:_\w+)*\.gguf$/i);
  return match ? match[1].toUpperCase() : '';
}

interface TokenPayload {
  conversation_id: string;
  content: string;
  is_reasoning: boolean;
}

interface GenerationStats {
  total_seconds: number;
  thinking_seconds: number;
  token_count: number;
  tokens_per_second: number;
}

interface ChatCompletePayload {
  conversation_id: string;
  message: Message;
  stats: GenerationStats;
}

// --- Reactive state (wrapped in object to allow export + mutation) ---

export const store = $state({
  conversations: [] as Conversation[],
  activeConversationId: null as string | null,
  messages: [] as Message[],
  models: [] as ModelInfo[],
  activeModel: null as ModelInfo | null,
  isGenerating: false,
  isThinking: false,
  streamingContent: '',
  serverStatus: 'stopped' as 'stopped' | 'loading' | 'ready',
  serverError: null as string | null,
  lastStats: null as GenerationStats | null,
  generationError: null as string | null,
  thinkingEnabled: true,
  trash: [] as Conversation[],
  showTrash: false,
  modelTags: {} as Record<string, string[]>,
});

// --- Backend calls ---

export async function loadConversations() {
  try {
    store.conversations = await invoke<Conversation[]>('list_conversations');
  } catch (e) {
    console.error('Failed to load conversations:', e);
  }
}

export async function loadMessages(conversationId: string) {
  try {
    store.activeConversationId = conversationId;
    store.messages = await invoke<Message[]>('get_messages', { conversationId });
  } catch (e) {
    console.error('Failed to load messages:', e);
  }
}

export async function loadModels() {
  try {
    store.models = await invoke<ModelInfo[]>('list_models');
  } catch (e) {
    console.error('Failed to load models:', e);
  }
}

export async function loadModelTags() {
  try {
    store.modelTags = await invoke<Record<string, string[]>>('get_all_model_tags') ?? {};
  } catch (e) {
    console.error('Failed to load model tags:', e);
  }
}

export async function setModelTags(modelPath: string, tags: string[]) {
  try {
    await invoke('set_model_tags', { modelPath, tags });
    store.modelTags = { ...store.modelTags, [modelPath]: tags };
  } catch (e) {
    console.error('Failed to set model tags:', e);
  }
}

export function getModelSizeClass(model: ModelInfo): 'SLM' | 'LLM' {
  return model.parameter_count > 0 && model.parameter_count < 10_000_000_000 ? 'SLM' : 'LLM';
}

export async function selectModel(model: ModelInfo) {
  store.serverStatus = 'loading';
  store.activeModel = model;
  store.serverError = null;
  try {
    await invoke('start_model', { modelPath: model.path });
    store.serverStatus = 'ready';
  } catch (e) {
    const errMsg = String(e);
    console.error('Failed to start model:', errMsg);
    store.serverStatus = 'stopped';
    store.serverError = errMsg;
    // Keep activeModel so user can see which one failed
  }
}

export async function newConversation(): Promise<Conversation | null> {
  try {
    const conv = await invoke<Conversation>('create_conversation', { title: null });
    store.conversations = [conv, ...store.conversations];
    store.activeConversationId = conv.id;
    store.messages = [];
    return conv;
  } catch (e) {
    console.error('Failed to create conversation:', e);
    return null;
  }
}

export async function deleteConversation(id: string) {
  try {
    await invoke('delete_conversation', { id });
    const conv = store.conversations.find(c => c.id === id);
    store.conversations = store.conversations.filter(c => c.id !== id);
    if (conv) {
      store.trash = [{ ...conv, deleted_at: new Date().toISOString() }, ...store.trash];
    }
    if (store.activeConversationId === id) {
      store.activeConversationId = null;
      store.messages = [];
    }
  } catch (e) {
    console.error('Failed to delete conversation:', e);
  }
}

export async function loadTrash() {
  try {
    store.trash = await invoke<Conversation[]>('list_trash');
  } catch (e) {
    console.error('Failed to load trash:', e);
  }
}

export async function restoreConversation(id: string) {
  try {
    await invoke('restore_conversation', { id });
    const conv = store.trash.find(c => c.id === id);
    store.trash = store.trash.filter(c => c.id !== id);
    if (conv) {
      store.conversations = [{ ...conv, deleted_at: null }, ...store.conversations];
    }
  } catch (e) {
    console.error('Failed to restore conversation:', e);
  }
}

export async function permanentDeleteConversation(id: string) {
  try {
    await invoke('permanent_delete_conversation', { id });
    store.trash = store.trash.filter(c => c.id !== id);
  } catch (e) {
    console.error('Failed to permanently delete conversation:', e);
  }
}

export async function sendMessage(content: string) {
  if (!store.activeConversationId || !content.trim()) return;

  store.isGenerating = true;
  store.isThinking = false;
  store.streamingContent = '';
  store.generationError = null;

  // Optimistically add user message to UI
  const userMsg: Message = {
    id: crypto.randomUUID(),
    conversation_id: store.activeConversationId,
    role: 'user',
    content: content.trim(),
    model: null,
    created_at: new Date().toISOString(),
  };
  store.messages = [...store.messages, userMsg];

  try {
    const assistantMsg = await invoke<Message>('send_message', {
      conversationId: store.activeConversationId,
      content: content.trim(),
      thinking: store.thinkingEnabled,
    });

    // Replace streaming message with final message (skip empty)
    store.messages = store.messages.filter(m => m.id !== '__streaming__');
    if (assistantMsg.content) {
      store.messages = [...store.messages, assistantMsg];
    }
    store.streamingContent = '';

    // Auto-title from first user message
    if (store.messages.filter(m => m.role === 'user').length === 1) {
      const title = content.trim().slice(0, 50) + (content.trim().length > 50 ? '...' : '');
      try {
        await invoke('rename_conversation', { id: store.activeConversationId, title });
        store.conversations = store.conversations.map(c =>
          c.id === store.activeConversationId ? { ...c, title } : c
        );
      } catch (e) {
        console.error('Failed to rename conversation:', e);
      }
    }
  } catch (e) {
    console.error('Failed to send message:', e);
    store.messages = store.messages.filter(m => m.id !== '__streaming__');
  } finally {
    store.isGenerating = false;
    store.isThinking = false;
    store.streamingContent = '';
  }
}

export async function stopGeneration() {
  try {
    await invoke('stop_generation');
  } catch (e) {
    console.error('Failed to stop generation:', e);
  }
}

// --- Event listeners ---

export function setupEventListeners() {
  listen<TokenPayload>('chat-token', (event) => {
    const { conversation_id, content, is_reasoning } = event.payload;
    if (conversation_id !== store.activeConversationId) return;

    if (is_reasoning) {
      // Model is thinking — track state but don't show content
      store.isThinking = true;
      return;
    }

    // Content token — model finished thinking
    store.isThinking = false;
    store.streamingContent += content;

    // Add or update streaming message
    const hasStreaming = store.messages.some(m => m.id === '__streaming__');
    if (!hasStreaming) {
      store.messages = [...store.messages, {
        id: '__streaming__',
        conversation_id,
        role: 'assistant',
        content: store.streamingContent,
        model: null,
        created_at: new Date().toISOString(),
      }];
    } else {
      store.messages = store.messages.map(m =>
        m.id === '__streaming__' ? { ...m, content: store.streamingContent } : m
      );
    }
  });

  listen<ChatCompletePayload>('chat-complete', (event) => {
    store.lastStats = event.payload.stats;
  });

  listen<{ conversation_id: string; error: string }>('chat-error', (event) => {
    const { conversation_id, error } = event.payload;
    if (conversation_id !== store.activeConversationId) return;
    store.generationError = error;
    store.isGenerating = false;
    store.isThinking = false;
    // Remove streaming message
    store.messages = store.messages.filter(m => m.id !== '__streaming__');
    store.streamingContent = '';
  });
}
