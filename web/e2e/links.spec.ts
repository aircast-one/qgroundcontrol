import { closeSettings, connectMock, expect, openApp, openMenu, test } from './fixtures';
import { plugInR2d2Radio } from './r2d2';

const RADIO_PORT = /0483:5740/;
import type { Page } from '@playwright/test';

const openLinks = async (page: Page) => {
  await openMenu(page, 'Settings');
  await page.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  const row = page.getByRole('button', { name: /^Links / });
  await expect(async () => {
    if (await row.isVisible()) await row.click();
    await expect(page.getByRole('button', { name: 'Add a link' })).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
};

test('the links page shows the simulated vehicle and stopping it disconnects', async ({ vehicle }) => {
  await openLinks(vehicle);
  await expect(vehicle.getByRole('button', { name: /^Links / })).toBeHidden();
  await vehicle.getByRole('button', { name: 'Stop', exact: true }).click();
  await closeSettings(vehicle);
  await expect(vehicle.getByRole('button', { name: /Not connected|How to connect/ })).toBeVisible({ timeout: 30_000 });
});

test('the links page offers every link type the browser can open', async ({ vehicle }) => {
  await openLinks(vehicle);
  await vehicle.getByRole('button', { name: 'Add a link' }).click();
  await expect(vehicle.getByRole('button', { name: /^Simulated vehicle/ })).toBeVisible();
  await expect(vehicle.getByRole('button', { name: /^Replay a flight log/ })).toBeVisible();
});

test('the battery sheet shows each battery with its voltage', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: /^Battery \d+%/ }).click();
  const sheet = vehicle.getByRole('dialog');
  await expect(sheet.getByText(/^Battery 1/)).toBeVisible();
  await expect(sheet.getByText(/\d+\.\d\d V/).first()).toBeVisible();
  await expect(sheet.getByText('Total draw')).toBeVisible();
  await expect(sheet.getByText('Battery failsafes')).toBeVisible();
});

test('an ArduPilot battery sheet names its low-battery failsafe by voltage', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await app.getByRole('button', { name: /^Battery/ }).click();
  await expect(app.getByRole('dialog').getByText(/ at \d+\.\d V/)).toBeVisible();
});

test('a MAVLink WebSocket link connects a vehicle and hears the ground station', async ({ app, request }) => {
  const before = ((await (await request.get('http://localhost:18760/stats')).json()) as { gcsHeartbeats: number }).gcsHeartbeats;
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add a link' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('ws://localhost:18760');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await expect(app.getByText('20.0 m', { exact: true })).toBeVisible();
  await expect.poll(async () => ((await (await request.get('http://localhost:18760/stats')).json()) as { gcsHeartbeats: number }).gcsHeartbeats, { timeout: 15_000 }).toBeGreaterThan(before);
});

test('a WebSocket link is saved and reconnects by itself after the page reloads', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add a link' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('ws://localhost:18760');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await app.reload();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await openMenu(app, 'Settings');
  await app.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  await app.getByRole('button', { name: /^Links/ }).first().click();
  await expect(app.getByText('ws://localhost:18760', { exact: false }).first()).toBeVisible();
});

test('a WebSocket address that is not ws:// is refused, and one that does not answer says so', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add a link' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('http://localhost:18760');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByText('A MAVLink WebSocket address starts with ws:// or wss://.')).toBeVisible();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('ws://localhost:1');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByText('No MAVLink WebSocket answered at ws://localhost:1', { exact: true })).toBeVisible({ timeout: 15_000 });
});

test('with a saved link and no aircraft, the Fly screen offers that link, not only a new one', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await expect(app.getByRole('button', { name: 'New drone? Flash an Aircast card' })).toBeVisible();
  await app.getByRole('button', { name: 'Add a link' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('ws://localhost:1');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await closeSettings(app);
  await expect(app.getByRole('button', { name: /ws:\/\/localhost:1/ })).toBeVisible({ timeout: 15_000 });
  await expect(app.getByRole('button', { name: 'Connection settings' }), 'with a link saved, the way out is its settings, not another new link').toBeVisible();
  await app.setViewportSize({ width: 390, height: 844 });
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  const sheet = app.getByRole('dialog');
  await expect(sheet.getByRole('button', { name: /ws:\/\/localhost:1/ })).toBeVisible();
  await expect(sheet.getByRole('button', { name: 'Connection settings', exact: true })).toBeVisible();
  await expect(sheet.getByText('New drone? Flash an Aircast card')).toHaveCount(0);
});

test('the USB radio form lets the browser grant a serial port', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add a link' }).click();
  await app.getByRole('button', { name: /^USB telemetry radio/ }).click();
  await expect(app.getByText('Nothing is plugged in. Connect a radio over USB and it will appear here.')).toBeVisible();
  await expect(app.getByRole('button', { name: 'Allow a USB port' })).toBeVisible();
});

