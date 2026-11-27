import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath } from 'url';

const websiteRoot = fileURLToPath(new URL('.', import.meta.url));
const latestManifestUrl =
  'https://github.com/RustyPiano/ClipLeaf/releases/latest/download/latest.json';
const latestManifest = await fetch(latestManifestUrl);
if (!latestManifest.ok) {
  throw new Error(`GET ${latestManifestUrl} returned ${latestManifest.status}`);
}
const { version: releaseVersion } = (await latestManifest.json()) as { version: string };

export default defineConfig({
  root: websiteRoot,
  plugins: [tailwindcss(), svelte()],
  define: {
    __RELEASE_VERSION__: JSON.stringify(releaseVersion),
  },
  resolve: {
    alias: {
      $site: fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    port: 4173,
    strictPort: false,
  },
  preview: {
    port: 4174,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: ['es2022', 'chrome117', 'safari17.4'],
  },
});
