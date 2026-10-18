/// <reference types="vitest/config" />
import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

const device = process.env.QGC_WEB_DEVICE?.replace(/^.*@/, '');
const aircastd = device && `http://${device}`;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: './',
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
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
        }
      : undefined,
  },
  preview: { port: 8800, strictPort: true },
  build: { target: 'es2023' },
  test: { environment: 'node', include: ['src/**/*.test.{ts,tsx}'] },
});
