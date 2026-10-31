import { expect, test as base, type Page } from '@playwright/test';
import { flyNow, openMenu, test as hosted } from './fixtures';

const DEVICE = 'http://localhost:18780';
const SILENT = 'http://localhost:18788';
const UPGRADING = 'localhost:18786';
const SECURE = 'localhost:18785';
const PLAIN_UPGRADING = 'http://aircast-device.test:18786';
const SBS1 = 30003;
const UPLOADS = 18782;

type Stats = { uploads: string[]; firmware: number[]; relays: string[]; store: string; tcpHeartbeats: number; tracks: string[]; secure: string[] };

const stats = async (): Promise<Stats> => (await fetch(`${DEVICE}/stats`)).json() as Promise<Stats>;

const test = base.extend<{ device: Page }>({
  device: async ({ page }, use) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.addLocatorHandler(page.getByRole('dialog').filter({ hasText: 'did not respond to request for parameters' }), (dialog) => dialog.getByRole('button', { name: 'OK' }).click());
    await page.goto(`${DEVICE}/`);
    await expect(page.getByRole('region', { name: 'Map' })).toBeVisible({ timeout: 60_000 });
    await expect(page.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
    await use(page);
    expect(errors, 'uncaught page errors').toEqual([]);
  },
});

const settingsPage = async (page: Page, tab: string, name: RegExp) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: tab }).click();
  const row = page.getByRole('button', { name }).first();
  await expect(async () => {
    if (await row.isVisible()) await row.click();
    await expect(page.getByRole('button', { name: 'Back', exact: true })).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
};

const typeSetting = async (page: Page, label: string, value: string) => {
  const field = page.getByRole('textbox', { name: label });
  await field.fill(value);
  await field.press('Enter');
};

test('a device page reads an ADS-B server on its network through the device', async ({ device }) => {
  await device.getByRole('button', { name: 'Settings', exact: true }).click();
  await device.getByPlaceholder('Search settings').fill('ADS-B');
  await device.getByRole('switch', { name: 'Show nearby aircraft (ADS-B)' }).click();
  await expect.poll(async () => (await stats()).relays.some((query) => query.includes(`tcp=127.0.0.1%3A${SBS1}`)), { timeout: 30_000 }).toBe(true);
  await expect(device.getByText(/ADSB Server Error/)).toHaveCount(0);
});

test('a device page uploads a PX4 log kept on the device to a server without CORS', async ({ device }) => {
  await fetch(`${DEVICE}/fly/files/Aircast/Logs/001-2026-10-18-03-00-00-000.ulg`, { method: 'PUT', body: 'ULog-e2e' });
  await settingsPage(device, 'General', /^PX4 log transfer/);
  await typeSetting(device, 'Email Address', 'pilot@example.com');
  await typeSetting(device, 'Upload URL', `http://127.0.0.1:${UPLOADS}/upload`);
  await device.getByRole('checkbox', { name: '001-2026-10-18-03-00-00-000' }).check();
  await device.getByRole('button', { name: 'Upload…' }).click();
  await device.getByRole('dialog').getByRole('button', { name: 'Ok', exact: true }).click();
  await expect.poll(async () => (await stats()).uploads.find((sent) => sent.includes('ULog-e2e')) ?? '', { timeout: 60_000 }).toContain('filename="001-2026-10-18-03-00-00-000.ulg"');
  await expect(device.getByText('Uploaded')).toBeVisible({ timeout: 30_000 });
});

test('a device page flashes the flight controller on the device', async ({ device }) => {
  await openMenu(device, 'Analyze');
  await device.getByRole('button', { name: /^Firmware/ }).click();
  await device.getByRole('radio', { name: 'Flight controller on the Aircast device' }).click();
  await device.getByRole('checkbox', { name: 'Advanced settings' }).check();
  await device.getByRole('button', { name: /^Firmware, / }).click();
  await device.getByRole('menuitem', { name: 'A firmware file' }).click();
  const chooser = device.waitForEvent('filechooser');
  await device.getByRole('button', { name: 'Choose file' }).click();
  await (await chooser).setFiles({ name: 'copter.apj', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify({ board_id: 50, image: 'eJwDAAAAAAE=', image_size: 0 })) });
  await device.getByRole('button', { name: 'Flash', exact: true }).click();
  await expect(device.getByText('Upgrade complete', { exact: true })).toBeVisible({ timeout: 60_000 });
  expect((await stats()).firmware.length).toBeGreaterThan(0);
});

