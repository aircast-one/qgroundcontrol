import { defineConfig, devices } from '@playwright/test';

const port = Number(process.env.E2E_PORT ?? 8800);
const WT_ORIGIN = 'localhost:18443';
const WT_CERT_SPKI = 'Cm4S0tfKFG+7Z629ilyBsCcsJONuTgfn2fhtM4zirXU=';

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
    ignoreHTTPSErrors: true,
    launchOptions: { args: [`--origin-to-force-quic-on=${WT_ORIGIN}`, `--ignore-certificate-errors-spki-list=${WT_CERT_SPKI}`] },
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
      command: 'GOWORK=off go -C e2e/wt run .',
      url: 'https://localhost:18443/stats',
      ignoreHTTPSErrors: true,
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      command: 'node e2e/device/aircastd.ts',
      url: 'http://localhost:18780/stats',
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
