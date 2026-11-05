import { connectMock, expect, openApp, test } from './fixtures';
import { plugInR2d2Radio, setRadioChannels } from './r2d2';

const RADIO_FOUND = 'An EdgeTX radio is streaming its sticks on USB. Fly with it?';

test('the R2D2 camera sliders point the gimbal and the zoom knob sets the zoom', async ({ page }) => {
  test.setTimeout(180_000);
  await page.context().route((url) => !['localhost', '127.0.0.1'].includes(url.hostname), (route) => route.abort());
  await page.addLocatorHandler(page.getByText('Configuration tasks remain before this vehicle is ready to fly.', { exact: false }), () => page.getByRole('button', { name: 'Later', exact: true }).click());
  await plugInR2d2Radio(page);
  await openApp(page);
  await expect(page.getByText(RADIO_FOUND)).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Use', exact: true }).click();

  await connectMock(page, ['Enable camera', 'Enable gimbal']);
  await expect(page.getByRole('button', { name: /^Zoom \d+%$/ })).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'All status' }).click();
  await expect(page.getByRole('button', { name: /· P: -?[\d.]+ ·/ }), 'a mock with a camera takes about half a minute to report its gimbal').toBeVisible({ timeout: 60_000 });
  await page.keyboard.press('Escape');

  await setRadioChannels(page, { 18: -512, 20: 512, 21: 1024 });
  await expect(page.getByRole('button', { name: 'Zoom 100%' }), 'the zoom knob turned all the way is full zoom').toBeVisible({ timeout: 20_000 });
  await page.getByRole('button', { name: 'All status' }).click();
  const gimbal = page.getByRole('button', { name: /· P: -?[\d.]+ ·/ });
  await expect(gimbal, 'the right slider a quarter up tilts the camera a quarter of the way from straight down to ahead').toHaveAccessibleName(/P: -67\.5/, { timeout: 20_000 });
  await expect(gimbal, 'the left slider three quarters up turns it halfway to the right yaw limit, east on a mock facing north').toHaveAccessibleName(/Az: 90\.0/);
});
