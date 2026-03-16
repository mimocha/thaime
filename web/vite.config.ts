import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  // For GitHub Pages deployment under a subpath, set base accordingly.
  // e.g. base: '/thaime/' if hosted at https://<user>.github.io/thaime/
  base: './',
  build: {
    outDir: 'dist',
  },
  optimizeDeps: {
    exclude: ['thaime_wasm'],
  },
  server: {
    fs: {
      // Allow serving the .wasm binary from the wasm-pack output directory.
      // Vite blocks @fs access to files outside the project root by default.
      allow: ['.', path.resolve(__dirname, '../crates/thaime_wasm/pkg')],
    },
  },
});
