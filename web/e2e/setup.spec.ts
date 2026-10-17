import { connectMock, expect, test } from './fixtures';
import type { Page } from '@playwright/test';

test.describe.configure({ timeout: 120_000 });

const openAircraftSetup = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'General' }).click();
  await expect(async () => {
    await page.getByRole('button', { name: 'Aircraft setup' }).click();
    await expect(page.getByRole('button', { name: /^Sensors/ })).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 60_000 });
};

test('ArduPilot setup names the frame and failsafe actions from the parameter metadata', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await expect(app.getByRole('button', { name: /^Frame Quad/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Failsafes .*RTL/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Flight modes .*Stabilize/ })).toBeVisible();
});

test('PX4 setup lists sensors, radio and parameters for the vehicle', async ({ app }) => {
  await connectMock(app);
  await openAircraftSetup(app);
  await expect(app.getByRole('button', { name: /^Sensors/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Radio/ })).toBeVisible();
});
