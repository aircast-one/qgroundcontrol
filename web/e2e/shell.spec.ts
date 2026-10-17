import { expect, openApp, openMenu, test } from './fixtures';

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

test('fly screen shows no vehicle until one connects', async ({ app }) => {
  await expect(app.getByRole('button', { name: /Not connected|No vehicle/ })).toBeVisible();
});

test.describe('on a phone in landscape', () => {
  test.use({ viewport: { width: 915, height: 412 } });

  ['Plan', 'Analyze'].map((item) =>
    test(`menu opens ${item} and the rail returns to Fly`, async ({ app }) => {
      await openMenu(app, item);
      await expect(app.getByRole('menu')).toBeHidden();
      await app.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true }).click();
      await expect(app.getByRole('button', { name: 'Menu' })).toBeVisible();
    }),
  );
});

test('with no aircraft and no video the Fly screen opens on the map, not an empty video pane', async ({ app }) => {
  const map = app.getByRole('region', { name: 'Map' });
  await expect.poll(async () => (await map.boundingBox())?.width ?? 0).toBeGreaterThan(1000);
  await expect(app.getByText('Video starts when your aircraft connects')).toBeHidden();
});

test('on a desktop the rail stays on Fly and the map inset is a real preview', async ({ vehicle }) => {
  const rail = vehicle.getByRole('navigation');
  await expect(rail.getByRole('button', { name: 'Fly', exact: true })).toBeVisible();
  await expect(vehicle.getByRole('button', { name: 'Menu' })).toHaveCount(0);
  await expect.poll(async () => (await vehicle.getByRole('button', { name: 'Map', exact: true }).boundingBox())?.width ?? 0).toBeGreaterThanOrEqual(320);
});

test('menu opens the settings sheet with its tabs', async ({ app }) => {
  await openMenu(app, 'Settings');
  const tabs = app.getByRole('tablist');
  await expect(tabs.getByRole('tab')).toHaveText(['Safety', 'Control', 'Camera', 'Transmission', 'General']);
  await tabs.getByRole('tab', { name: 'General' }).click();
  await expect(tabs.getByRole('tab', { name: 'General' })).toHaveAttribute('aria-selected', 'true');
  await app.getByRole('button', { name: 'Close settings' }).click();
  await expect(tabs).toBeHidden();
});
