import { expect, openApp, test } from './fixtures';
import { plugInUblox } from './ublox';

test('a u-blox base on a USB port is set up in the browser and starts its survey-in', async ({ page }) => {
  await plugInUblox(page);
  await openApp(page);
  const rtk = page.getByRole('button', { name: 'RTK', exact: true });
  await expect(rtk).toBeVisible({ timeout: 60_000 });
  await rtk.click();
  await expect(page.getByText('Survey-in Active', { exact: true })).toBeVisible({ timeout: 30_000 });
});
