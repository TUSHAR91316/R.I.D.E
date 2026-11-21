import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    target: "esnext",
    minify: "esbuild",
    cssMinify: true,
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        manualChunks: {
          // Editor core only; language packs stay lazily-loaded chunks.
          codemirror: ['codemirror', '@codemirror/view', '@codemirror/state', '@codemirror/language', '@codemirror/commands'],
        },
      },
    },
  },
});
