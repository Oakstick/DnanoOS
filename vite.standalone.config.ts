import { defineConfig } from 'vite';
import { viteSingleFile } from 'vite-plugin-singlefile';

/**
 * Builds the whole game into one HTML file that runs on a double-click, with
 * no server behind it.
 *
 * The usual build cannot do this: a browser refuses to fetch a module script
 * over `file://`, so `dist/index.html` only works when something is serving
 * it. Inlining sidesteps that, because an inline module never gets fetched at
 * all. Everything therefore has to end up inside the one file: no code
 * splitting, no separate stylesheet, and an inline limit large enough that no
 * asset is left behind as a URL.
 *
 *   npm run build:standalone   ->   standalone/void-harvest.html
 */
export default defineConfig({
  base: './',
  plugins: [viteSingleFile()],
  build: {
    target: 'es2022',
    outDir: 'standalone',
    emptyOutDir: true,
    sourcemap: false,
    cssCodeSplit: false,
    assetsInlineLimit: 100_000_000,
    chunkSizeWarningLimit: 4000,
    rollupOptions: { output: { inlineDynamicImports: true } },
  },
});
