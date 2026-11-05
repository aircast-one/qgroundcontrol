import { connectMock, expect, openApp, test } from './fixtures';
import { plugInR2d2Radio, setRadioChannels } from './r2d2';

const RADIO_FOUND = 'An EdgeTX radio is streaming its sticks on USB. Fly with it?';

test('flipping the R2D2 drop switch up drops the payload through the app', async ({ page }) => {
  test.setTimeout(180_000);
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

  await setRadioChannels(page, { 11: 1024 });
  await expect(page.getByRole('status').filter({ hasText: 'Payload dropped' }), 'the switch up is a drop').toBeVisible({ timeout: 30_000 });
  await setRadioChannels(page, { 11: -1024 });
  await page.getByRole('button', { name: 'More', exact: true }).click();
  await page.getByRole('dialog').getByText('Payload').click();
  const sheet = page.getByRole('dialog');
  await expect(sheet.getByText(/^Dropped \d+ s ago$/)).toBeVisible({ timeout: 30_000 });
  await sheet.getByRole('button', { name: 'Grab', exact: true }).click();
  await expect(sheet.getByText(/^Grabbed \d+ s ago$/)).toBeVisible();
  await setRadioChannels(page, { 11: 1024 });
  await expect(sheet.getByText(/^Dropped \d+ s ago$/), 'the next flip up drops again').toBeVisible({ timeout: 30_000 });
  await setRadioChannels(page, { 11: -1024 });
  await expect(sheet.getByText(/^Dropped \d+ s ago$/), 'flipping it back down is not a drop').toBeVisible();
});