test('a device page reaches a TCP vehicle on its network through the device', async ({ device }) => {
  await settingsPage(device, 'Transmission', /^Links/);
  await device.getByRole('button', { name: 'Add link…' }).click();
  await expect(device.getByRole('button', { name: /^Wi-Fi or network/ })).toBeVisible();
  await device.getByRole('button', { name: /^Network server \(TCP\)/ }).click();
  await device.getByRole('textbox', { name: 'Host address' }).fill('127.0.0.1');
  await device.getByRole('textbox', { name: 'Port' }).fill('18784');
  await device.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect.poll(async () => (await stats()).relays.some((query) => query.includes('tcp=127.0.0.1%3A18784')), { timeout: 30_000 }).toBe(true);
  await expect.poll(async () => (await stats()).tcpHeartbeats, { timeout: 30_000 }).toBeGreaterThan(0);
});

test('a device page sends a UDP link to the server still typed on its form', async ({ device }) => {
  await settingsPage(device, 'Transmission', /^Links/);
  await device.getByRole('button', { name: 'Add link…' }).click();
  await device.getByRole('button', { name: /^Wi-Fi or network/ }).click();
  await device.getByRole('textbox', { name: 'Listening port' }).fill('18791');
  await device.getByRole('textbox', { name: 'Server address' }).fill('127.0.0.1:18783');
  await device.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect.poll(async () => (await stats()).relays.some((query) => query.includes('udp=18791&to=127.0.0.1%3A18783')), { timeout: 30_000 }).toBe(true);
});

test('a device page draws what the device detects on its camera', async ({ device }) => {
  await device.evaluate(async ([module, port]) => {
    const { Qgc } = (await import(module)) as { Qgc: { set: (path: string, value: unknown) => Promise<unknown> } };
    await Qgc.set('settings.appSettings.detectionsHttpPort', port);
  }, ['/src/bridge/Qgc.ts', 18780] as const);
  await expect(device.locator('svg rect')).toBeVisible({ timeout: 60_000 });
});

test('a device page tracks what the pilot boxes with the device tracker', async ({ device }) => {
  await device.evaluate(async ([module, port]) => {
    const { Qgc } = (await import(module)) as { Qgc: { set: (path: string, value: unknown) => Promise<unknown> } };
    await Qgc.set('settings.appSettings.detectionsHttpPort', port);
  }, ['/src/bridge/Qgc.ts', 18780] as const);
  const detected = device.locator('svg rect');
  await expect(detected).toBeVisible({ timeout: 60_000 });
  await device.evaluate(async (module) => {
    const { Qgc } = (await import(module)) as { Qgc: { set: (path: string, value: unknown) => Promise<unknown> } };
    await Qgc.set('vehicle.cameraManager.currentCameraInstance.trackingEnabled', true);
  }, '/src/bridge/Qgc.ts');
  await flyNow(device);
  const at = await detected.boundingBox();
  if (at === null) throw new Error('the detection is not on screen');
  await device.mouse.move(at.x + 2, at.y + 20);
  await device.mouse.down();
  await device.mouse.move(at.x + 30, at.y + 46, { steps: 5 });
  await device.mouse.up();
  await expect.poll(async () => (await stats()).tracks.find((sent) => sent.startsWith('PUT /api/streams/e2e/track/visual')) ?? '', { timeout: 30_000 }).toMatch(/"w":0\.\d+/);
  await expect(device.getByText('car 90%')).toBeVisible();
});

test('the Device tab holds the device pages, drawn natively, and device trouble shows in the fly view', async ({ device }) => {
  await device.getByRole('button', { name: 'e2e-device: No camera yet' }).click();
  await expect(device.getByRole('tablist').getByRole('tab', { name: 'Device' })).toHaveAttribute('aria-selected', 'true');
  await expect(device.getByText('Before you fly')).toBeVisible();
  await device.getByRole('button', { name: /^Connection/ }).click();
  await expect(device.getByText('Connection', { exact: true }).first()).toBeVisible();
  await expect(device.locator('iframe')).toHaveCount(0);
  await device.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(device.getByText('Before you fly')).toBeVisible();
  await device.getByRole('button', { name: 'Camera not set up' }).click();
  await expect(device.getByText('Cameras', { exact: true }).first()).toBeVisible();
  await expect(device.locator('iframe')).toHaveCount(0);
  await device.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(device.getByText('Before you fly')).toBeVisible();
});

