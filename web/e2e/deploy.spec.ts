import { expect, test } from '@playwright/test';
import { openApp } from './fixtures';

test('a page that loads while a new build replaces the old one reloads itself instead of staying blank', async ({ page }) => {
  const asked = { core: 0 };
  await page.route('**/groundstation_bg*.wasm', (route) => (asked.core++ === 0 ? route.abort() : route.continue()));
  await openApp(page);
  await expect(page.getByRole('button', { name: /Connecting|No vehicle|Not connected/ }).first()).toBeVisible();
  expect(asked.core).toBe(2);
});
