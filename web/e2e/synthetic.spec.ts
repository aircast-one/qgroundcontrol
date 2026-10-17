import { expect, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('with a vehicle connected the Fly screen draws the 3D view instead of failing', async ({ vehicle }) => {
  await expect(vehicle.getByText('3D view failed to start')).toBeHidden({ timeout: 30_000 });
  const canvas = vehicle.locator('canvas:not(.maplibregl-canvas)').filter({ visible: true });
  await expect.poll(async () => {
    const boxes = await canvas.evaluateAll((all) => all.map((element) => element.getBoundingClientRect().width * element.getBoundingClientRect().height));
    return Math.max(0, ...boxes);
  }, { timeout: 30_000 }).toBeGreaterThan(400 * 300);
});
