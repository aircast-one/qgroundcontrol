import { connectMock, expect, telemetry, test } from './fixtures';
import type { Page } from '@playwright/test';

const openGeneral = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'General' }).click();
};

test('theme switches between dark and light and survives a reload', async ({ app }) => {
  await openGeneral(app);
  await expect(app.locator('html')).toHaveClass(/dark/);
  await app.getByRole('radio', { name: 'Light' }).click();
  await expect(app.locator('html')).toHaveClass(/light/);
  await app.reload();
  await expect(app.locator('html')).toHaveClass(/light/);
});

test('a switch setting keeps its value across a reload', async ({ app }) => {
  await openGeneral(app);
  const mute = app.getByRole('switch', { name: 'Mute all audio output' });
  await expect(mute).not.toBeChecked();
  await mute.click();
  await expect(mute).toBeChecked();
  await app.reload();
  await openGeneral(app);
  await expect(app.getByRole('switch', { name: 'Mute all audio output' })).toBeChecked();
});

test('the enforce checklist switch unlocks only with the checklist on', async ({ app }) => {
  await openGeneral(app);
  const enforce = app.getByRole('switch', { name: 'Enforce preflight checklist' });
  await expect(enforce).toBeDisabled();
  await app.getByRole('switch', { name: 'Use preflight checklist' }).click();
  await expect(enforce).toBeEnabled();
});

test('settings search finds a setting by name', async ({ app }) => {
  await app.getByRole('button', { name: 'Settings', exact: true }).click();
  await app.getByPlaceholder('Search settings').fill('mute');
  await expect(app.getByText('Mute all audio output')).toBeVisible();
});

test('turning on the ADS-B server connection keeps the core running', async ({ app }) => {
  await app.getByRole('button', { name: 'Settings', exact: true }).click();
  await app.getByPlaceholder('Search settings').fill('ADS-B');
  const adsb = app.getByRole('switch', { name: 'Show nearby aircraft (ADS-B)' });
  await adsb.click();
  await expect(adsb).toBeChecked();
  await app.getByRole('button', { name: 'Close settings' }).click();
  await connectMock(app);
});

test('turning on the on-screen sticks shows both sticks on the Fly screen', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Settings', exact: true }).click();
  await vehicle.getByRole('tablist').getByRole('tab', { name: 'General' }).click();
  const sticks = vehicle.getByRole('switch', { name: 'Enabled' });
  await sticks.click();
  await expect(sticks).toBeChecked();
  await expect(vehicle.getByRole('switch', { name: 'Left-handed mode (swap sticks)' })).toBeEnabled();
  await vehicle.getByRole('button', { name: 'Close settings' }).click();
  await expect(vehicle.getByLabel('Left virtual stick')).toBeVisible({ timeout: 60_000 });
  await expect(vehicle.getByLabel('Right virtual stick')).toBeVisible();
});

test('imperial units turn the Fly telemetry into feet', async ({ vehicle }) => {
  await expect(telemetry(vehicle, 'H')).toContainText('m');
  await openGeneral(vehicle);
  await vehicle.getByRole('button', { name: 'Imperial', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Imperial', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await vehicle.getByRole('button', { name: 'Close settings' }).click();
  await expect(telemetry(vehicle, 'H')).toContainText('ft');
});

test('the radio on USB settings sit with the other stick inputs and let the browser grant the radio its serial port', async ({ app }) => {
  await app.getByRole('button', { name: 'Settings', exact: true }).click();
  await app.getByRole('tablist').getByRole('tab', { name: 'Control' }).click();
  await expect(app.getByText('Radio on USB (EdgeTX script)', { exact: false })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Allow a USB port' })).toBeVisible();
});
