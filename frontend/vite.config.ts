import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig, type Plugin } from 'vite'

function tauriMockPlugin(): Plugin {
  return {
    name: 'tauri-mock-inject',
    transformIndexHtml(html) {
      if (process.env.VITE_CYPRESS !== 'true') return html;
      return html.replace(
        '<script type="module" src="/src/main.ts"></script>',
        `<script type="module" src="/src/test/tauri-mock-bootstrap.ts"></script>
    <script type="module" src="/src/main.ts"></script>`,
      );
    },
  };
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [tauriMockPlugin(), svelte()],
})