test('on a desktop the device home is a dashboard, with device health beside the checks', async ({ device }) => {
  await device.getByRole('button', { name: 'e2e-device: No camera yet' }).click();
  const checks = device.getByRole('region', { name: 'Before you fly' });
  const health = device.getByRole('region', { name: 'Device health' });
  await expect(health).toBeVisible();
  const [left, right] = await Promise.all([checks.boundingBox(), health.boundingBox()]);
  expect(left).not.toBeNull();
  expect(right).not.toBeNull();
  expect(right!.x).toBeGreaterThanOrEqual(left!.x + left!.width);
  expect(right!.y).toBeLessThan(left!.y + left!.height);
});

test('on a desktop the flight log lists the runs beside the chosen run, with its files at hand', async ({ device }) => {
  await device.getByRole('button', { name: 'e2e-device: No camera yet' }).click();
  await device.getByRole('button', { name: /^Flight log/ }).click();
  const runs = device.getByRole('list', { name: 'Runs' });
  const chosen = device.getByRole('region', { name: 'Thu 29 Oct, 20:56' });
  await expect(chosen.getByText('siyi stopped sending video')).toBeVisible();
  await expect(chosen.getByRole('link', { name: 'CSV' })).toBeVisible();
  const [list, detail] = await Promise.all([runs.boundingBox(), chosen.boundingBox()]);
  expect(detail!.x).toBeGreaterThanOrEqual(list!.x + list!.width);
  await runs.getByRole('button', { name: /^Thu 29 Oct, 20:50/ }).click();
  await expect(device.getByRole('region', { name: 'Thu 29 Oct, 20:50' }).getByText('Nothing worth recording')).toBeVisible();
  await expect(runs.getByRole('button', { name: /^Thu 29 Oct, 20:50/ })).toHaveAttribute('aria-current', 'true');
});

test('a device page offers its synthetic view on every visit, even after it was removed', async ({ device }) => {
  const offered = async () => {
    await device.getByRole('button', { name: /^Cameras, showing/ }).click();
    await expect(device.getByRole('menuitem', { name: /Synthetic view/ })).toBeVisible();
    await device.keyboard.press('Escape');
  };
  await device.addLocatorHandler(device.getByRole('dialog').filter({ hasText: /transfer failed/ }), (dialog) => dialog.getByRole('button', { name: 'OK' }).click());
  await offered();
  const sources = device.getByRole('button', { name: /^Video sources/ });
  const synthetic = device.getByRole('button', { name: /Synthetic view/ });
  await device.getByRole('button', { name: 'Settings', exact: true }).click();
  await device.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  await expect(async () => {
    if (await sources.isVisible()) await sources.click();
    if (await synthetic.isVisible()) await synthetic.click();
    const remove = device.getByRole('button', { name: 'Remove' });
    if (await remove.isVisible()) await remove.click();
    await expect(device.getByRole('button', { name: 'Add a video source' })).toBeVisible({ timeout: 2_000 });
    await expect(synthetic).toHaveCount(0);
  }).toPass({ timeout: 45_000 });
  await device.getByRole('button', { name: 'Close settings' }).click();
  await device.reload();
  await offered();
});

hosted('an app served elsewhere connects to a device by its address and remembers it until let go', async ({ app }) => {
  await openMenu(app, 'Settings');
  await app.getByRole('tablist').getByRole('tab', { name: 'Device' }).click();
  const address = app.getByRole('textbox', { name: 'Device address' });
  const connect = app.getByRole('button', { name: 'Connect', exact: true });
  await address.fill('localhost:1');
  await connect.click();
  await expect(app.getByText(/^localhost:1 did not answer as an Aircast device/)).toBeVisible({ timeout: 30_000 });
  await address.fill(new URL(DEVICE).host);
  await connect.click();
  await expect(app.getByText('Before you fly')).toBeVisible({ timeout: 30_000 });
  await expect(app.getByText(new URL(DEVICE).host, { exact: true })).toBeVisible();
  await app.reload();
  await app.getByRole('button', { name: 'e2e-device: No camera yet' }).click();
  await expect(app.getByText('Before you fly')).toBeVisible();
  await Promise.all([app.waitForEvent('load'), app.getByRole('button', { name: 'Disconnect' }).click()]);
  await openMenu(app, 'Settings');
  await app.getByRole('tablist').getByRole('tab', { name: 'Device' }).click();
  await expect(app.getByRole('textbox', { name: 'Device address' })).toBeVisible();
});

