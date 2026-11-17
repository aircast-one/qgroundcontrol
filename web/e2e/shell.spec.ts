import { APP, closeSettings, expect, openAircraftSetup, openApp, openMenu, takeOff, test } from './fixtures';

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
  await expect(app.getByRole('button', { name: 'Add a camera to see video' })).toBeHidden();
});

test('on a desktop Fly keeps the same rail as every other screen, and the map inset starts as a button and shows as a real preview', async ({ vehicle }) => {
  const rail = vehicle.getByRole('navigation');
  await expect(rail.getByRole('button')).toHaveText(['Fly', 'Plan', 'Analyze', 'Aircraft setup', 'Settings']);
  await expect(rail.getByRole('button', { name: 'Fly', exact: true })).toHaveAttribute('aria-current', 'page');
  await expect(vehicle.getByRole('button', { name: 'Menu' })).toBeVisible();
  const inset = async () => (await vehicle.getByRole('button', { name: 'Map', exact: true }).boundingBox())?.width ?? 0;
  await expect.poll(inset).toBe(48);
  const left = async (name: string) => (await vehicle.getByRole('button', { name, exact: true }).boundingBox())?.x ?? -1;
  const actions = await vehicle.getByRole('group', { name: 'Flight actions' }).getByRole('button').first().boundingBox();
  expect(await left('Map'), 'the map button lines up with the action rail').toBe(actions?.x ?? -2);
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Edit layout').click();
  await vehicle.getByRole('button', { name: 'Show', exact: true }).click();
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect.poll(inset).toBeGreaterThanOrEqual(320);
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
  await closeSettings(app);
  await expect(tabs).toBeHidden();
});

test('Escape never changes screens, and the browser Back button walks back through the screens without leaving the app', async ({ app }) => {
  await openMenu(app, 'Plan');
  const plan = app.getByText('Click the map to add a waypoint');
  await expect(plan).toBeVisible();
  await app.keyboard.press('Escape');
  await expect(plan).toBeVisible();
  await openMenu(app, 'Settings');
  const tabs = app.getByRole('tablist');
  await expect(tabs).toBeVisible();
  await app.keyboard.press('Escape');
  await expect(tabs).toBeVisible();
  await app.goBack();
  await expect(tabs).toBeHidden();
  await expect(plan).toBeVisible();
  await app.goBack();
  await expect(plan).toBeHidden();
  await expect(app.getByRole('region', { name: 'Map' })).toBeVisible();
  expect(new URL(app.url()).pathname).toBe('/');
});

test('Back inside Aircraft setup closes the page it is on and stays in the app', async ({ vehicle }) => {
  await openAircraftSetup(vehicle);
  const setup = vehicle.getByRole('navigation').getByRole('button', { name: 'Aircraft setup', exact: true });
  await expect(setup).toHaveAttribute('aria-current', 'page');
  await vehicle.getByRole('button', { name: /^Power/ }).click();
  await expect(vehicle.getByText('Power', { exact: true }).last()).toBeVisible();
  await vehicle.goBack();
  await expect(setup).toHaveAttribute('aria-current', 'page');
  await expect(vehicle.getByRole('button', { name: /^Sensors/ })).toBeVisible();
  expect(new URL(vehicle.url()).pathname).toBe('/');
});

test('Settings keeps the keyboard inside it and hands focus back to the control that opened it', async ({ vehicle }) => {
  const rail = vehicle.getByRole('navigation').getByRole('button', { name: 'Settings', exact: true });
  await rail.focus();
  await vehicle.keyboard.press('Enter');
  await vehicle.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  await vehicle.getByRole('button', { name: /^NTRIP/ }).focus();
  await vehicle.keyboard.press('Tab');
  await vehicle.keyboard.press('Tab');
  await expect(vehicle.getByRole('button', { name: /Change flight mode/ }).first()).not.toBeFocused();
  await expect(vehicle.getByRole('button', { name: 'Menu', exact: true })).not.toBeFocused();
  await vehicle.getByRole('button', { name: /^NTRIP/ }).focus();
  await vehicle.goBack();
  await expect(vehicle.getByRole('tablist')).toBeHidden();
  await expect(rail).toBeFocused();
});

test('while armed every other screen keeps the flight in view and leads back to Fly', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(vehicle.getByRole('button', { name: /^Armed$|Flying/ }).first()).toBeVisible({ timeout: 60_000 });
  const status = vehicle.getByRole('status', { name: 'Flight status' });
  await expect(status).toBeHidden();
  await openMenu(vehicle, 'Settings');
  await expect(status).toBeVisible();
  await expect(status.getByLabel(/^Alt /)).toBeVisible();
  await expect(status.getByLabel(/^Battery /)).toBeVisible();
  await expect(status.getByLabel('Link OK')).toBeVisible();
  await openMenu(vehicle, 'Plan');
  await expect(status).toBeVisible();
  await status.getByRole('button', { name: 'Back to Fly' }).click();
  await expect(status).toBeHidden();
  await expect(vehicle.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true })).toHaveAttribute('aria-current', 'page');
});

test('in the browser Alt with a digit or comma moves between screens', async ({ app }) => {
  const rail = app.getByRole('navigation');
  await app.keyboard.press('Alt+Digit2');
  await expect(rail.getByRole('button', { name: 'Plan', exact: true })).toHaveAttribute('aria-current', 'page');
  await app.keyboard.press('Alt+Comma');
  await expect(rail.getByRole('button', { name: 'Settings', exact: true })).toHaveAttribute('aria-current', 'page');
  await app.keyboard.press('Alt+Digit4');
  await expect(rail.getByRole('button', { name: 'Aircraft setup', exact: true })).toHaveAttribute('aria-current', 'page');
  await app.keyboard.press('Alt+Digit1');
  await expect(rail.getByRole('button', { name: 'Fly', exact: true })).toHaveAttribute('aria-current', 'page');
});
