import { APP, expect, openApp, openMenu, test } from './fixtures';

test('first run sets units once and stays dismissed', async ({ page }) => {
  await openApp(page, { firstRun: false });
  const welcome = page.getByRole('dialog').filter({ has: page.getByRole('heading', { name: 'Welcome to Aircast' }) });
  await expect(welcome.getByRole('radio', { name: 'Feet' }).first()).toBeVisible();
  await welcome.getByRole('button', { name: 'Imperial System' }).click();
  const restart = page.getByRole('dialog').filter({ hasText: 'Restart application for changes to take effect.' });
  await restart.getByRole('button', { name: 'OK' }).click();
  await expect(welcome.getByRole('button', { name: 'Imperial System' })).toHaveAttribute('aria-pressed', 'true');
  await expect(welcome.getByRole('radio', { name: 'Feet' }).first()).toBeChecked();
  await welcome.getByRole('button', { name: 'Continue' }).click();
  await expect(page.getByRole('dialog')).toBeHidden();
  await page.reload();
  await expect(page.getByRole('region', { name: 'Map' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Welcome to Aircast' })).toBeHidden();
});

test('the page says it is loading until the flight core arrives, then opens the app', async ({ page }) => {
  const core = Promise.withResolvers<void>();
  await page.route('**/groundstation_bg*.wasm', async (route) => {
    await core.promise;
    await route.continue();
  });
  await page.goto(APP);
  await expect(page.getByRole('progressbar', { name: 'Loading Aircast' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Menu' })).toHaveCount(0);
  core.resolve();
  await expect(page.getByRole('heading', { name: 'Welcome to Aircast' })).toBeVisible();
  await expect(page.getByRole('progressbar', { name: 'Loading Aircast' })).toHaveCount(0);
});

test('the loading screen counts the core download', async ({ page }) => {
  const network = await page.context().newCDPSession(page);
  await network.send('Network.enable');
  await network.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: 4_000_000, uploadThroughput: -1 });
  await page.goto(APP);
  await expect(page.getByText(/^Loading Aircast… \d+%$/)).toBeVisible({ timeout: 30_000 });
  await network.send('Network.emulateNetworkConditions', { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
  await expect(page.getByRole('heading', { name: 'Welcome to Aircast' })).toBeVisible();
});

test('fly screen shows no vehicle until one connects', async ({ app }) => {
  await expect(app.getByRole('button', { name: /Not connected|No vehicle/ })).toBeVisible();
});

['Plan', 'Analyze'].map((item) =>
  test(`menu opens ${item} and the rail returns to Fly`, async ({ app }) => {
    await openMenu(app, item);
    await expect(app.getByRole('menu')).toBeHidden();
    await app.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true }).click();
    await expect(app.getByRole('button', { name: 'Menu' })).toBeVisible();
  }),
);

test('with no aircraft and no video the Fly screen opens on the map, not an empty video pane', async ({ app }) => {
  const map = app.getByRole('region', { name: 'Map' });
  await expect.poll(async () => (await map.boundingBox())?.width ?? 0).toBeGreaterThan(1000);
  await expect(app.getByText('Video starts when your aircraft connects')).toBeHidden();
});

test('on a desktop Fly stays full screen with no rail, and the map inset is a real preview', async ({ vehicle }) => {
  await expect(vehicle.getByRole('navigation')).toHaveCount(0);
  await expect(vehicle.getByRole('button', { name: 'Menu' })).toBeVisible();
  await expect.poll(async () => (await vehicle.getByRole('button', { name: 'Map', exact: true }).boundingBox())?.width ?? 0).toBeGreaterThanOrEqual(320);
});

test('the menu takes the app full screen and back, for a browser whose F11 does nothing', async ({ app }) => {
  const fullScreen = () => app.evaluate(() => document.fullscreenElement !== null);
  await openMenu(app, 'Full screen');
  await expect.poll(fullScreen).toBe(true);
  await openMenu(app, 'Exit full screen');
  await expect.poll(fullScreen).toBe(false);
});

test('menu opens the settings sheet with its tabs', async ({ app }) => {
  await openMenu(app, 'Settings');
  const tabs = app.getByRole('tablist');
  await expect(tabs.getByRole('tab')).toHaveText(['General', 'Safety', 'Control', 'Camera', 'Transmission', 'Device']);
  await expect(tabs.getByRole('tab', { name: 'General' })).toHaveAttribute('aria-selected', 'true');
  await app.getByRole('button', { name: 'Close settings' }).click();
  await expect(tabs).toBeHidden();
});
