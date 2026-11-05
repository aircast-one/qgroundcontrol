import type { Page } from '@playwright/test';
import { connectMock, expect, openApp, test } from './fixtures';
import { plugInR2d2Radio, setRadioChannels } from './r2d2';

const RADIO_FOUND = 'An EdgeTX radio is streaming its sticks on USB. Fly with it?';
const SIX_POS = [-1024, -614, -205, 205, 614, 1024] as const;

const flyWithRadio = async (page: Page) => {
  await page.context().route((url) => !['localhost', '127.0.0.1'].includes(url.hostname), (route) => route.abort());
  await page.addLocatorHandler(page.getByText('Configuration tasks remain before this vehicle is ready to fly.', { exact: false }), () => page.getByRole('button', { name: 'Later', exact: true }).click());
  await plugInR2d2Radio(page);
  await openApp(page);
  await expect(page.getByText(RADIO_FOUND)).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Use', exact: true }).click();
  await connectMock(page, [], 'APM ArduCopter Vehicle');
  await page.getByRole('button', { name: 'All status' }).click();
  await expect(page.getByText('Radio · live')).toBeVisible({ timeout: 20_000 });
  await page.keyboard.press('Escape');
};

const flip = async (page: Page, raw: Readonly<Record<number, number>>) => {
  await setRadioChannels(page, raw);
  await page.waitForTimeout(100);
};

const openPayload = async (page: Page) => {
  await page.getByRole('button', { name: 'More', exact: true }).click();
  await page.getByRole('dialog').getByText('Payload').click();
  return page.getByRole('dialog');
};

test('flipping the R2D2 drop switch up drops the payload through the app', async ({ page }) => {
  test.setTimeout(180_000);
  await flyWithRadio(page);

  await flip(page, { 11: 1024 });
  await expect(page.getByRole('status').filter({ hasText: 'Payload dropped' }), 'the switch up is a drop').toBeVisible({ timeout: 30_000 });
  await flip(page, { 11: -1024 });
  const sheet = await openPayload(page);
  await expect(sheet.getByText(/^Dropped \d+ s ago$/)).toBeVisible({ timeout: 30_000 });
  await sheet.getByRole('button', { name: 'Grab', exact: true }).click();
  await expect(sheet.getByText(/^Grabbed \d+ s ago$/)).toBeVisible();
  await flip(page, { 11: 1024 });
  await expect(sheet.getByText(/^Dropped \d+ s ago$/), 'the next flip up drops again').toBeVisible({ timeout: 30_000 });
  await flip(page, { 11: -1024 });
  await expect(sheet.getByText(/^Dropped \d+ s ago$/), 'flipping it back down is not a drop').toBeVisible();
});

test('the six-position switch picks which bay the drop switch drops', async ({ page }) => {
  test.setTimeout(180_000);
  await flyWithRadio(page);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByPlaceholder('Search settings').fill('bay');
  const bays = page.getByRole('textbox', { name: /^How many payload bays/ });
  await bays.fill('4');
  await bays.press('Enter');
  await page.getByRole('radio', { name: 'A servo per bay' }).click();
  await expect(page.getByText('Bay 4 output', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close settings' }).click();
  const sheet = await openPayload(page);
  await expect(sheet.getByText('Bay 2 · output 10', { exact: true })).toBeVisible();

  await flip(page, { 5: SIX_POS[2], 11: 1024 });
  await expect(sheet.getByText(/^Bay 3 dropped \d+ s ago$/), 'the switch at its third position drops bay 3').toBeVisible({ timeout: 30_000 });
  await flip(page, { 11: -1024 });
  await flip(page, { 5: SIX_POS[0], 11: 1024 });
  await expect(sheet.getByText(/^Bay 1 dropped \d+ s ago$/), 'back at the first position it drops bay 1').toBeVisible({ timeout: 30_000 });
});