hosted('an app served elsewhere moves to the https address the device names, so nothing it loads is insecure', async ({ app }) => {
  await openMenu(app, 'Settings');
  await app.getByRole('tablist').getByRole('tab', { name: 'Device' }).click();
  await app.getByRole('textbox', { name: 'Device address' }).fill(UPGRADING);
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByText('Before you fly')).toBeVisible({ timeout: 30_000 });
  await expect(app.getByText(SECURE, { exact: true })).toBeVisible();
  await expect.poll(async () => (await stats()).secure, { timeout: 30_000 }).toContain('/api/views/aircraft/events');
  await Promise.all([app.waitForEvent('load'), app.getByRole('button', { name: 'Disconnect' }).click()]);
});

test('a pilot looking for the RTSP and telemetry links finds them by search and from the Transmission tab', async ({ device }) => {
  await device.getByRole('button', { name: 'Settings', exact: true }).click();
  await device.getByRole('textbox', { name: 'Search settings' }).fill('rtsp');
  await device.getByRole('button', { name: /^Ground station links/ }).click();
  await expect(device.getByText('rtsp://e2e-device:8554/e2e')).toBeVisible();
  await expect(device.getByText('e2e-device:14550')).toBeVisible();
  await device.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  await device.getByRole('button', { name: /^Ground station links.*RTSP video, MAVLink telemetry/ }).click();
  await expect(device.getByText('rtsp://e2e-device:8554/e2e')).toBeVisible();
});

base('a page the device served over http flies from there, even when the device names an https address', async ({ page }) => {
  await page.goto(`http://${UPGRADING}/`);
  await expect(page.getByRole('region', { name: 'Map' })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  expect(new URL(page.url()).origin).toBe(`http://${UPGRADING}`);
});

base('a page the device served over plain http, where Chrome hides USB, opens on the https name the device holds a certificate for when asked', async ({ page }) => {
  await page.goto(`${PLAIN_UPGRADING}/`);
  await expect(page.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'Control' }).click();
  await expect(page.getByRole('button', { name: 'Allow a USB port' })).toHaveCount(0);
  await Promise.all([page.waitForURL(`https://${SECURE}/**`), page.getByRole('button', { name: 'Open over https' }).click()]);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'Control' }).click();
  await expect(page.getByRole('button', { name: 'Allow a USB port' })).toBeVisible();
});

base('a page the device served stays on http when its https name never answers, as it must once the cloud is gone', async ({ page }) => {
  const started = Date.now();
  await page.goto('http://localhost:18787/');
  await expect(page.getByRole('region', { name: 'Map' })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  expect(new URL(page.url()).origin).toBe('http://localhost:18787');
  expect(Date.now() - started).toBeLessThan(30_000);
});

base('a device that hears no flight controller says so and opens its telemetry page', async ({ page }) => {
  await page.goto(`${SILENT}/`);
  const status = page.getByRole('button', { name: /^No flight controller/ });
  await expect(status).toBeVisible({ timeout: 60_000 });
  await status.click();
  const sheet = page.getByRole('dialog');
  await expect(sheet.getByText('Aircast e2e-device hears no flight controller. Check it is plugged in and powered on.')).toBeVisible();
  await expect(sheet.getByText('New drone? Flash an Aircast card')).toHaveCount(0);
  await sheet.getByRole('button', { name: 'Check device telemetry' }).click();
  const fc = page.getByRole('region', { name: 'Status' });
  await expect(fc.getByText('No flight controller is attached. Plug the controller in and power it on.')).toBeVisible();
  await expect(page.getByRole('region', { name: 'Connect another ground station' }).getByText('e2e-device:14550')).toBeVisible();
});

base('on a phone the device page offers to check the device, not to add a link', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(`${SILENT}/`);
  await expect(page.getByRole('button', { name: 'Check device' })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole('button', { name: 'Add a link' })).toHaveCount(0);
});
