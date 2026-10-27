# Panopticon

App de escritorio para chatear con LLMs locales, inspirada en Open WebUI. Hoy funciona sobre **llama.cpp** (`llama-server`); el objetivo es **agregar inferencia con vLLM** para hacer la herramienta más potente.

Construida con **Tauri v2** (backend en Rust) y **Svelte 5 + TypeScript** (Vite).

## Funcionalidades

- **Chat local con streaming**: `llama-server` corre como subproceso y se comunica por su API REST compatible con OpenAI. Los tokens llegan a la UI por eventos de Tauri (`chat-token`, `chat-complete`, `chat-error`), con soporte para bloques de razonamiento ("thinking").
- **Descubrimiento de modelos**: escanea un directorio configurable en busca de archivos `.gguf` y lee sus metadatos directamente del encabezado binario, sin herramientas externas.
- **Centro de configuración de modelos**: clasificación SLM (<10B parámetros) / LLM (≥10B), etiquetas por tarea (coding, chat, vision…), detección de GPU con `nvidia-smi` y estimación de VRAM para cargar modelos de forma segura.
- **Modo Orquestador (multi-agente)**: cinco roles (Orchestrator, Architect, Auditor, Constructor, Judge) que se coordinan a través de un hub central. Las respuestas de cada agente se guardan como artefactos numerados, la ejecución se pausa en checkpoints para que intervengas, y el Constructor puede proponer escribir archivos en un directorio de trabajo (siempre con tu aprobación).
- **Persistencia local**: SQLite + `sqlite-vec` embebido para conversaciones, ajustes, etiquetas y sesiones del orquestador.

## Roadmap: inferencia con vLLM

llama.cpp es excelente para correr modelos GGUF cuantizados en una sola máquina, pero tiene límites cuando se necesita más rendimiento. El siguiente paso de Panopticon es **soportar vLLM como backend de inferencia**, junto a llama.cpp:

- **Más throughput y concurrencia**: PagedAttention y batching continuo, útiles sobre todo en el modo Orquestador, donde varios agentes generan respuestas.
- **Más formatos de modelo**: pesos de Hugging Face (safetensors), AWQ, GPTQ, FP8, etc.
- **Backend seleccionable**: elegir llama.cpp o vLLM por modelo o por agente, reutilizando la misma capa de API compatible con OpenAI.
- **Gestión del proceso de vLLM**: arranque, parada y monitoreo desde la app, igual que hoy con `llama-server`.

## Estructura del proyecto

```
frontend/               UI en Svelte/Vite
  src/lib/              Componentes y stores (Svelte 5 runes)
  src/lib/orchestrator/ UI del modo Orquestador
  src/test/             Capa de mocks de IPC de Tauri para Cypress
  cypress/              Tests E2E, fixtures y comandos
src-tauri/              Backend en Rust (Tauri v2)
  src/commands/         Handlers de comandos (chat, conversations, models, orchestrator, server, settings)
  src/db.rs             Esquema SQLite + CRUD
  src/gguf.rs           Parser de metadatos GGUF
  src/gpu.rs            Detección de GPU
  src/llama.rs          Ciclo de vida de llama-server
  src/orchestrator.rs   Estado, directivas y ciclo de vida del equipo multi-agente
```

## Requisitos

- Rust (stable), Node 22+, npm
- Dependencias de sistema de Tauri (en Fedora: `webkit2gtk4.1-devel javascriptcoregtk4.1-devel`)
- Un binario de [llama.cpp](https://github.com/ggml-org/llama.cpp) `llama-server` compilado (CUDA recomendado)
- Modelos en formato `.gguf`

Las rutas del binario de `llama-server` y del directorio de modelos se configuran desde la app.

## Uso

```bash
npm install && (cd frontend && npm install)

npm run tauri dev      # App completa en modo desarrollo
npm run tauri build    # Build de producción
```

## Tests

Los tests E2E corren con Cypress contra el frontend usando un mock de la IPC de Tauri, así que no necesitan el runtime nativo.

```bash
cd frontend
npm run dev:test       # Vite con los mocks de Tauri
npm run cy:open        # Cypress interactivo
npm run cy:run         # Cypress headless
```
