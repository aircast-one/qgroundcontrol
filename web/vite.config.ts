/// <reference types="vitest/config" />
import { existsSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { FULL_ICON_FONT, ICON_FONT_SUBSET, iconFont } from './scripts/iconFont';

const device = process.env.QGC_WEB_DEVICE?.replace(/^.*@/, '');
const aircastd = device && `http://${device}`;
const root = fileURLToPath(new URL('.', import.meta.url));
const core = join(root, 'src/core/pkg/groundstation_bg.wasm');

export default defineConfig(({ command }) => ({
  plugins: [react(), tailwindcss(), iconFont(root)],
  base: './',
  define: { 'import.meta.env.CORE_BYTES': existsSync(core) ? statSync(core).size : 0 },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@icon-font': join(root, dirname(command === 'build' ? ICON_FONT_SUBSET : FULL_ICON_FONT)),
    },
  },
  worker: { format: 'es' },
  server: {
    port: aircastd ? 8801 : 8800,
    strictPort: true,
    fs: { allow: [fileURLToPath(new URL('..', import.meta.url))] },
    proxy: aircastd
      ? {
          '^/(device\\.json|mavlink|socket|files|proxy)\\b': {
            target: aircastd,
            ws: true,
            changeOrigin: true,
            rewrite: (path) => `/fly${path}`,
            configure: (proxy) => proxy.on('proxyReqWs', (request) => request.setHeader('origin', aircastd)),
          },
          '^/api/': { target: aircastd, changeOrigin: true },
        }
      : undefined,
  },
  preview: { port: 8800, strictPort: true },
  build: { target: 'es2023' },
  test: { environment: 'node', include: ['src/**/*.test.{ts,tsx}', 'scripts/**/*.test.ts'] },
}));