test('a browser that cannot reach USB says so once, instead of claiming nothing is plugged in', async ({ page }) => {
  await page.addInitScript(() => delete (Navigator.prototype as { serial?: unknown }).serial);
  await openApp(page);
  await page.getByRole('button', { name: /How to connect/ }).click();
  await page.getByRole('button', { name: 'Add a link' }).click();
  await page.getByRole('button', { name: /^USB telemetry radio/ }).click();
  await expect(page.getByText('Use Chrome or Edge to connect a radio over USB.')).toBeVisible();
  await expect(page.getByText('Nothing is plugged in', { exact: false })).toBeHidden();
  await expect(page.getByRole('button', { name: 'Connect', exact: true })).toBeDisabled();
});

test('the RC transmitter entry hands a plugged-in EdgeTX radio to the stick input', async ({ page }) => {
  await page.addLocatorHandler(page.getByText('An EdgeTX radio is streaming its sticks on USB. Fly with it?'), () => page.getByRole('button', { name: 'Not now', exact: true }).click());
  await plugInR2d2Radio(page);
  await openApp(page);
  await page.getByRole('button', { name: /How to connect/ }).click();
  await page.getByRole('button', { name: 'Add a link' }).click();
  await page.getByRole('button', { name: /^RC transmitter \(EdgeTX script\)/ }).click();
  await page.getByRole('button', { name: RADIO_PORT }).click();
  await expect(page.getByText('Change the port under Control > Radio on USB.', { exact: false })).toBeVisible();
  await expect(page.getByRole('button', { name: RADIO_PORT })).toContainText('Sends the sticks');
});

test('a WebSocket link that keeps failing waits apart from Connected, and its address is fixed in place', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add a link' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  const address = app.getByRole('textbox', { name: 'WebSocket address' });
  await expect(address).toHaveValue('');
  await expect(address).toHaveAttribute('placeholder', 'Example: ws://192.168.1.10:5760');
  await address.fill('ws://localhost:1');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByText('Waiting for a vehicle', { exact: true })).toBeVisible({ timeout: 15_000 });
  await expect(app.getByText('Connected', { exact: true })).toHaveCount(0);
  await app.getByRole('button', { name: 'More actions for localhost:1' }).click();
  await app.getByRole('menuitem', { name: 'Edit link' }).click();
  const edited = app.getByRole('textbox', { name: 'WebSocket address' });
  await expect(edited).toHaveValue('ws://localhost:1');
  await edited.fill('ws://localhost:18760');
  await app.getByRole('button', { name: 'Save and connect' }).click();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
});

test('How to connect replaces the Fly card instead of saying the same thing twice', async ({ app }) => {
  await expect(app.getByRole('button', { name: 'Add a link' })).toHaveCount(1);
  await app.getByRole('button', { name: /How to connect/ }).click();
  const sheet = app.getByRole('dialog', { name: 'Not connected' });
  await expect(sheet.getByText('Not connected', { exact: true })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Add a link' })).toHaveCount(1);
  await expect(sheet.getByRole('button', { name: 'Add a link' })).toBeVisible();
});

test('a saved link still trying its address says a tap disconnects it, and that the address did not answer', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add a link' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await app.getByRole('textbox', { name: 'WebSocket address' }).fill('localhost:1');
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await closeSettings(app);
  const row = app.getByRole('button', { name: /ws:\/\/localhost:1/ });
  await expect(row).toContainText('Disconnect', { timeout: 15_000 });
  await expect(app.getByText("Can't reach localhost:1. Check its address and that the aircraft is on.")).toBeVisible({ timeout: 15_000 });
  await row.click();
  await expect(row).toContainText('Connect');
  await expect(row).not.toContainText('Disconnect');
});

test('a half-typed WebSocket address survives the window switching between phone and desktop width', async ({ app }) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add a link' }).click();
  await app.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  const address = app.getByRole('textbox', { name: 'WebSocket address' });
  await address.fill('ws://192.168.1.50:5760');
  await app.setViewportSize({ width: 412, height: 860 });
  await expect(address).toHaveValue('ws://192.168.1.50:5760');
  await app.setViewportSize({ width: 1280, height: 800 });
  await expect(address).toHaveValue('ws://192.168.1.50:5760');
});
