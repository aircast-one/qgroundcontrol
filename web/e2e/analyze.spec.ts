import { expect, openMenu, test } from './fixtures';

const PAGES = ['Flight logs', 'Geotag images', 'Vibration', 'MAVLink inspector', 'Console', 'Messages', 'Firmware'] as const;

PAGES.map((page) =>
  test(`Analyze › ${page} opens with the vehicle connected`, async ({ vehicle }) => {
    await openMenu(vehicle, 'Analyze');
    const row = vehicle.getByRole('button', { name: new RegExp(`^${page}`) });
    const titled = vehicle.getByText(page, { exact: true });
    const before = await titled.count();
    await row.click();
    await expect.poll(() => titled.count()).toBeGreaterThan(before);
  }),
);

test('the MAVLink inspector lists the vehicle heartbeat', async ({ vehicle }) => {
  await openMenu(vehicle, 'Analyze');
  await vehicle.getByRole('button', { name: /^MAVLink inspector/ }).click();
  await expect(vehicle.getByText(/HEARTBEAT/).first()).toBeVisible();
});
