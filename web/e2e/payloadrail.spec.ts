import type { Page } from '@playwright/test';
import { connectMock, expect, openApp, test } from './fixtures';
import { lastAnswer, plugInR2d2Radio, readFlightModesOn, setRadioChannels } from './r2d2';

const RADIO_FOUND = 'An EdgeTX radio is streaming its sticks on USB. Fly with it?';
const SIX_POS = [-1024, -614, -205, 205, 614, 1024] as const;
const R2D2_SIX_POS = [-179, -154, -127, -100, -73, -46] as const;
const SAFE = 'The payload is safe. Arm it first.';

const fourServoBays = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByPlaceholder('Search settings').fill('bay');
  const bays = page.getByRole('textbox', { name: /^How many payload bays/ });
  await bays.fill('4');
  await bays.press('Enter');
  await page.getByRole('radio', { name: 'A servo per bay' }).click();
  await expect(page.getByText('Bay 4 output', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close settings' }).click();
};

const rail = (page: Page) => page.getByRole('group', { name: 'Payload' });

test('the payload rail arms, picks a bay and drops it, and refuses a drop while safe', async ({ page }) => {
  test.setTimeout(180_000);
  await page.context().route((url) => !['localhost', '127.0.0.1'].includes(url.hostname), (route) => route.abort());
  await page.addLocatorHandler(page.getByText('Configuration tasks remain before this vehicle is ready to fly.', { exact: false }), () => page.getByRole('button', { name: 'Later', exact: true }).click());
  await openApp(page);
  await connectMock(page, [], 'APM ArduCopter Vehicle');
  await fourServoBays(page);

  await expect(rail(page).getByRole('button', { name: 'Payload safe, tap to arm' })).toBeVisible({ timeout: 30_000 });
  await expect(rail(page).getByRole('button', { name: /^Bay \d/ })).toHaveCount(4);
  await expect(rail(page).getByRole('button', { name: 'Bay 1, loaded, selected' })).toBeVisible();
  await rail(page).getByRole('button', { name: 'Drop 1' }).click();
  await expect(rail(page).getByText(SAFE)).toBeVisible();

  await rail(page).getByRole('button', { name: 'Payload safe, tap to arm' }).click();
  await expect(rail(page).getByRole('button', { name: 'Payload armed, tap to make safe' })).toBeVisible();
  await rail(page).getByRole('button', { name: 'Bay 2, loaded' }).click();
  await expect(rail(page).getByRole('button', { name: 'Bay 2, loaded, selected' })).toBeVisible();
  await rail(page).getByRole('button', { name: 'Drop 2' }).click();
  await expect(rail(page).getByRole('button', { name: 'Bay 2, empty, selected' }), 'the dropped bay reads empty').toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'More', exact: true }).click();
  await page.getByRole('dialog').getByText('Payload').click();
  await expect(page.getByRole('dialog').getByText(/^Bay 2 dropped \d+ s ago$/)).toBeVisible({ timeout: 30_000 });
});

test("the lock and the radio's BOMARM switch both arm, the last one moved wins, and the 6POS switch picks the bay", async ({ page }) => {
  test.setTimeout(180_000);
  await page.context().route((url) => !['localhost', '127.0.0.1'].includes(url.hostname), (route) => route.abort());
  await page.addLocatorHandler(page.getByText('Configuration tasks remain before this vehicle is ready to fly.', { exact: false }), () => page.getByRole('button', { name: 'Later', exact: true }).click());
  await plugInR2d2Radio(page);
  await openApp(page);
  await expect(page.getByText(RADIO_FOUND)).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Use', exact: true }).click();
  await connectMock(page, [], 'APM ArduCopter Vehicle');
  await fourServoBays(page);
  await readFlightModesOn(page, 10);
  await page.getByRole('button', { name: 'All status' }).click();
  await expect(page.getByText('Radio · live')).toBeVisible({ timeout: 20_000 });
  await page.keyboard.press('Escape');

  await rail(page).getByRole('button', { name: 'Payload safe, tap to arm' }).click({ timeout: 30_000 });
  await expect(rail(page).getByRole('button', { name: 'Payload armed, tap to make safe' }), 'a tap arms while the radio streams with its switch down').toBeVisible();
  await expect.poll(() => lastAnswer(page), { message: 'the radio is told the payload is armed', timeout: 10_000 }).toContain('|armed|');
  await setRadioChannels(page, { 12: 1024, 5: SIX_POS[2] });
  await expect(rail(page).getByRole('button', { name: 'Bay 3, loaded, selected' })).toBeVisible({ timeout: 30_000 });
  await setRadioChannels(page, { 12: -1024 });
  await expect(rail(page).getByRole('button', { name: 'Payload safe, tap to arm' }), 'flipping the switch down makes it safe').toBeVisible({ timeout: 30_000 });
  await setRadioChannels(page, { 12: 1024 });
  await expect(rail(page).getByRole('button', { name: 'Payload armed, tap to make safe' }), 'flipping it up arms').toBeVisible({ timeout: 30_000 });
  await expect(rail(page).getByRole('button', { name: 'Drop 3' })).toBeVisible();
  await setRadioChannels(page, { 5: R2D2_SIX_POS[0] });
  await expect(rail(page).getByRole('button', { name: 'Drop 1' }), "R2D2's default model sends button 1 of its 6POS as -179").toBeVisible({ timeout: 30_000 });
  await setRadioChannels(page, { 5: R2D2_SIX_POS[3] });
  await expect(rail(page).getByRole('button', { name: 'Drop 4' }), 'and button 4 as -100').toBeVisible({ timeout: 30_000 });
  await setRadioChannels(page, { 5: 0 });
  await rail(page).getByRole('button', { name: 'Bay 2, loaded' }).click();
  await expect(rail(page).getByRole('button', { name: 'Drop 2' }), 'a channel between 6POS positions leaves the pick to the app').toBeVisible({ timeout: 30_000 });
});
