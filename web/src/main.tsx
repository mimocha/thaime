/*
 * SPDX-License-Identifier: MPL-2.0
 */

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';

// Remove the static loading shell before React renders to avoid 2x viewport height flash
document.getElementById('loading-shell')?.remove();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
