import { connectMock, expect, flightMode, openApp, takeOff, test } from './fixtures';

test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });
test.describe.configure({ timeout: 120_000 });

test('on a phone the Fly screen runs in portrait and still takes off', async ({ page }) => {
  await openApp(page);
  await connectMock(page);
  await expect(flightMode(page)).toBeVisible();
  await takeOff(page, '5 m');
  await expect(page.getByRole('button', { name: /^(Takeoff|Hold)\b.*Change flight mode$/ })).toBeVisible({ timeout: 60_000 });
});
