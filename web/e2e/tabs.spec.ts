import type { Page } from '@playwright/test';
import { APP, expect, test } from './fixtures';

const VEHICLE = 'ws://localhost:18760';

const secondTab = async (app: Page): Promise<Page> => {
  const second = await app.context().newPage();
  await second.goto(APP);
  await expect(second.getByRole('region', { name: 'Map' })).toBeVisible({ timeout: 60_000 });
  return second;
};

const openGeneral = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'General' }).click();
};

const openLinks = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  const row = page.getByRole('button', { name: /^Links/ });
  await expect(async () => {
    if (await row.isVisible()) await row.click();
    await expect(page.getByRole('button', { name: 'Add link…' })).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
};

const addWebSocket = async (page: Page, url: string) => {
  await page.getByRole('button', { name: 'Add link…' }).click();
  await page.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await page.getByRole('textbox', { name: 'WebSocket address' }).fill(url);
  await page.getByRole('button', { name: 'Connect', exact: true }).click();
};

test('a second tab runs the app too, and a setting changed in one tab is not wiped by the other', async ({ app }) => {
  const second = await secondTab(app);
  await openGeneral(app);
  await app.getByRole('switch', { name: 'Mute all audio output' }).click();
  await expect(app.getByRole('switch', { name: 'Mute all audio output' })).toBeChecked();

  await openGeneral(second);
  await expect(second.getByRole('switch', { name: 'Mute all audio output' })).toBeChecked();
  await second.getByRole('radio', { name: 'Light' }).click();

  await app.bringToFront();
  await app.reload();
  await openGeneral(app);
  await expect(app.getByRole('switch', { name: 'Mute all audio output' })).toBeChecked();
  await expect(app.locator('html')).toHaveClass(/light/);
});

test('a link saved in one tab is listed in the other, and survives that tab saving its own link', async ({ app }) => {
  const second = await secondTab(app);
  await app.getByRole('button', { name: /How to connect/ }).click();
  await addWebSocket(app, VEHICLE);
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });

  await openLinks(second);
  await expect(second.getByText(VEHICLE)).toBeVisible();
  await addWebSocket(second, 'ws://localhost:18799');
  await expect(second.getByRole('button', { name: 'More actions for localhost:18799' })).toBeVisible();

  await app.bringToFront();
  await app.reload();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
});
