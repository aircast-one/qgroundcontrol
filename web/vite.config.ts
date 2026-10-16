/// <reference types="vitest/config" />
import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
  worker: { format: 'es' },
  server: { port: 8800, strictPort: true },
  preview: { port: 8800, strictPort: true },
  build: { target: 'es2023' },
  test: { environment: 'node', include: ['src/**/*.test.{ts,tsx}'] },
});
