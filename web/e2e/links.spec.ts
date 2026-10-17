import { expect, test } from './fixtures';
import type { Page } from '@playwright/test';

const openLinks = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  const row = page.getByRole('button', { name: /^Links / });
  await expect(async () => {
    if (await row.isVisible()) await row.click();
    await expect(page.getByRole('button', { name: 'Add link…' })).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
};

test('the links page shows the simulated vehicle and stopping it disconnects', async ({ vehicle }) => {
  await openLinks(vehicle);
  await expect(vehicle.getByRole('button', { name: /^Links / })).toBeHidden();
  await vehicle.getByRole('button', { name: 'Stop', exact: true }).click();
  await vehicle.getByRole('button', { name: 'Close settings' }).click();
  await expect(vehicle.getByRole('button', { name: /Not connected|How to connect/ })).toBeVisible({ timeout: 30_000 });
});

test('the links page offers every link type the browser can open', async ({ vehicle }) => {
  await openLinks(vehicle);
  await vehicle.getByRole('button', { name: 'Add link…' }).click();
  await expect(vehicle.getByRole('button', { name: /^Simulated vehicle/ })).toBeVisible();
  await expect(vehicle.getByRole('button', { name: /^Replay a flight log/ })).toBeVisible();
});

test('the battery sheet shows each battery with its voltage', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: /^Battery \d+%$/ }).click();
  const sheet = vehicle.getByRole('dialog');
  await expect(sheet.getByText(/^Battery 1/)).toBeVisible();
  await expect(sheet.getByText(/\d+\.\d\d V/).first()).toBeVisible();
  await expect(sheet.getByText('Battery failsafes')).toBeVisible();
});
