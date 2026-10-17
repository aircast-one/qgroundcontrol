import { defineConfig, devices } from '@playwright/test';

const port = Number(process.env.E2E_PORT ?? 8800);

export default defineConfig({
  testDir: 'e2e',
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: true,
  workers: 4,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: `http://localhost:${port}`,
    trace: 'retain-on-failure',
    viewport: { width: 1280, height: 800 },
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 800 } } }],
  webServer: [
    {
      command: `npx vite --port ${port} --strictPort`,
      url: `http://localhost:${port}`,
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      command: 'mediamtx e2e/whep/mediamtx.yml',
      url: 'http://localhost:18889/e2e/',
      reuseExistingServer: true,
      timeout: 30_000,
    },
    {
      command: 'node e2e/ws/mavlink-ws.ts',
      url: 'http://localhost:18760/',
      reuseExistingServer: true,
      timeout: 30_000,
    },
  ],
});
