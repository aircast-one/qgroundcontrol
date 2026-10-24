import { expect, openApp, test } from './fixtures';
import { plugInR2d2Radio } from './r2d2';

const RADIO_PORT = /0483:5740/;
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
  await vehicle.getByRole('button', { name: /^Battery \d+%/ }).click();
  const sheet = vehicle.getByRole('dialog');
  await expect(sheet.getByText(/^Battery 1/)).toBeVisible();
  await expect(sheet.getByText(/\d+\.\d\d V/).first()).toBeVisible();
  await expect(sheet.getByText('Battery failsafes')).toBeVisible();
});

test('a MAVLink WebSocket link connects a vehicle and hears the ground station', async ({ app, request }) => {
  const before = ((await (await request.get('http://localhost:18760/stats')).json()) as { gcsHeartbeats: number }).gcsHeartbeats;
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add link…' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('ws://localhost:18760');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await expect(app.getByText('20.0m', { exact: true })).toBeVisible();
  await expect.poll(async () => ((await (await request.get('http://localhost:18760/stats')).json()) as { gcsHeartbeats: number }).gcsHeartbeats, { timeout: 15_000 }).toBeGreaterThan(before);
});

test('a WebSocket link is saved and reconnects by itself after the page reloads', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add link…' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('ws://localhost:18760');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await app.reload();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await app.getByRole('button', { name: 'Settings', exact: true }).click();
  await app.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  await app.getByRole('button', { name: /^Links/ }).first().click();
  await expect(app.getByText('ws://localhost:18760', { exact: false }).first()).toBeVisible();
});

test('a WebSocket address that is not ws:// is refused, and one that does not answer says so', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add link…' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('http://localhost:18760');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByText('A MAVLink WebSocket address starts with ws:// or wss://.')).toBeVisible();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('ws://localhost:1');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByText('No MAVLink WebSocket answered at ws://localhost:1')).toBeVisible({ timeout: 15_000 });
});

test('the USB radio form lets the browser grant a serial port', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add link…' }).click();
  await app.getByRole('button', { name: /^USB telemetry radio/ }).click();
  await expect(app.getByText('Nothing is plugged in. Connect a radio over USB and it will appear here.')).toBeVisible();
  await expect(app.getByRole('button', { name: 'Allow a USB port' })).toBeVisible();
});

test('a browser that cannot reach USB says so once, instead of claiming nothing is plugged in', async ({ page }) => {
  await page.addInitScript(() => delete (Navigator.prototype as { serial?: unknown }).serial);
  await openApp(page);
  await page.getByRole('button', { name: /How to connect/ }).click();
  await page.getByRole('button', { name: 'Add link…' }).click();
  await page.getByRole('button', { name: /^USB telemetry radio/ }).click();
  await expect(page.getByText('This browser cannot reach USB devices. Use Chrome or Edge, or connect over a WebSocket.')).toBeVisible();
  await expect(page.getByText('Nothing is plugged in', { exact: false })).toBeHidden();
  await expect(page.getByRole('button', { name: 'Connect', exact: true })).toBeDisabled();
});

test('the RC transmitter entry hands a plugged-in EdgeTX radio to the stick input', async ({ page }) => {
  await page.addLocatorHandler(page.getByText('An EdgeTX radio is streaming its sticks on USB. Fly with it?'), () => page.getByRole('button', { name: 'Not now', exact: true }).click());
  await plugInR2d2Radio(page);
  await openApp(page);
  await page.getByRole('button', { name: /How to connect/ }).click();
  await page.getByRole('button', { name: 'Add link…' }).click();
  await page.getByRole('button', { name: /^RC transmitter \(EdgeTX script\)/ }).click();
  await page.getByRole('button', { name: RADIO_PORT }).click();
  await expect(page.getByText('Change the port under Control > Radio on USB.', { exact: false })).toBeVisible();
  await expect(page.getByRole('button', { name: RADIO_PORT })).toContainText('Sends the sticks');
});
