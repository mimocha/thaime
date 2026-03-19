/*
 * SPDX-License-Identifier: MPL-2.0
 */

import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import fs from 'fs';

// Read version from workspace Cargo.toml
function getCargoVersion(): string {
  const cargo = fs.readFileSync(path.resolve(__dirname, '../Cargo.toml'), 'utf-8');
  const match = cargo.match(/^version\s*=\s*"([^"]+)"/m);
  return match ? match[1] : 'unknown';
}

export default defineConfig({
  define: {
    __THAIME_VERSION__: JSON.stringify(getCargoVersion()),
  },
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
