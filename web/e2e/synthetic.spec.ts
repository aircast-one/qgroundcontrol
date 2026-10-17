import { expect, flightMode, takeOff, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('with a vehicle connected the Fly screen draws the 3D view instead of failing', async ({ vehicle }) => {
  await expect(vehicle.getByText('3D view failed to start')).toBeHidden({ timeout: 30_000 });
  const canvas = vehicle.locator('canvas:not(.maplibregl-canvas)').filter({ visible: true });
  await expect.poll(async () => {
    const boxes = await canvas.evaluateAll((all) => all.map((element) => element.getBoundingClientRect().width * element.getBoundingClientRect().height));
    return Math.max(0, ...boxes);
  }, { timeout: 30_000 }).toBeGreaterThan(400 * 300);
});

test('the synthetic view stays on screen in flight with no camera, and comes back from the map', async ({ vehicle }) => {
  const map = vehicle.getByRole('button', { name: 'Map', exact: true });
  await expect(map).toBeVisible({ timeout: 30_000 });
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^(Takeoff|Hold) Flying/, { timeout: 30_000 });
  await expect(map).toBeVisible();
  await map.click();
  const back = vehicle.getByRole('button', { name: 'Video', exact: true });
  await expect(vehicle.getByText('Synthetic view', { exact: true })).toBeVisible();
  await back.click();
  await expect(map).toBeVisible();
});
